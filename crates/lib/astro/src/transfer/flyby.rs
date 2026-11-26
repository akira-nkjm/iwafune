//! 惑星のスイングバイ（重力アシスト）のパッチドコニックの幾何。日付つきの遷移（[`super::dated`]）が、途中の天体の通過ごとに
//! この関数で入りと出の超過速度から偏向角・近点の半径・近点の噴射（動力つきのフライバイ）を求める。
//!
//! 天体の影響圏は点とみなし、通過は一瞬（影響圏の中の時間は扱わない）。天体に対する入りの超過速度 `v∞_in = v_入 − v_天体` と
//! 出の超過速度 `v∞_out = v_出 − v_天体`（`v_入`・`v_出` は前後の日心の Lambert の弧の速度）のなす角が偏向角 δ。
//! 双曲線は離心率 `e = 1 + r_p v∞²/μ` で、片側の半偏向角は `asin(1/e)`（Curtis, *Orbital Mechanics for Engineering Students*,
//! 3rd ed., §8.10。Vallado, *Fundamentals of Astrodynamics and Applications*, 4th ed., §12.4）。
//!
//! - 動力なし: `|v∞_in| = |v∞_out|`。両側が同じ双曲線で、`sin(δ/2) = 1/(1 + r_p v∞²/μ)` から近点の半径 `r_p` が決まる。
//! - 動力あり: 大きさが違えば、近点の半径 `r_p` を、2 つの双曲線の半偏向角の和 `asin(1/e_in) + asin(1/e_out)` が δ に
//!   なるように決める（右辺は `r_p` について単調に減る。二分法）。近点の速さは `√(v∞² + 2μ/r_p)`
//!   （[`super::patched::hyperbolic_periapsis_speed_m_s`]）で、近点で出す噴射は 2 つの速さの差。
//! - 近点の高度の下限（`min_altitude_m`）での最大の偏向角より δ が大きければ実現できない。その場合は下限の半径で近点を通り、
//!   足りない偏向 `δ − δ_max` を出の超過速度の向きの変更（大きさ `|v∞_out|` のままの弦 `2 |v∞_out| sin(Δ/2)`）として
//!   足りない ΔV にする（簡易の見積もりで、最適化ではない）。
//!
//! 近点の高度の下限の既定（[`default_min_altitude_m`]）は天体の種類ごとにここで決める。

use super::patched::hyperbolic_periapsis_speed_m_s;
use crate::AstroError;
use crate::ephem::CentralBody;
use crate::math::{Vec3, bisect};
use crate::time::Epoch;

/// 偏向角がこれ以下（[rad]）なら曲がらない通過とみなし、近点の半径は無限大（`None`）。
const NO_TURN_RAD: f64 = 1e-9;

/// 近点の半径の探索の範囲（対数）。下は 1 mm、上は 1e18 m（どの天体の影響圏よりも十分に外）。
const LN_RADIUS_RANGE: (f64, f64) = (-6.9, 41.4);

/// 近点の半径の二分法の許容幅（対数。相対の誤差）。
const LN_RADIUS_TOL: f64 = 1e-14;

/// スイングバイが動力なしとみなせる ΔV の上限 [m/s]（表示の区別だけ。計算は ΔV をそのまま数える）。
pub const BALLISTIC_DV_M_S: f64 = 1e-3;

/// 日付つきの遷移の途中の通過 1 回（天体・日・近点の高度の下限）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlybyStop {
    /// 通過する天体（出発・到着の天体と同じ親を回る惑星）
    pub body: CentralBody,
    /// 通過の日
    pub date: Epoch,
    /// 近点の高度の下限 [m]（天体の赤道半径から。`None` なら [`default_min_altitude_m`]）
    pub min_altitude_m: Option<f64>,
}

/// スイングバイ 1 回の結果。ベクトルは ECI（J2000 平均赤道）の向き。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlybyPass {
    /// 通過した天体
    pub body: CentralBody,
    /// 通過の日
    pub epoch: Epoch,
    /// 入りの超過速度ベクトル（前の弧の到着の速度 − 天体の速度）[m/s]
    pub v_inf_in_m_s: Vec3,
    /// 出の超過速度ベクトル（次の弧の出発の速度 − 天体の速度）[m/s]
    pub v_inf_out_m_s: Vec3,
    /// 偏向角 δ（入りと出の超過速度のなす角）[rad]
    pub turn_angle_rad: f64,
    /// 偏向角から決まる近点の半径 [m]（天体の中心から。曲がらない通過は `None` = 無限大）。実現できないときも、必要な値（下限より小さい）
    pub periapsis_radius_m: Option<f64>,
    /// 近点の半径の下限 [m]（天体の赤道半径 + 高度の下限）
    pub min_radius_m: f64,
    /// 下限の半径で得られる最大の偏向角 [rad]
    pub max_turn_rad: f64,
    /// 偏向角が下限の半径で実現できるか
    pub feasible: bool,
    /// 近点の噴射の ΔV [m/s]（入りと出の近点の速さの差。実現できないときは下限の半径での値）。動力なしなら 0
    pub powered_dv_m_s: f64,
    /// 実現できないときの、足りない偏向の分の ΔV [m/s]（実現できるなら 0）
    pub shortfall_dv_m_s: f64,
}

impl FlybyPass {
    /// このスイングバイの ΔV の合計（近点の噴射 + 足りない偏向の分）[m/s]。
    pub fn total_dv_m_s(&self) -> f64 {
        self.powered_dv_m_s + self.shortfall_dv_m_s
    }

    /// 動力なし（噴射が [`BALLISTIC_DV_M_S`] 以下）で、偏向角が実現できるか。
    pub fn is_ballistic(&self) -> bool {
        self.feasible && self.total_dv_m_s() <= BALLISTIC_DV_M_S
    }
}

/// 近点の高度の下限の慣例の既定 [m]（天体の赤道半径から）。**既定の高度の下限はここだけで決める。**
///
/// 概念設計の慣例で、実際の航法の余裕ではない。固い表面がある惑星は、大気の影響と航法の誤差を避ける 300 km
/// （水星は大気がないので 200 km）。木星型の惑星（木星・土星・天王星・海王星）は、放射線帯・環・大気の
/// 深さの慣例が一つに決まらないので `None`（呼ぶ側が高度の下限を書かせる）。太陽と月は通過の相手にならない（`None`）。
pub fn default_min_altitude_m(body: CentralBody) -> Option<f64> {
    match body {
        CentralBody::Mercury => Some(200e3),
        CentralBody::Venus | CentralBody::Earth | CentralBody::Mars => Some(300e3),
        CentralBody::Sun
        | CentralBody::Moon
        | CentralBody::Jupiter
        | CentralBody::Saturn
        | CentralBody::Uranus
        | CentralBody::Neptune => None,
    }
}

/// 速さ `v_inf_m_s` の双曲線の、近点の半径 `r_p_m` での半偏向角 `asin(1/(1 + r_p v∞²/μ))` [rad]。
fn half_turn_rad(mu: f64, r_p_m: f64, v_inf_m_s: f64) -> f64 {
    (1.0 / (1.0 + r_p_m * v_inf_m_s * v_inf_m_s / mu)).asin()
}

/// 偏向角 `turn_rad` を 2 つの双曲線の半偏向角の和で作る近点の半径 [m]（二分法。見つからなければ `None`）。
fn solve_periapsis_m(mu: f64, v_in: f64, v_out: f64, turn_rad: f64) -> Option<f64> {
    let f = |ln_r: f64| {
        let r = ln_r.exp();
        half_turn_rad(mu, r, v_in) + half_turn_rad(mu, r, v_out) - turn_rad
    };
    bisect(f, LN_RADIUS_RANGE.0, LN_RADIUS_RANGE.1, LN_RADIUS_TOL, 300).map(f64::exp)
}

/// 入りと出の超過速度の大きさ `(v_in, v_out)` と近点の高度の下限が、正の有限の値か。
fn check_inputs(
    body: CentralBody,
    (v_in, v_out): (f64, f64),
    min_altitude_m: f64,
) -> Result<(), AstroError> {
    if !(v_in.is_finite() && v_out.is_finite() && v_in > 0.0 && v_out > 0.0) {
        return Err(AstroError::InvalidInput(format!(
            "The excess speeds at the {body:?} flyby must be positive finite values ({v_in} m/s in, {v_out} m/s out)"
        )));
    }
    if !(min_altitude_m.is_finite() && min_altitude_m > 0.0) {
        return Err(AstroError::InvalidInput(format!(
            "The minimum flyby altitude at {body:?} must be a positive finite value ({min_altitude_m} m)"
        )));
    }
    Ok(())
}

/// 天体 `body` の通過（日 `epoch`）の幾何と近点の噴射。`v_inf_in`・`v_inf_out` は天体に対する超過速度ベクトル、
/// `min_altitude_m` は近点の高度の下限（正の有限の値）。超過速度の大きさが 0・有限でないとき、下限が正でないときはエラー。
pub fn flyby_geometry(
    body: CentralBody,
    epoch: Epoch,
    v_inf_in: Vec3,
    v_inf_out: Vec3,
    min_altitude_m: f64,
) -> Result<FlybyPass, AstroError> {
    let (v_in, v_out) = (v_inf_in.norm(), v_inf_out.norm());
    check_inputs(body, (v_in, v_out), min_altitude_m)?;
    let mu = body.gm();
    let min_radius_m = body.radius_m() + min_altitude_m;
    let turn_rad = v_inf_in.angle_to(v_inf_out);
    let max_turn_rad =
        half_turn_rad(mu, min_radius_m, v_in) + half_turn_rad(mu, min_radius_m, v_out);
    let speed_dv = |r_p: f64| {
        (hyperbolic_periapsis_speed_m_s(mu, r_p, v_out)
            - hyperbolic_periapsis_speed_m_s(mu, r_p, v_in))
        .abs()
    };
    let base = FlybyPass {
        body,
        epoch,
        v_inf_in_m_s: v_inf_in,
        v_inf_out_m_s: v_inf_out,
        turn_angle_rad: turn_rad,
        periapsis_radius_m: None,
        min_radius_m,
        max_turn_rad,
        feasible: true,
        powered_dv_m_s: (v_out - v_in).abs(),
        shortfall_dv_m_s: 0.0,
    };
    if turn_rad <= NO_TURN_RAD {
        return Ok(base);
    }
    let periapsis = solve_periapsis_m(mu, v_in, v_out, turn_rad);
    if turn_rad <= max_turn_rad {
        let r_p = periapsis.ok_or_else(|| {
            AstroError::InvalidInput(format!(
                "Could not solve the flyby periapsis radius at {body:?} (turn {turn_rad} rad)"
            ))
        })?;
        return Ok(FlybyPass {
            periapsis_radius_m: Some(r_p),
            powered_dv_m_s: speed_dv(r_p),
            ..base
        });
    }
    // 下限の半径でも曲がりきれない: 下限で通過し、足りない偏向は出の超過速度の向きの変更（弦）で補う
    let missing = turn_rad - max_turn_rad;
    Ok(FlybyPass {
        periapsis_radius_m: periapsis,
        feasible: false,
        powered_dv_m_s: speed_dv(min_radius_m),
        shortfall_dv_m_s: 2.0 * v_out * (0.5 * missing).sin(),
        ..base
    })
}
