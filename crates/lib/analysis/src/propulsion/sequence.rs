//! 質量を減らしながらマヌーバを順に当てる。
//!
//! - 噴射ごとに推薬 `m_p = m (1 − e^(−s Δv / v_e))`（`sat_astro::rocket::propellant_for`。`s` は ΔV に掛ける倍率で、
//!   マージン込みの設計値なら `1 + マージン`）を引き、噴射時間は `m_p v_e / F`（`burn_time`）。
//! - 有限燃焼は噴射ごとに、その時点の質量で損失込みの ΔV を求める（`sat_astro::transfer::finite_burn`）。噴射の位置の
//!   平均運動は `√(μ / r³)`（その半径の円軌道）。
//! - 初期質量（ウェット）は、最後に残る質量が「乾燥質量 + 残る推薬（使う推薬 × 残る割合）」になるまで繰り返して決める。
//! - スラスタの無いマヌーバは ΔV だけを求め、質量を変えない。

use super::input::{ManeuverInput, PropulsionInput, PropulsionThruster, TransferMethod};
use super::plan::{Ideal, ManeuverNote, Orbit, Planned, plan};
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
}

impl Sequence<'_> {
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

/// マヌーバの計画を順に作る（最後の軌道も返す）。
fn plans(input: &PropulsionInput) -> Result<(Vec<Step<'_>>, Orbit), AnalysisError> {
    let central = input.initial_orbit.central;
    let mut orbit = Orbit::of(&input.initial_orbit)?;
    let mut out = Vec::with_capacity(input.maneuvers.len());
    for m in &input.maneuvers {
        let thruster = input.thruster_of(m)?;
        let p = plan(central, orbit, m)?;
        orbit = p.to;
        out.push((m, thruster, p));
    }
    Ok((out, orbit))
}

/// マヌーバの列を当てる。初期質量は乾燥質量と残る推薬に合うまで繰り返して決める。
pub(super) fn run(input: &PropulsionInput, opts: Options) -> Result<Sequence<'_>, AnalysisError> {
    let dry = input.dry_mass.value();
    ensure_positive("乾燥質量", dry)?;
    let central = input.initial_orbit.central;
    let (plans, final_orbit) = plans(input)?;
    let r = opts.residual_fraction;
    let mut m0 = dry;
    for _ in 0..MAX_ITERATIONS {
        let legs = forward(central, &plans, m0, opts.dv_scale)?;
        let end = legs.last().map_or(m0, |l| l.mass_after_kg);
        let target = (dry + r * m0) / (1.0 + r);
        if (end - target).abs() <= MASS_TOLERANCE * m0 {
            return Ok(Sequence {
                legs,
                initial_mass_kg: m0,
                final_mass_kg: end,
                dry_mass_kg: dry,
                final_orbit,
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
        "推薬の質量が収束しません（ΔV が排気速度に対して大きすぎます）".to_owned(),
    ))
}

/// 初期質量 `m0` から順に当てる。
fn forward<'a>(
    central: CentralBody,
    plans: &[Step<'a>],
    m0: f64,
    dv_scale: f64,
) -> Result<Vec<Leg<'a>>, AnalysisError> {
    let mut mass = m0;
    let mut legs = Vec::with_capacity(plans.len());
    for (m, thruster, planned) in plans {
        let leg = apply(central, m, *thruster, planned, mass, dv_scale)?;
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
    central: CentralBody,
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
            for b in &t.burns {
                let mass = leg.mass_after_kg;
                let dv = burn_dv(central, b.radius_m, b.delta_v_m_s, finite, engine, mass)
                    .unwrap_or_else(|| {
                        leg.notes.push(ManeuverNote::FiniteBurnTooLong);
                        b.delta_v_m_s
                    });
                leg.ideal_delta_v_m_s += b.delta_v_m_s;
                fire(&mut leg, engine, b.radius_m, dv, dv_scale)?;
            }
            leg.transfer_time_s = Some(impulsive_time(t.coast_s, &leg.burns, finite));
        }
        Ideal::LowThrust(dv) => {
            leg.ideal_delta_v_m_s = *dv;
            fire(
                &mut leg,
                engine,
                planned.from.apsides.semi_major_axis_m(),
                *dv,
                dv_scale,
            )?;
            leg.transfer_time_s = leg.burn_time_s;
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
    radius_m: f64,
    dv: f64,
    dv_scale: f64,
) -> Result<(), AnalysisError> {
    leg.delta_v_m_s += dv;
    let Some(e) = engine else {
        leg.burns.push(Burn {
            radius_m,
            delta_v_m_s: dv,
            burn_time_s: None,
        });
        return Ok(());
    };
    let invalid = || {
        AnalysisError::InvalidInput(format!(
            "マヌーバ {} の推薬を求められません",
            leg.maneuver.id
        ))
    };
    let mp = propellant_for(e.ve, leg.mass_after_kg, dv * dv_scale).ok_or_else(invalid)?;
    let t = burn_time(e.thrust, e.ve, mp).ok_or_else(invalid)?;
    leg.mass_after_kg -= mp;
    leg.propellant_kg = Some(leg.propellant_kg.unwrap_or(0.0) + mp);
    leg.burn_time_s = Some(leg.burn_time_s.unwrap_or(0.0) + t);
    leg.burns.push(Burn {
        radius_m,
        delta_v_m_s: dv,
        burn_time_s: Some(t),
    });
    Ok(())
}

/// インパルス・有限燃焼の遷移の時間: 噴射の間の慣性飛行。有限燃焼は噴射を近点・遠点を中心に置くので、
/// 最初と最後の噴射の半分ずつを足す（噴射が 1 回ならその噴射時間）。
fn impulsive_time(coast_s: f64, burns: &[Burn], finite: bool) -> f64 {
    if !finite {
        return coast_s;
    }
    let t = |b: Option<&Burn>| b.and_then(|b| b.burn_time_s).unwrap_or(0.0);
    match burns {
        [] => 0.0,
        [one] => t(Some(one)),
        _ => coast_s + 0.5 * (t(burns.first()) + t(burns.last())),
    }
}
