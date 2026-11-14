//! 数値伝搬（`propagated`）のマヌーバを推力を入れて伝搬し（`sat_astro::orbit::propagate_powered`）、マヌーバの順の
//! 軌道の推移（[`OrbitHistory`]）を作る。マヌーバの列を 1 回歩き、時刻と軌道を順に進める。
//!
//! - 伝搬の始めの状態: 区間の軌道が J2 の平均要素なら、平均要素から長半径の短周期項を足した接触の状態
//!   （`osculating_state_from_mean`）。数値伝搬の軌道ならその伝搬の状態。
//! - 力: 最初の軌道の伝搬の方法に合わせる（二体なら点質量、J2 なら J2、数値伝搬ならその力の設定）。
//!   中心天体は今の区間の軌道のもの（地球以外では J2 は天体の自転軸まわり。平均の長半径の換算は ECI の赤道で測る近似）。
//!   天体間の遷移（`target.body`）は数値伝搬しない（`plan` が誤りにする）。そのあとの区間は目標の天体を中心にする。
//! - 質量: 公称の列（マージンなし）のマヌーバの前の質量。目標: 計画の目標の平均の長半径と傾斜角。
//! - 上限の時間: Edelbaum の見積もり（噴き続けた時間）× [`PROPAGATION_TIME_FACTOR`] をデューティで割り、
//!   日照だけなら [`SUNLIT_ONLY_TIME_FACTOR`] を掛ける（1 日より短くしない）。

use super::history::{OrbitHistory, OrbitSegment};
use super::input::{
    DEFAULT_CONTROL_STEP_S, ManeuverInput, PROPAGATION_TIME_FACTOR, PropulsionInput,
    PropulsionThruster, SUNLIT_ONLY_TIME_FACTOR, TransferMethod,
};
use super::plan::Orbit;
use crate::spec::{AnalysisError, SUNLIT_THRESHOLD};
use sat_astro::constants::SECONDS_PER_DAY;
use sat_astro::ephem::CentralBody;
use sat_astro::force::{ForceConfig, GravityModel};
use sat_astro::orbit::{
    KeplerElements, OrbitElements, OrbitSpec, OrbitState, PoweredArc, PoweredTarget, Propagator,
    ThrustArc, elements_to_state, mean_semi_major_axis_m, orbit_averaged_elements,
    osculating_elements, osculating_state_from_mean, propagate_powered,
};
use sat_units::typed::Qty;

/// 数値伝搬のマヌーバ 1 つの結果。
#[derive(Debug, Clone)]
pub(super) struct Flight {
    /// 噴射の始め（最初の軌道の元期からの経過）[s]
    pub start_s: f64,
    pub mass_before_kg: f64,
    pub arc: PoweredArc,
    /// 終わりの軌道（平均の長半径・離心率・傾斜角）
    pub end_orbit: Orbit,
    /// 終わりの要素（J2 の軌道なら周回平均）
    pub end_elements: KeplerElements,
}

/// 歩くのに要るマヌーバ 1 つの値（`sequence` の区間から作る。`sequence` に依らないよう葉の型にする）。
#[derive(Debug, Clone, Copy)]
pub(super) struct LegStep<'a> {
    pub maneuver: &'a ManeuverInput,
    pub thruster: Option<&'a PropulsionThruster>,
    /// 計画の目標の軌道
    pub to: Orbit,
    /// 理想の ΔV [m/s]（数値伝搬なら Edelbaum の見積もり）
    pub ideal_delta_v_m_s: f64,
    pub mass_before_kg: f64,
    /// 推薬を求めたときのマヌーバのあとの質量 [kg]
    pub mass_after_kg: Option<f64>,
    pub transfer_time_s: Option<f64>,
}

/// 最初の軌道の伝搬の方法に合わせた力。
fn forces_of(p: &Propagator) -> ForceConfig {
    match p {
        Propagator::Kepler => ForceConfig::gravity_only(GravityModel::PointMass),
        Propagator::J2 => ForceConfig::gravity_only(GravityModel::J2),
        Propagator::Numerical { forces } => forces.clone(),
    }
}

/// 時刻と区間を進めながらマヌーバの列を歩く。
struct Walker<'a> {
    input: &'a PropulsionInput,
    t: f64,
    segments: Vec<OrbitSegment>,
}

impl<'a> Walker<'a> {
    fn new(input: &'a PropulsionInput) -> Self {
        Self {
            input,
            t: 0.0,
            segments: OrbitHistory::constant(input.initial_orbit.clone()).segments,
        }
    }

    fn current(&self) -> Result<(&OrbitSpec, f64), AnalysisError> {
        let seg = self
            .segments
            .last()
            .ok_or_else(|| AnalysisError::InvalidInput("軌道の推移が空です".into()))?;
        Ok((&seg.orbit, self.t - seg.start.value()))
    }

    fn with_j2(&self) -> bool {
        matches!(self.input.initial_orbit.propagator, Propagator::J2)
    }

    /// 今の区間の `dt` 秒後の接触の状態。
    fn state_now(&self) -> Result<OrbitState, AnalysisError> {
        let (spec, dt) = self.current()?;
        Ok(match spec.propagator {
            Propagator::J2 => {
                osculating_state_from_mean(spec.central, &spec.mean_elements_at(dt)?)?
            }
            Propagator::Kepler => elements_to_state(&spec.mean_elements_at(dt)?, spec.central.gm()),
            Propagator::Numerical { .. } => spec.state_at(dt)?,
        })
    }

    /// 今の区間の `dt` 秒後の要素（J2 の軌道は平均要素、数値伝搬の軌道は接触要素）。
    fn elements_now(&self) -> Result<KeplerElements, AnalysisError> {
        let (spec, dt) = self.current()?;
        Ok(match spec.propagator {
            Propagator::Numerical { .. } => {
                osculating_elements(&spec.state_at(dt)?, spec.central.gm())?
            }
            _ => spec.mean_elements_at(dt)?,
        })
    }

    /// 今の区間の中心天体（天体間の遷移のあとは目標の天体）。
    fn central(&self) -> CentralBody {
        self.segments
            .last()
            .map_or(self.input.initial_orbit.central, |s| s.orbit.central)
    }

    /// 中心天体 `central` の要素 → 時刻 `t_s` を元期にした区間の軌道。
    fn spec_of(&self, central: CentralBody, el: &KeplerElements, t_s: f64) -> OrbitSpec {
        let first = &self.input.initial_orbit;
        let mut spec = OrbitSpec::new(
            first.epoch.plus_seconds(t_s),
            OrbitElements {
                semi_major_axis: Qty::from_raw(el.a_m),
                eccentricity: el.e,
                inclination: Qty::from_raw(el.i_rad),
                raan: Qty::from_raw(el.raan_rad),
                arg_perigee: Qty::from_raw(el.argp_rad),
                mean_anomaly: Qty::from_raw(el.mean_anomaly_rad),
            },
            first.propagator.clone(),
        );
        spec.central = central;
        spec
    }

    /// 接触の状態 → 時刻 `t_s` の区間の軌道（J2 の軌道なら長半径を平均に直す）。
    fn spec_from_state(&self, s: &OrbitState, t_s: f64) -> Result<OrbitSpec, AnalysisError> {
        let central = self.central();
        let mut el = osculating_elements(s, central.gm())?;
        if self.with_j2() {
            el.a_m = mean_semi_major_axis_m(central, s, true)?;
        }
        Ok(self.spec_of(central, &el, t_s))
    }

    fn push(
        &mut self,
        start_s: f64,
        leg: &LegStep<'_>,
        thrusting: bool,
        mass: Option<f64>,
        orbit: OrbitSpec,
    ) {
        self.segments.push(OrbitSegment {
            start: Qty::from_raw(start_s),
            maneuver: Some(leg.maneuver.id.clone()),
            thrusting,
            mass: mass.map(Qty::from_raw),
            orbit,
        });
    }

    /// 数値伝搬でないマヌーバ: 終わりの時刻に、今の要素の長半径・離心率・傾斜角を目標に替えた軌道に移る。
    fn step_leg(&mut self, leg: &LegStep<'_>) -> Result<(), AnalysisError> {
        let end = self.t + leg.transfer_time_s.unwrap_or(0.0);
        self.t = end;
        let mut el = self.elements_now()?;
        let to = &leg.to;
        el.a_m = to.apsides.semi_major_axis_m();
        el.e = to.eccentricity();
        el.i_rad = to.inclination_rad;
        let mass = leg.mass_after_kg;
        let spec = self.spec_of(to.central, &el, end);
        self.push(end, leg, false, mass, spec);
        Ok(())
    }

    /// 数値伝搬のマヌーバ: 伝搬の点ごとに区間を作る（終わりの区間は周回平均の要素）。
    fn step_flight(&mut self, leg: &LegStep<'_>, f: &Flight) -> Result<(), AnalysisError> {
        let n = f.arc.samples.len();
        for (k, s) in f.arc.samples.iter().enumerate().take(n.saturating_sub(1)) {
            let t = f.start_s + s.t_s;
            let spec = self.spec_from_state(&s.state, t)?;
            self.push(t, leg, k + 1 < n, Some(s.mass_kg), spec);
        }
        let end = f.start_s + f.arc.duration_s;
        let spec = self.spec_of(self.central(), &f.end_elements, end);
        self.push(end, leg, false, Some(f.arc.end_mass_kg), spec);
        self.t = end;
        Ok(())
    }

    /// 伝搬の終わりの要素（J2 の軌道なら周回平均の要素、ほかは接触要素）。
    fn end_elements(&self, s: &OrbitState, t_s: f64) -> Result<KeplerElements, AnalysisError> {
        let first = &self.input.initial_orbit;
        let central = self.central();
        Ok(if self.with_j2() {
            let forces = forces_of(&first.propagator);
            orbit_averaged_elements(central, &forces, first.epoch.plus_seconds(t_s), s)?
        } else {
            osculating_elements(s, central.gm())?
        })
    }

    /// 今の時刻・状態から、マヌーバ `leg` を推力を入れて伝搬する。
    fn fly(&self, leg: &LegStep<'_>) -> Result<Flight, AnalysisError> {
        let m = leg.maneuver;
        let thruster = leg.thruster.ok_or_else(|| {
            AnalysisError::InvalidInput(format!(
                "マヌーバ {} の数値伝搬にはスラスタが要ります",
                m.id
            ))
        })?;
        let ve = thruster.exhaust_velocity()?;
        let thrust = thruster.thrust.value();
        let duty = m.thrust.duty()?;
        let sun = if m.thrust.sunlit_only() {
            SUNLIT_ONLY_TIME_FACTOR
        } else {
            1.0
        };
        let estimate = leg.ideal_delta_v_m_s * leg.mass_before_kg / thrust;
        let arc = ThrustArc {
            thrust_n: thrust,
            exhaust_velocity_m_s: ve,
            steering: m.thrust.steering,
            min_sunlit_factor: m.thrust.sunlit_only().then_some(SUNLIT_THRESHOLD),
            duty,
            control_step_s: DEFAULT_CONTROL_STEP_S,
            max_duration_s: (estimate * PROPAGATION_TIME_FACTOR * sun / duty).max(SECONDS_PER_DAY),
        };
        let (spec, dt) = self.current()?;
        let to = &leg.to;
        let target = PoweredTarget {
            semi_major_axis_m: to.apsides.semi_major_axis_m(),
            inclination_rad: to.inclination_rad,
        };
        let out = propagate_powered(
            spec.central,
            &forces_of(&self.input.initial_orbit.propagator),
            spec.epoch.plus_seconds(dt),
            self.state_now()?,
            leg.mass_before_kg,
            &arc,
            &target,
        )?;
        let el = self.end_elements(&out.end_state, self.t + out.duration_s)?;
        Ok(Flight {
            start_s: self.t,
            mass_before_kg: leg.mass_before_kg,
            end_orbit: Orbit::from_elements(spec.central, el.a_m, el.e, el.i_rad),
            end_elements: el,
            arc: out,
        })
    }
}

/// マヌーバの列を歩き、軌道の推移と数値伝搬の結果を返す。`known` があれば数値伝搬はそれを使う（伝搬し直さない）。
pub(super) fn walk(
    input: &PropulsionInput,
    legs: &[LegStep<'_>],
    known: Option<&[Option<Flight>]>,
) -> Result<(OrbitHistory, Vec<Option<Flight>>), AnalysisError> {
    let mut w = Walker::new(input);
    let mut flights = Vec::with_capacity(legs.len());
    for (i, leg) in legs.iter().enumerate() {
        if leg.maneuver.method != TransferMethod::Propagated {
            w.step_leg(leg)?;
            flights.push(None);
            continue;
        }
        let f = match known.and_then(|k| k.get(i)).and_then(Option::as_ref) {
            Some(f) => f.clone(),
            None => w.fly(leg)?,
        };
        w.step_flight(leg, &f)?;
        flights.push(Some(f));
    }
    Ok((
        OrbitHistory {
            segments: w.segments,
        },
        flights,
    ))
}
