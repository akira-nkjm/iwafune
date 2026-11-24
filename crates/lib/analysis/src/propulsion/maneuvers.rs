//! マヌーバごとの ΔV・噴射時間・推薬・前後の軌道（`propulsion.maneuvers`）。
//!
//! 公称の値（ΔV のマージンも残る推薬も見込まない）。初期質量は乾燥質量 + 使う推薬。マージン込みの推薬は
//! `propulsion.propellant`、区分ごとの合計とマージンは `propulsion.dv_budget`。

use super::dated::{self, DatedTransferResult};
use super::flight::Flight;
use super::input::{DvCategory, PropulsionInput, TransferMethod};
pub use super::plan::ManeuverNote;
use super::plan::Orbit;
use super::sequence::{Leg, Options, Sequence, run};
use crate::spec::{Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, Verdict};
use sat_astro::ephem::CentralBody;
use sat_astro::force::thrust::ThrustSteering;
use sat_units::typed::{Angle, Fraction, Length, Mass, Power, Qty, Time, Velocity};
use serde::{Deserialize, Serialize};

/// 地球以外の中心天体（地球のときは中心天体の欄を書かない）。
fn non_earth(c: CentralBody) -> Option<CentralBody> {
    (c != CentralBody::Earth).then_some(c)
}

/// マヌーバの前後の軌道（中心天体・高度と傾斜角）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ManeuverOrbit {
    /// 中心天体（地球なら `None` で書かない。高度はこの天体の半径から）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub central: Option<CentralBody>,
    pub perigee_altitude: Qty<Length>,
    pub apogee_altitude: Qty<Length>,
    pub inclination: Qty<Angle>,
}

impl ManeuverOrbit {
    pub(super) fn of(o: &Orbit) -> Self {
        let h = |r: f64| Qty::from_raw(r - o.central.radius_m());
        Self {
            central: non_earth(o.central),
            perigee_altitude: h(o.apsides.periapsis_m),
            apogee_altitude: h(o.apsides.apoapsis_m),
            inclination: Qty::from_raw(o.inclination_rad),
        }
    }
}

/// 噴射 1 回。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ManeuverBurn {
    /// 噴射する中心天体（地球なら `None` で書かない）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub central: Option<CentralBody>,
    /// 双曲線超過速度（天体間の遷移の脱出・捕獲だけ）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub v_inf: Option<Qty<Velocity>>,
    /// 噴射する位置の高度（`central` の半径から）
    pub altitude: Qty<Length>,
    /// ΔV（有限燃焼なら損失込み）
    pub delta_v: Qty<Velocity>,
    /// 噴射時間（スラスタが無ければ `None`）
    pub burn_time: Option<Qty<Time>>,
}

/// 数値伝搬（`propagated`）のマヌーバの伝搬の結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ManeuverPropagation {
    /// 噴射の始め（最初の軌道の元期からの経過）
    pub start: Qty<Time>,
    /// 始めから目標に着くまでの時間（日照だけ・デューティで噴かない時間を含む）
    pub duration: Qty<Time>,
    /// 噴いた時間の合計
    pub firing_time: Qty<Time>,
    /// 噴いた時間 / 時間
    pub firing_fraction: Fraction,
    /// 伝搬で積んだ ΔV（`∫ F/m dt`）
    pub delta_v: Qty<Velocity>,
    /// 伝搬で使った推薬（伝搬に使った質量から）
    pub propellant: Qty<Mass>,
    /// 伝搬に使った始めの質量（公称の列の見積もり）
    pub mass_before: Qty<Mass>,
    /// 終わりの平均の高度・傾斜角・離心率
    pub final_altitude: Qty<Length>,
    pub final_inclination: Qty<Angle>,
    pub final_eccentricity: f64,
    /// 上限の時間までに目標に着いたか
    pub reached: bool,
    pub sunlit_only: bool,
    pub duty: Fraction,
    pub steering: ThrustSteering,
}

/// マヌーバ 1 つの結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ManeuverResult {
    pub id: String,
    pub category: DvCategory,
    pub method: TransferMethod,
    pub thruster: Option<String>,
    pub before: ManeuverOrbit,
    pub after: ManeuverOrbit,
    /// ΔV（有限燃焼なら損失込み）
    pub delta_v: Qty<Velocity>,
    /// 損失を除いた ΔV（インパルス・低推力では `delta_v` と同じ）
    pub ideal_delta_v: Qty<Velocity>,
    pub burns: Vec<ManeuverBurn>,
    /// 噴射時間の合計（スラスタが無ければ `None`）
    pub burn_time: Option<Qty<Time>>,
    /// 遷移にかかる時間（インパルスは噴射の間の慣性飛行、低推力は噴射時間。低推力でスラスタが無ければ `None`）
    pub transfer_time: Option<Qty<Time>>,
    /// 推薬（スラスタが無ければ `None`）
    pub propellant: Option<Qty<Mass>>,
    pub mass_before: Qty<Mass>,
    pub mass_after: Qty<Mass>,
    /// 噴射中の消費電力（スラスタの値）
    pub firing_power: Option<Qty<Power>>,
    /// 書かれているが粗い段階では効かないので無視した制約のキー
    pub ignored_constraints: Vec<String>,
    /// 数値伝搬の結果（`propagated` のときだけ）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub propagation: Option<ManeuverPropagation>,
    /// 日付つきの惑星間遷移の結果（`transfer` を書いたときだけ）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dated_transfer: Option<DatedTransferResult>,
    pub notes: Vec<ManeuverNote>,
    pub checks: Vec<Check>,
}

/// `propulsion.maneuvers` の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ManeuversOutput {
    pub maneuvers: Vec<ManeuverResult>,
    /// ΔV の合計（公称）
    pub total_delta_v: Qty<Velocity>,
    /// 推薬の合計（スラスタのあるマヌーバの公称）
    pub total_propellant: Qty<Mass>,
    pub dry_mass: Qty<Mass>,
    /// 最初のマヌーバの前の質量（乾燥質量 + 公称の推薬）
    pub initial_mass: Qty<Mass>,
    /// 最初のマヌーバの前と、最後のマヌーバのあとの軌道
    pub initial_orbit: ManeuverOrbit,
    pub final_orbit: ManeuverOrbit,
}

/// 数値伝搬の結果 → 表示用の値。
fn propagation(central: CentralBody, leg: &Leg<'_>, f: &Flight) -> ManeuverPropagation {
    let a = &f.arc;
    let end = a.samples.last();
    let t = &leg.maneuver.thrust;
    ManeuverPropagation {
        start: Qty::from_raw(f.start_s),
        duration: Qty::from_raw(a.duration_s),
        firing_time: Qty::from_raw(a.firing_time_s),
        firing_fraction: Fraction::new(if a.duration_s > 0.0 {
            a.firing_time_s / a.duration_s
        } else {
            0.0
        }),
        delta_v: Qty::from_raw(a.delta_v_m_s),
        propellant: Qty::from_raw(a.propellant_kg),
        mass_before: Qty::from_raw(f.mass_before_kg),
        final_altitude: Qty::from_raw(
            end.map_or(0.0, |s| s.mean_semi_major_axis_m) - central.radius_m(),
        ),
        final_inclination: Qty::from_raw(end.map_or(0.0, |s| s.inclination_rad)),
        final_eccentricity: end.map_or(0.0, |s| s.eccentricity),
        reached: a.reached,
        sunlit_only: t.sunlit_only(),
        duty: Fraction::new(t.duty.map_or(1.0, Fraction::value)),
        steering: t.steering,
    }
}

/// 区間 1 つ → 結果（ΔV のマージンを含まない公称の値）。
pub(super) fn result(leg: &Leg<'_>, flight: Option<&Flight>) -> ManeuverResult {
    let m = leg.maneuver;
    let checks = if leg.notes.contains(&ManeuverNote::FiniteBurnTooLong) {
        vec![Check::new(
            "finite_burn_arc",
            "Finite-burn approximation",
            Verdict::Warning,
            format!(
                "The burn of {} exceeds half an orbit, so losses cannot be estimated (set method to low_thrust)",
                m.id
            ),
        )]
    } else {
        Vec::new()
    };
    ManeuverResult {
        id: m.id.clone(),
        category: m.category,
        method: m.method,
        thruster: m.thruster.clone(),
        before: ManeuverOrbit::of(&leg.planned.from),
        after: ManeuverOrbit::of(&leg.planned.to),
        delta_v: Qty::from_raw(leg.delta_v_m_s),
        ideal_delta_v: Qty::from_raw(leg.ideal_delta_v_m_s),
        burns: leg
            .burns
            .iter()
            .map(|b| ManeuverBurn {
                central: non_earth(b.site.central),
                v_inf: b.site.v_inf_m_s.map(Qty::from_raw),
                altitude: Qty::from_raw(b.radius_m - b.site.central.radius_m()),
                delta_v: Qty::from_raw(b.delta_v_m_s),
                burn_time: b.burn_time_s.map(Qty::from_raw),
            })
            .collect(),
        burn_time: leg.burn_time_s.map(Qty::from_raw),
        transfer_time: leg.transfer_time_s.map(Qty::from_raw),
        propellant: leg.propellant_kg.map(Qty::from_raw),
        mass_before: Qty::from_raw(leg.mass_before_kg),
        mass_after: Qty::from_raw(leg.mass_after_kg),
        firing_power: leg.thruster.and_then(|t| t.firing_power),
        ignored_constraints: m.ignored_constraints.clone(),
        propagation: flight.map(|f| propagation(leg.planned.to.central, leg, f)),
        dated_transfer: dated::result(leg),
        notes: leg.notes.clone(),
        checks,
    }
}

/// 公称の列（ΔV のマージン・残る推薬なし）。
pub(super) fn nominal(input: &PropulsionInput) -> Result<Sequence<'_>, AnalysisError> {
    run(input, Options::NOMINAL)
}

/// マヌーバごとの ΔV・噴射時間・推薬・前後の軌道。
pub struct Maneuvers;

impl Analysis for Maneuvers {
    const ID: &'static str = "propulsion.maneuvers";
    const TITLE: &'static str = "マヌーバ";
    const TITLE_EN: &'static str = "Maneuvers";
    const AREA: AnalysisArea = AnalysisArea::System(System::Propulsion);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "マヌーバを順に方式（インパルス・有限燃焼・低推力・数値伝搬）で計算し、ΔV・噴射時間・推薬・前後の軌道を出す";
    const DESCRIPTION_EN: &'static str = "Computes each maneuver in order by its method (impulsive, finite burn, low thrust, propagated): delta-V, burn time, propellant and the orbits before and after";

    type Input = PropulsionInput;
    type Output = ManeuversOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let seq = nominal(input)?;
        let maneuvers: Vec<ManeuverResult> = seq
            .legs
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let mut r = result(l, seq.flights.get(i).and_then(Option::as_ref));
                r.checks.extend(dated::checks(&seq, input, i));
                r
            })
            .collect();
        Ok(ManeuversOutput {
            total_delta_v: Qty::from_raw(seq.legs.iter().map(|l| l.delta_v_m_s).sum()),
            total_propellant: Qty::from_raw(seq.used_propellant_kg()),
            dry_mass: input.dry_mass,
            initial_mass: Qty::from_raw(seq.initial_mass_kg),
            initial_orbit: ManeuverOrbit::of(&Orbit::of(&input.initial_orbit)?),
            final_orbit: ManeuverOrbit::of(&seq.final_orbit),
            maneuvers,
        })
    }
}
