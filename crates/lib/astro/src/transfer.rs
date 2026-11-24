//! 軌道遷移の ΔV。**遷移の式（ホーマン・平面変更・バイエリプティック・Edelbaum の低推力・
//! 有限燃焼の損失の近似・パッチドコニック・Lambert の問題）はここだけ**（ΔV の解析が使う。推薬との換算は [`crate::rocket`]）。
//!
//! - 軌道は近点・遠点の半径の組（[`Apsides`]）で表す。近点線は揃っているとみなし、噴射は近点か遠点で接線方向に行う。
//! - インパルスの 2 回噴射（[`apse_transfer`]）: 出発の軌道の近点（または遠点）から、目標の軌道の遠点（または近点）までの
//!   遷移楕円を通る。安い方を選ぶ。円 → 円ならホーマン遷移（Vallado, *Fundamentals of Astrodynamics and Applications*,
//!   4th ed., §6.3.1）。平面変更は半径の大きい方の噴射に合わせ、`Δv = √(v₁² + v₂² − 2 v₁ v₂ cos θ)`（同 §6.4.3）。
//! - バイエリプティック遷移（[`bi_elliptic`]）: 中間の遠点 `r_b` を通る 3 回噴射（同 §6.3.2）。平面変更は `r_b` で行う。
//! - 低推力（[`edelbaum`]）: 円軌道どうしで高度と傾斜角を同時に変える Edelbaum の式
//!   `Δv = √(v₀² + v₁² − 2 v₀ v₁ cos(π Δi / 2))`（Edelbaum, ARS Journal 31(8), 1961）。
//! - 有限燃焼（[`finite_burn`]）: 向きを固定した噴射が弧 `Φ = n t_b` を掃く間、推力の進行方向の成分だけが効くとして
//!   `Δv_有限 = Δv_インパルス · (Φ/2) / sin(Φ/2)`（Robbins, AIAA Journal 4(8), 1966。小さい弧で `1 + Φ²/24`）。
//!
//! - 中心天体を替える遷移（[`patched_conic`]）: 天体の円軌道どうしのホーマン遷移と、出発・到着の双曲線の近点の噴射
//!   （パッチドコニック。地球 → 月・地球 → 火星など。[`patched`]）。
//! - Lambert の問題（[`lambert()`]）: 2 点と飛行時間から両端の速度（普遍変数。[`lambert`] モジュール）。
//! - 日付を決めた惑星間の遷移（[`dated_transfer`]）: 出発・到着の日の天体の状態から日心の Lambert を解き、
//!   v∞・C3・出発の赤緯・両端の噴射を求める（[`dated`] モジュール）。
//! - 途中の天体のスイングバイ（[`dated_transfer_via`]・[`flyby_geometry`]）: 日付つきの遷移を、天体の組ごとの Lambert の弧（脚。[`lambert_chain`]）の
//!   つなぎにし、通過ごとに入り・出の超過速度から回頭角・近点の半径・近点の噴射を求める（[`flyby`] モジュール。動力あり・なし、近点の高度の下限）。
//! - 日付つきの遷移の巡航の軌道（[`cruise_orbit`]）: 同じ Lambert の弧（[`lambert_arc`]）を、親のまわりの二体の軌道（接触要素）にする。
//! - 日付つきの遷移の巡航の区間（[`cruise_leg`]）: 上の軌道を、出発の天体の影響圏を出てから到着の天体の影響圏に入るまでに切り出す（影響圏の中の双曲線の弧は解析しない）。
//! - 通過のある遷移の巡航の軌道（[`cruise_trajectory`]。脚ごとの区間 [`cruise_legs`] を通過の日を境に並べた弧の連鎖 `orbit::Trajectory::Arcs`）: 境で位置は連続・速度が跳ぶ。
//!   通過の天体の影響圏の中は解かない（通過は一瞬）。通過のない遷移は弧 1 本で、[`cruise_leg`] の軌道そのもの。
//! - 日付つきの遷移の巡航の多体の確認（[`check_cruise`]。通過のある遷移は脚ごとに [`check_cruise_leg`]）: Lambert の出発の状態を、太陽と惑星の点質量で数値伝搬して、
//!   到着の日のずれ・最接近・到着の超過速度の差を求める（[`cruise`] モジュール。伝搬は `crate::orbit::propagate_states`）。
//!
//! 値は SI（m・m/s・s・rad・kg・N）。重力定数 `mu` は中心天体のもの [m³/s²]。

use crate::AstroError;
use crate::rocket::{burn_time, propellant_for};
use core::f64::consts::PI;

pub mod cruise;
pub mod dated;
pub mod flyby;
pub mod lambert;
pub mod leg;
pub mod patched;
pub use cruise::{CruiseCheck, CruiseSample, check_cruise, check_cruise_leg};
pub use dated::{
    DatedTransfer, LambertArc, cruise_orbit, dated_transfer, dated_transfer_via, lambert_arc,
    lambert_chain,
};
pub use flyby::{BALLISTIC_DV_M_S, FlybyPass, FlybyStop, default_min_altitude_m, flyby_geometry};
pub use lambert::{LambertSolution, lambert, lambert_about_normal};
pub use leg::{CruiseLeg, cruise_leg, cruise_legs, cruise_trajectory};
pub use patched::{
    PatchedBurn, PatchedConic, hyperbolic_periapsis_dv_m_s, hyperbolic_periapsis_speed_m_s,
    patched_conic,
};

/// ΔV がこれ以下の噴射は数えない [m/s]（同じ軌道どうしの丸めの誤差）。
pub const NEGLIGIBLE_DV_M_S: f64 = 1e-9;

/// 軌道の近点と遠点の半径 [m]（中心天体の中心から）。円軌道は 2 つが等しい。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Apsides {
    pub periapsis_m: f64,
    pub apoapsis_m: f64,
}

impl Apsides {
    /// 2 つの半径から作る（小さい方を近点にする）。
    pub fn new(r1_m: f64, r2_m: f64) -> Self {
        // NaN を残す（min・max は NaN を捨てるので比較で並べる）
        let (periapsis_m, apoapsis_m) = if r1_m <= r2_m {
            (r1_m, r2_m)
        } else {
            (r2_m, r1_m)
        };
        Self {
            periapsis_m,
            apoapsis_m,
        }
    }

    /// 半径 `r_m` の円軌道。
    pub fn circular(r_m: f64) -> Self {
        Self::new(r_m, r_m)
    }

    /// 長半径 [m]。
    pub fn semi_major_axis_m(&self) -> f64 {
        0.5 * (self.periapsis_m + self.apoapsis_m)
    }

    fn check(&self) -> Result<(), AstroError> {
        let ok =
            self.periapsis_m.is_finite() && self.apoapsis_m.is_finite() && self.periapsis_m > 0.0;
        if ok {
            Ok(())
        } else {
            Err(AstroError::InvalidInput(format!(
                "Orbit radii must be positive finite values ({} m, {} m)",
                self.periapsis_m, self.apoapsis_m
            )))
        }
    }
}

/// 活力積分 `v = √(μ (2/r − 1/a))` [m/s]。
pub fn vis_viva_m_s(mu: f64, r_m: f64, semi_major_axis_m: f64) -> f64 {
    (mu * (2.0 / r_m - 1.0 / semi_major_axis_m)).max(0.0).sqrt()
}

/// 速さ `v1` から `v2` へ、向きを `angle_rad` 変える 1 回の噴射の ΔV [m/s]（余弦定理）。
pub fn combined_burn_m_s(v1_m_s: f64, v2_m_s: f64, angle_rad: f64) -> f64 {
    (v1_m_s * v1_m_s + v2_m_s * v2_m_s - 2.0 * v1_m_s * v2_m_s * angle_rad.cos())
        .max(0.0)
        .sqrt()
}

/// 速さ `v` のまま軌道面を `angle_rad` 変える ΔV `2 v sin(θ/2)` [m/s]。
pub fn plane_change_m_s(v_m_s: f64, angle_rad: f64) -> f64 {
    combined_burn_m_s(v_m_s, v_m_s, angle_rad)
}

/// インパルスの噴射 1 回。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpulsiveBurn {
    /// 噴射する位置の半径 [m]
    pub radius_m: f64,
    /// ΔV の大きさ [m/s]
    pub delta_v_m_s: f64,
}

/// インパルスの遷移（噴射の列と、噴射の間の慣性飛行の時間）。
#[derive(Debug, Clone, PartialEq)]
pub struct ImpulsiveTransfer {
    /// 噴射の順。ΔV が無視できる噴射は含めない
    pub burns: Vec<ImpulsiveBurn>,
    /// 最初の噴射から最後の噴射までの時間 [s]（噴射が 1 回以下なら 0）
    pub coast_s: f64,
}

impl ImpulsiveTransfer {
    /// ΔV の合計 [m/s]。
    pub fn total_m_s(&self) -> f64 {
        self.burns.iter().map(|b| b.delta_v_m_s).sum()
    }

    fn of(burns: &[ImpulsiveBurn], coast_s: f64) -> Self {
        let burns: Vec<ImpulsiveBurn> = burns
            .iter()
            .copied()
            .filter(|b| b.delta_v_m_s > NEGLIGIBLE_DV_M_S)
            .collect();
        let coast_s = if burns.len() > 1 { coast_s } else { 0.0 };
        Self { burns, coast_s }
    }
}

/// 楕円の半周の時間 `π √(a³/μ)` [s]。
fn half_period_s(mu: f64, ellipse: Apsides) -> f64 {
    let a = ellipse.semi_major_axis_m();
    PI * (a * a * a / mu).sqrt()
}

fn check_mu(mu: f64) -> Result<(), AstroError> {
    if mu.is_finite() && mu > 0.0 {
        Ok(())
    } else {
        Err(AstroError::InvalidInput(format!(
            "Gravitational parameter must be positive ({mu})"
        )))
    }
}

/// `from` の半径 `r_start` の点から、`to` の半径 `r_end` の点までの遷移楕円を通る 2 回噴射。
/// 平面変更は半径の大きい方（速さの小さい方）の噴射に合わせる。
fn two_burn(
    mu: f64,
    from: Apsides,
    to: Apsides,
    r_start: f64,
    r_end: f64,
    angle_rad: f64,
) -> ImpulsiveTransfer {
    let transfer = Apsides::new(r_start, r_end);
    let at = transfer.semi_major_axis_m();
    let (v1a, v1b) = (
        vis_viva_m_s(mu, r_start, from.semi_major_axis_m()),
        vis_viva_m_s(mu, r_start, at),
    );
    let (v2a, v2b) = (
        vis_viva_m_s(mu, r_end, at),
        vis_viva_m_s(mu, r_end, to.semi_major_axis_m()),
    );
    let (dv1, dv2) = if r_end >= r_start {
        ((v1b - v1a).abs(), combined_burn_m_s(v2a, v2b, angle_rad))
    } else {
        (combined_burn_m_s(v1a, v1b, angle_rad), (v2b - v2a).abs())
    };
    ImpulsiveTransfer::of(
        &[
            ImpulsiveBurn {
                radius_m: r_start,
                delta_v_m_s: dv1,
            },
            ImpulsiveBurn {
                radius_m: r_end,
                delta_v_m_s: dv2,
            },
        ],
        half_period_s(mu, transfer),
    )
}

/// 軌道 `from` から `to` へ、軌道面を `plane_angle_rad` 変えるインパルスの 2 回噴射（近点・遠点で接線方向）。
/// 出発の近点から目標の遠点へ行くか、出発の遠点から目標の近点へ行くかの安い方を選ぶ。円 → 円ならホーマン遷移。
pub fn apse_transfer(
    mu: f64,
    from: Apsides,
    to: Apsides,
    plane_angle_rad: f64,
) -> Result<ImpulsiveTransfer, AstroError> {
    check_mu(mu)?;
    from.check()?;
    to.check()?;
    let a = two_burn(
        mu,
        from,
        to,
        from.periapsis_m,
        to.apoapsis_m,
        plane_angle_rad,
    );
    let b = two_burn(
        mu,
        from,
        to,
        from.apoapsis_m,
        to.periapsis_m,
        plane_angle_rad,
    );
    Ok(if b.total_m_s() < a.total_m_s() { b } else { a })
}

/// 半径 `r1_m` の円軌道から `r2_m` の円軌道へのホーマン遷移（同じ軌道面）。
pub fn hohmann(mu: f64, r1_m: f64, r2_m: f64) -> Result<ImpulsiveTransfer, AstroError> {
    apse_transfer(mu, Apsides::circular(r1_m), Apsides::circular(r2_m), 0.0)
}

/// 中間の遠点 `via_m` を通るバイエリプティック遷移（3 回噴射）。1 回目は `from` の近点で遠点を `via_m` に上げ、
/// 2 回目は `via_m` で近点を `to` の近点に合わせ（平面変更もここ）、3 回目は `to` の近点で遠点を `to` に合わせる。
/// `via_m` は 2 つの軌道の遠点より小さくできない。
pub fn bi_elliptic(
    mu: f64,
    from: Apsides,
    to: Apsides,
    via_m: f64,
    plane_angle_rad: f64,
) -> Result<ImpulsiveTransfer, AstroError> {
    check_mu(mu)?;
    from.check()?;
    to.check()?;
    if !(via_m.is_finite() && via_m >= from.apoapsis_m.max(to.apoapsis_m)) {
        return Err(AstroError::InvalidInput(format!(
            "Intermediate apoapsis of the bi-elliptic transfer ({via_m} m) must be at least the departure and target apoapsis"
        )));
    }
    let (r1, r2) = (from.periapsis_m, to.periapsis_m);
    let first = Apsides::new(r1, via_m);
    let second = Apsides::new(r2, via_m);
    let burns = [
        ImpulsiveBurn {
            radius_m: r1,
            delta_v_m_s: (vis_viva_m_s(mu, r1, first.semi_major_axis_m())
                - vis_viva_m_s(mu, r1, from.semi_major_axis_m()))
            .abs(),
        },
        ImpulsiveBurn {
            radius_m: via_m,
            delta_v_m_s: combined_burn_m_s(
                vis_viva_m_s(mu, via_m, first.semi_major_axis_m()),
                vis_viva_m_s(mu, via_m, second.semi_major_axis_m()),
                plane_angle_rad,
            ),
        },
        ImpulsiveBurn {
            radius_m: r2,
            delta_v_m_s: (vis_viva_m_s(mu, r2, to.semi_major_axis_m())
                - vis_viva_m_s(mu, r2, second.semi_major_axis_m()))
            .abs(),
        },
    ];
    let coast = half_period_s(mu, first) + half_period_s(mu, second);
    Ok(ImpulsiveTransfer::of(&burns, coast))
}

/// Edelbaum の低推力の遷移: 長半径 `a0_m` の円軌道から `a1_m` の円軌道へ、傾斜角を `delta_inclination_rad` 変える
/// 連続噴射の ΔV [m/s]。傾斜角を変えなければ円軌道の速さの差 `|v₀ − v₁|`。
pub fn edelbaum(
    mu: f64,
    a0_m: f64,
    a1_m: f64,
    delta_inclination_rad: f64,
) -> Result<f64, AstroError> {
    check_mu(mu)?;
    Apsides::new(a0_m, a1_m).check()?;
    let v0 = (mu / a0_m).sqrt();
    let v1 = (mu / a1_m).sqrt();
    Ok(combined_burn_m_s(
        v0,
        v1,
        0.5 * PI * delta_inclination_rad.abs(),
    ))
}

/// 向きを固定した噴射が弧 `arc_rad` を掃くときの ΔV の倍率 `(Φ/2) / sin(Φ/2)`（Robbins の近似。Φ → 0 で 1）。
pub fn finite_burn_factor(arc_rad: f64) -> f64 {
    let half = 0.5 * arc_rad.abs();
    if half < 1e-6 {
        1.0 + half * half / 6.0
    } else {
        half / half.sin()
    }
}

/// 有限燃焼の近似が使える弧の上限 [rad]（半周。これを超える噴射は低推力の扱いにする）。
pub const FINITE_BURN_MAX_ARC_RAD: f64 = PI;

/// 有限燃焼の 1 回の噴射の結果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiniteBurn {
    /// 損失込みの ΔV [m/s]
    pub delta_v_m_s: f64,
    /// 噴射時間 [s]
    pub burn_time_s: f64,
    /// 噴射の間に掃く弧 `n t_b` [rad]
    pub arc_rad: f64,
}

/// インパルスの ΔV `impulsive_m_s` を、推力 `thrust_n`・排気速度 `exhaust_velocity_m_s`・噴射前の質量 `initial_mass_kg` の
/// 有限燃焼で出すときの ΔV と噴射時間（[`finite_burn_factor`] と噴射時間 `t_b` を収束するまで繰り返す）。
/// `mean_motion_rad_s` は噴射する軌道の平均運動。弧が [`FINITE_BURN_MAX_ARC_RAD`] を超えたら `InvalidInput`。
pub fn finite_burn(
    impulsive_m_s: f64,
    mean_motion_rad_s: f64,
    thrust_n: f64,
    exhaust_velocity_m_s: f64,
    initial_mass_kg: f64,
) -> Result<FiniteBurn, AstroError> {
    let invalid = || {
        AstroError::InvalidInput(format!(
            "Invalid finite burn input (delta-v {impulsive_m_s} m/s, thrust {thrust_n} N, exhaust velocity {exhaust_velocity_m_s} m/s, mass {initial_mass_kg} kg)"
        ))
    };
    let mut dv = impulsive_m_s;
    for _ in 0..100 {
        let mp = propellant_for(exhaust_velocity_m_s, initial_mass_kg, dv).ok_or_else(invalid)?;
        let t = burn_time(thrust_n, exhaust_velocity_m_s, mp).ok_or_else(invalid)?;
        let arc = mean_motion_rad_s * t;
        if !arc.is_finite() || arc > FINITE_BURN_MAX_ARC_RAD {
            return Err(AstroError::InvalidInput(format!(
                "Burn exceeds half an orbit (arc {:.0} deg); the finite burn approximation is not applicable",
                arc.to_degrees()
            )));
        }
        let next = impulsive_m_s * finite_burn_factor(arc);
        if (next - dv).abs() < 1e-9 {
            let mp =
                propellant_for(exhaust_velocity_m_s, initial_mass_kg, next).ok_or_else(invalid)?;
            let t = burn_time(thrust_n, exhaust_velocity_m_s, mp).ok_or_else(invalid)?;
            return Ok(FiniteBurn {
                delta_v_m_s: next,
                burn_time_s: t,
                arc_rad: mean_motion_rad_s * t,
            });
        }
        dv = next;
    }
    Err(AstroError::InvalidInput(
        "Finite burn delta-v did not converge".to_owned(),
    ))
}

#[cfg(test)]
mod dated_tests;
#[cfg(test)]
mod flyby_tests;
#[cfg(test)]
mod interplanetary_tests;
#[cfg(test)]
mod leg_tests;
#[cfg(test)]
mod tests;
