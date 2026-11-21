//! RW のアンロードを RCS で行うときの、1 日あたりの推薬と ΔV 換算（ΔV バジェットに渡す値）。
//!
//! - 1 日に溜まる角運動量 `H_day = |1 周の永年成分| × (1 日 / 周期)`（捨てる量の 1 日の合計。回数によらない）。
//! - 捨てる向き: 永年成分の向きは機体座標系では姿勢とともに回るので、機体の ±x・±y・±z のうち最も推薬の要る向きで捨てるとみなす（保守側）。
//! - 推薬と噴射時間は RCS でのアンロード時間と同じ計算（[`crate::aocs::unload_time::rcs_unload`]、同時噴射の仮定）。
//! - ΔV 換算 `Δv_day = v_e ln(m₀ / (m₀ − m_day))`（`sat_astro::rocket::delta_v`）。`v_e = ΣF / Σ(F / (Isp g₀))` は噴射するスラスタの実効排気速度、
//!   `m₀` は入力の機体の質量。推薬が質量より十分小さければ日数に比例するので、バジェットは日数を掛けて使える。
//!   正味の推力による並進（軌道の変化）は ΔV 換算に含めない（`per_day.net_force` が副作用）。

use crate::aocs::thrusters::Thruster;
use crate::aocs::unload_time::{UnloadAssumption, UnloadTimeRcsResult, rcs_unload};
use crate::spec::{AnalysisError, Check, Verdict, ensure_positive};
use sat_astro::constants::SECONDS_PER_DAY;
use sat_astro::math::Vec3;
use sat_astro::rocket::{delta_v, exhaust_velocity, mass_flow_kg_s};
use sat_units::typed::{AngularMomentum, Mass, Qty, Velocity};
use serde::{Deserialize, Serialize};

/// RCS でのアンロードの入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RwUnloadingRcsInput {
    /// スラスタ（空なら推薬を求められない）
    pub thrusters: Vec<Thruster>,
    /// 重心 [m]（機体座標系）
    pub center_of_mass: Vec3,
    /// 機体の質量（ΔV 換算の m₀）
    pub mass: Qty<Mass>,
}

/// 1 日に溜まる角運動量を RCS で捨てる推薬と ΔV 換算。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RwUnloadingRcs {
    /// 1 日に溜まる角運動量（1 周の永年成分 × 1 日の周回数）
    pub momentum_per_day: Qty<AngularMomentum>,
    /// 最も推薬の要る向きで 1 日分を捨てるときの結果（`time` は 1 日の噴射時間、`propellant` は 1 日の推薬）
    pub per_day: UnloadTimeRcsResult,
    /// 1 日の推薬の ΔV 換算（その向きに捨てられるスラスタが無ければ `None`）
    pub delta_v_per_day: Option<Qty<Velocity>>,
    /// 仮定（文は service が語彙で引く）
    pub assumption: UnloadAssumption,
}

/// 1 日分の推薬と ΔV 換算、と捨てられない向きがあるときの判定。`secular_per_orbit` [N·m·s]、`period_s` [s]。
pub(super) fn per_day(
    input: &RwUnloadingRcsInput,
    secular_per_orbit: f64,
    period_s: f64,
) -> Result<(RwUnloadingRcs, Option<Check>), AnalysisError> {
    ensure_positive("Spacecraft mass", input.mass.value())?;
    if input.thrusters.is_empty() {
        return Err(AnalysisError::InvalidInput(
            "RCS unloading requires thrusters".to_owned(),
        ));
    }
    let h_day = secular_per_orbit * SECONDS_PER_DAY / period_s;
    let worst = rcs_unload(h_day, None, &input.thrusters, input.center_of_mass)?.worst;
    let dv = worst.propellant.and_then(|mp| {
        let ve = mean_exhaust_velocity(&input.thrusters, &worst.thrusters)?;
        delta_v(ve, input.mass.value(), mp.value())
    });
    let check = worst.time.is_none().then(|| {
        Check::new(
            "rcs_direction",
            "Directions RCS can unload",
            Verdict::Violation,
            format!(
                "No thruster can produce torque to dump momentum along {}",
                worst.label
            ),
        )
    });
    Ok((
        RwUnloadingRcs {
            momentum_per_day: Qty::from_raw(h_day),
            per_day: worst,
            delta_v_per_day: dv.map(Qty::from_raw),
            assumption: UnloadAssumption::RcsSimultaneousFiring,
        },
        check,
    ))
}

/// 噴射するスラスタ（ID の一覧）の実効排気速度 `ΣF / Σ(F / v_e,i)` [m/s]。1 本も無ければ `None`。
fn mean_exhaust_velocity(thrusters: &[Thruster], firing: &[String]) -> Option<f64> {
    let (force, flow) =
        thrusters
            .iter()
            .filter(|t| firing.contains(&t.id))
            .fold((0.0, 0.0), |(f, m), t| {
                let thrust = t.thrust.value();
                (
                    f + thrust,
                    m + mass_flow_kg_s(thrust, exhaust_velocity(t.isp.value())),
                )
            });
    (flow > 0.0).then(|| force / flow)
}
