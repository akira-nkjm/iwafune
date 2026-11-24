//! 質量を減らしながらマヌーバを順に当てる。
//!
//! - 噴射ごとに推薬 `m_p = m (1 − e^(−s Δv / v_e))`（`sat_astro::rocket::propellant_for`。`s` は ΔV に掛ける倍率で、
//!   マージン込みの設計値なら `1 + マージン`）を引き、噴射時間は `m_p v_e / F`（`burn_time`）。
//! - 有限燃焼は噴射ごとに、その時点の質量で損失込みの ΔV を求める（`sat_astro::transfer::finite_burn`）。噴射の位置の
//!   平均運動は `√(μ / r³)`（その半径の円軌道）。
//! - 初期質量（ウェット）は、最後に残る質量が「乾燥質量 + 残る推薬（使う推薬 × 残る割合）」になるまで繰り返して決める。
//! - スラスタの無いマヌーバは ΔV だけを求め、質量を変えない。
//! - 数値伝搬（`propagated`）のマヌーバがあれば、まず Edelbaum の見積もりで公称の列を作ってマヌーバの前の質量を決め、
//!   その質量で推力を入れて伝搬し（`flight`）、伝搬の ΔV・時間・終わりの軌道で計画を置き換えて列を作り直す。
//!   推薬は伝搬の ΔV からロケット方程式で求める（質量が伝搬と同じなら伝搬の推薬と一致する）。
//! - 軌道の推移（`history`）は列を作ったあとに 1 回歩いて作る。

use super::flight::{Flight, LegStep, walk};
use super::history::OrbitHistory;
use super::input::{ManeuverInput, PropulsionInput, PropulsionThruster, TransferMethod};
use super::plan::{BurnSite, Ideal, ManeuverNote, Orbit, Planned, plan};
use crate::spec::{AnalysisError, ensure_positive};
use sat_astro::ephem::CentralBody;
use sat_astro::orbit::mean_motion_rad_s;
use sat_astro::rocket::{burn_time, propellant_for};
use sat_astro::transfer::finite_burn;

/// 初期質量を決める繰り返しの上限。
const MAX_ITERATIONS: usize = 200;

/// 初期質量の収束の判定（相対）。
const MASS_TOLERANCE: f64 = 1e-12;

/// 噴射 1 回。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Burn {
    /// 噴射する場所（中心天体と双曲線超過速度）
    pub site: BurnSite,
    /// 噴射する位置の半径 [m]（`site.central` の中心から）
    pub radius_m: f64,
    /// 損失込みの ΔV [m/s]（倍率を掛ける前）
    pub delta_v_m_s: f64,
    pub burn_time_s: Option<f64>,
}

/// マヌーバ 1 つの結果。
#[derive(Debug, Clone)]
pub(super) struct Leg<'a> {
    pub maneuver: &'a ManeuverInput,
    pub thruster: Option<&'a PropulsionThruster>,
    pub planned: Planned,
    pub burns: Vec<Burn>,
    /// 損失込みの ΔV [m/s]
    pub delta_v_m_s: f64,
    /// 損失を除いた ΔV [m/s]
    pub ideal_delta_v_m_s: f64,
    pub transfer_time_s: Option<f64>,
    pub mass_before_kg: f64,
    pub mass_after_kg: f64,
    pub propellant_kg: Option<f64>,
    pub burn_time_s: Option<f64>,
    /// 計算の注記（計画の注記に、スラスタの有無・有限燃焼の可否を足したもの）
    pub notes: Vec<ManeuverNote>,
}

/// マヌーバの列の結果。
#[derive(Debug, Clone)]
pub(super) struct Sequence<'a> {
    pub legs: Vec<Leg<'a>>,
    pub initial_mass_kg: f64,
    pub final_mass_kg: f64,
    pub dry_mass_kg: f64,
    /// 最後のマヌーバのあとの軌道（マヌーバが無ければ初期の軌道）
    pub final_orbit: Orbit,
    /// マヌーバの順の軌道の推移
    pub history: OrbitHistory,
    /// 数値伝搬の結果（`legs` と同じ並び。数値伝搬でないマヌーバは None）
    pub flights: Vec<Option<Flight>>,
}

impl<'a> Sequence<'a> {
    /// 軌道の推移を歩くための値（区間ごと）。
    fn steps(&self) -> Vec<LegStep<'a>> {
        self.legs
            .iter()
            .map(|l| LegStep {
                maneuver: l.maneuver,
                thruster: l.thruster,
                to: l.planned.to,
                ideal_delta_v_m_s: l.ideal_delta_v_m_s,
                mass_before_kg: l.mass_before_kg,
                mass_after_kg: l.propellant_kg.map(|_| l.mass_after_kg),
                transfer_time_s: l.transfer_time_s,
            })
            .collect()
    }

    /// 使う推薬 [kg]（スラスタのあるマヌーバの合計）。
    pub fn used_propellant_kg(&self) -> f64 {
        self.legs.iter().filter_map(|l| l.propellant_kg).sum()
    }

    /// 残る推薬 [kg]。
    pub fn residual_kg(&self) -> f64 {
        self.final_mass_kg - self.dry_mass_kg
    }
}

/// ΔV に掛ける倍率と残る推薬の割合。
#[derive(Debug, Clone, Copy)]
pub(super) struct Options {
    pub dv_scale: f64,
    pub residual_fraction: f64,
}

impl Options {
    /// 公称（マージン・残る推薬なし）。
    pub const NOMINAL: Self = Self {
        dv_scale: 1.0,
        residual_fraction: 0.0,
    };

    /// 設計値（入力のマージンと残る推薬の割合）。
    pub fn design(input: &PropulsionInput) -> Result<Self, AnalysisError> {
        Ok(Self {
            dv_scale: 1.0 + input.dv_margin()?,
            residual_fraction: input.residual_fraction()?,
        })
    }
}

/// マヌーバ 1 つの計画（マヌーバ・スラスタ・前後の軌道と理想の ΔV）。
type Step<'a> = (&'a ManeuverInput, Option<&'a PropulsionThruster>, Planned);

/// マヌーバの計画を順に作る（最後の軌道も返す）。数値伝搬の結果 `flights` があれば、その ΔV・時間・終わりの軌道で置き換える。
fn plans<'a>(
    input: &'a PropulsionInput,
    flights: &[Option<Flight>],
) -> Result<(Vec<Step<'a>>, Orbit), AnalysisError> {
    let mut orbit = Orbit::of(&input.initial_orbit)?;
    let mut out = Vec::with_capacity(input.maneuvers.len());
    for (i, m) in input.maneuvers.iter().enumerate() {
        let thruster = input.thruster_of(m)?;
        let mut p = plan(orbit, m)?;
        if let Some(f) = flights.get(i).and_then(Option::as_ref) {
            p.ideal = Ideal::Propagated {
                delta_v_m_s: f.arc.delta_v_m_s,
                duration_s: f.arc.duration_s,
            };
            p.to = f.end_orbit;
            if !f.arc.reached {
                p.notes.push(ManeuverNote::PropagationNotReached);
            }
        }
        orbit = p.to;
        out.push((m, thruster, p));
    }
    Ok((out, orbit))
}

/// マヌーバの列を当てる（数値伝搬のマヌーバがあれば 2 回。モジュールの説明）。
pub(super) fn run(input: &PropulsionInput, opts: Options) -> Result<Sequence<'_>, AnalysisError> {
    let propagated = input
        .maneuvers
        .iter()
        .any(|m| m.method == TransferMethod::Propagated);
    let mut seq = if propagated {
        let first = iterate(input, plans(input, &[])?, Options::NOMINAL)?;
        let (_, flights) = walk(input, &first.steps(), None)?;
        let mut seq = iterate(input, plans(input, &flights)?, opts)?;
        seq.flights = flights;
        seq
    } else {
        iterate(input, plans(input, &[])?, opts)?
    };
    let (history, _) = walk(input, &seq.steps(), Some(&seq.flights))?;
    seq.history = history;
    Ok(seq)
}

/// 計画の列を当てる。初期質量は乾燥質量と残る推薬に合うまで繰り返して決める。
fn iterate<'a>(
    input: &'a PropulsionInput,
    (plans, final_orbit): (Vec<Step<'a>>, Orbit),
    opts: Options,
) -> Result<Sequence<'a>, AnalysisError> {
    let dry = input.dry_mass.value();
    ensure_positive("Dry mass", dry)?;
    let r = opts.residual_fraction;
    let mut m0 = dry;
    for _ in 0..MAX_ITERATIONS {
        let legs = forward(&plans, m0, opts.dv_scale)?;
        let end = legs.last().map_or(m0, |l| l.mass_after_kg);
        let target = (dry + r * m0) / (1.0 + r);
        if (end - target).abs() <= MASS_TOLERANCE * m0 {
            return Ok(Sequence {
                legs,
                initial_mass_kg: m0,
                final_mass_kg: end,
                dry_mass_kg: dry,
                final_orbit,
                history: OrbitHistory::constant(input.initial_orbit.clone()),
                flights: Vec::new(),
            });
        }
        if !(end > 0.0 && end.is_finite()) {
            break;
        }
        m0 *= target / end;
        if !m0.is_finite() {
            break;
        }
    }
    Err(AnalysisError::InvalidInput(
        "The propellant mass does not converge (ΔV is too large for the exhaust velocity)"
            .to_owned(),
    ))
}

/// 初期質量 `m0` から順に当てる。
fn forward<'a>(plans: &[Step<'a>], m0: f64, dv_scale: f64) -> Result<Vec<Leg<'a>>, AnalysisError> {
    let mut mass = m0;
    let mut legs = Vec::with_capacity(plans.len());
    for (m, thruster, planned) in plans {
        let leg = apply(m, *thruster, planned, mass, dv_scale)?;
        mass = leg.mass_after_kg;
        legs.push(leg);
    }
    Ok(legs)
}

/// スラスタの値（排気速度・推力）。
#[derive(Clone, Copy)]
struct Engine {
    ve: f64,
    thrust: f64,
}

/// 1 つのマヌーバを質量 `mass` から当てる。
fn apply<'a>(
    m: &'a ManeuverInput,
    thruster: Option<&'a PropulsionThruster>,
    planned: &Planned,
    mass: f64,
    dv_scale: f64,
) -> Result<Leg<'a>, AnalysisError> {
    let engine = thruster
        .map(|t| {
            Ok::<_, AnalysisError>(Engine {
                ve: t.exhaust_velocity()?,
                thrust: t.thrust.value(),
            })
        })
        .transpose()?;
    let mut notes = planned.notes.clone();
    if engine.is_none() {
        notes.push(ManeuverNote::NoThruster);
    }
    let mut leg = Leg {
        maneuver: m,
        thruster,
        planned: planned.clone(),
        burns: Vec::new(),
        delta_v_m_s: 0.0,
        ideal_delta_v_m_s: 0.0,
        transfer_time_s: None,
        mass_before_kg: mass,
        mass_after_kg: mass,
        propellant_kg: engine.map(|_| 0.0),
        burn_time_s: engine.map(|_| 0.0),
        notes,
    };
    match &planned.ideal {
        Ideal::Impulsive(t) => {
            let finite = m.method == TransferMethod::FiniteBurn;
            for (i, b) in t.burns.iter().enumerate() {
                let mass = leg.mass_after_kg;
                let site = planned.site(i);
                let dv = burn_dv(
                    site.central,
                    b.radius_m,
                    b.delta_v_m_s,
                    finite,
                    engine,
                    mass,
                )
                .unwrap_or_else(|| {
                    leg.notes.push(ManeuverNote::FiniteBurnTooLong);
                    b.delta_v_m_s
                });
                leg.ideal_delta_v_m_s += b.delta_v_m_s;
                fire(&mut leg, engine, (site, b.radius_m), dv, dv_scale)?;
            }
            leg.transfer_time_s = Some(impulsive_time(
                t.coast_s,
                &leg.burns,
                finite,
                (planned.launcher_departure, planned.mid_burns.clone()),
            ));
        }
        Ideal::LowThrust(dv) => {
            leg.ideal_delta_v_m_s = *dv;
            let at = (planned.site(0), planned.from.apsides.semi_major_axis_m());
            fire(&mut leg, engine, at, *dv, dv_scale)?;
            leg.transfer_time_s = leg.burn_time_s;
        }
        Ideal::Propagated {
            delta_v_m_s,
            duration_s,
        } => {
            leg.ideal_delta_v_m_s = *delta_v_m_s;
            let at = (planned.site(0), planned.from.apsides.semi_major_axis_m());
            fire(&mut leg, engine, at, *delta_v_m_s, dv_scale)?;
            leg.transfer_time_s = Some(*duration_s);
        }
    }
    leg.notes.dedup();
    Ok(leg)
}

/// 噴射 1 回の損失込みの ΔV（質量 `mass` から）。有限燃焼でなければインパルスの ΔV。有限燃焼の近似が使えなければ `None`。
fn burn_dv(
    central: CentralBody,
    radius_m: f64,
    impulsive: f64,
    finite: bool,
    engine: Option<Engine>,
    mass: f64,
) -> Option<f64> {
    let Some(e) = engine.filter(|_| finite) else {
        return Some(impulsive);
    };
    let n = mean_motion_rad_s(central.gm(), radius_m);
    finite_burn(impulsive, n, e.thrust, e.ve, mass)
        .ok()
        .map(|b| b.delta_v_m_s)
}

/// 噴射 1 回を当てる（推薬を引き、噴射時間を足す）。スラスタが無ければ ΔV だけを足す。
fn fire(
    leg: &mut Leg<'_>,
    engine: Option<Engine>,
    (site, radius_m): (BurnSite, f64),
    dv: f64,
    dv_scale: f64,
) -> Result<(), AnalysisError> {
    leg.delta_v_m_s += dv;
    let Some(e) = engine else {
        leg.burns.push(Burn {
            site,
            radius_m,
            delta_v_m_s: dv,
            burn_time_s: None,
        });
        return Ok(());
    };
    let invalid = || {
        AnalysisError::InvalidInput(format!(
            "Cannot compute the propellant for maneuver {}",
            leg.maneuver.id
        ))
    };
    let mp = propellant_for(e.ve, leg.mass_after_kg, dv * dv_scale).ok_or_else(invalid)?;
    let t = burn_time(e.thrust, e.ve, mp).ok_or_else(invalid)?;
    leg.mass_after_kg -= mp;
    leg.propellant_kg = Some(leg.propellant_kg.unwrap_or(0.0) + mp);
    leg.burn_time_s = Some(leg.burn_time_s.unwrap_or(0.0) + t);
    leg.burns.push(Burn {
        site,
        radius_m,
        delta_v_m_s: dv,
        burn_time_s: Some(t),
    });
    Ok(())
}

/// インパルス・有限燃焼の遷移の時間: 噴射の間の慣性飛行。有限燃焼は噴射を近点・遠点を中心に置くので、
/// 最初と最後の噴射の半分ずつを足す（噴射が 1 回ならその噴射時間）。出発の噴射を打ち上げ機が行うとき
/// （`launcher_departure`）は、列の最初は出発の噴射でなく捕獲の噴射なので、飛行の時間に半分だけ足す。
/// 飛行の途中の噴射（スイングバイの近点の噴射。`mid` の範囲）は両端の噴射ではないので数えない。
fn impulsive_time(
    coast_s: f64,
    burns: &[Burn],
    finite: bool,
    (launcher_departure, mid): (bool, std::ops::Range<usize>),
) -> f64 {
    if !finite {
        return coast_s;
    }
    let t = |b: Option<&Burn>| b.and_then(|b| b.burn_time_s).unwrap_or(0.0);
    let ends: Vec<Burn> = burns
        .iter()
        .enumerate()
        .filter(|(i, _)| !mid.contains(i))
        .map(|(_, b)| *b)
        .collect();
    let burns = ends.as_slice();
    match burns {
        [] => 0.0,
        [last] if launcher_departure => coast_s + 0.5 * t(Some(last)),
        [one] => t(Some(one)),
        _ => coast_s + 0.5 * (t(burns.first()) + t(burns.last())),
    }
}
