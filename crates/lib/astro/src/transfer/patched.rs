//! パッチドコニックの遷移（中心天体を替える遷移の ΔV）。
//!
//! 天体どうしの軌道は円で同じ面にあるとみなし（半径は [`CentralBody::orbit_radius_m`]）、間をホーマン遷移でつなぐ
//! （Curtis, *Orbital Mechanics for Engineering Students*, 3rd ed., §8.2〜8.6。Vallado §12.4）。
//! 影響圏の大きさは無視し、出発・到着の天体のまわりでは双曲線（超過速度 v∞）の近点で接線方向に噴射する:
//! `Δv = √(v∞² + 2μ/r_p) − v_軌道(r_p)`（同 §8.7・§8.9）。
//!
//! | 関係 | 例 | 遷移の楕円 | 出発 | 到着 |
//! |---|---|---|---|---|
//! | 子へ | 地球 → 月 | 出発の天体のまわり（近点 = 出発の軌道の近点、遠点 = 子の軌道の半径） | 楕円の近点の噴射 | 双曲線の捕獲 |
//! | 親へ | 月 → 地球 | 到着の天体のまわり（遠点 = 出発の天体の軌道の半径、近点 = 到着の軌道の近点） | 双曲線の脱出 | 楕円の近点の噴射 |
//! | 兄弟 | 地球 → 火星 | 共通の親（太陽）のまわり（2 つの天体の軌道の半径） | 双曲線の脱出 | 双曲線の捕獲 |
//!
//! 軌道面の違い（天体の軌道の傾斜・出発の軌道の傾斜角）と打ち上げの窓（会合の時期）は扱わない。
//! 日付を決めた遷移は [`super::lambert`] で解く。

use super::{Apsides, check_mu, vis_viva_m_s};
use crate::AstroError;
use crate::ephem::CentralBody;
use core::f64::consts::PI;

/// 中心天体のまわりの噴射 1 回（パッチドコニックの出発か到着）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatchedBurn {
    /// 噴射する天体
    pub central: CentralBody,
    /// 噴射する位置の半径 [m]（その天体の中心から）
    pub radius_m: f64,
    /// 双曲線超過速度 [m/s]（影響圏の境界での天体に対する速さ）。天体のまわりの楕円のまま出発・到着するなら `None`
    pub v_inf_m_s: Option<f64>,
    /// ΔV の大きさ [m/s]
    pub delta_v_m_s: f64,
}

/// パッチドコニックの遷移。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatchedConic {
    pub departure: PatchedBurn,
    /// 到着の捕獲の噴射（捕獲の軌道を書かなければ `None`。フライバイ・大気突入）
    pub arrival: Option<PatchedBurn>,
    /// 出発の噴射のあとの特性エネルギー C3 = v² − 2μ/r [m²/s²]（脱出なら v∞²、天体のまわりの楕円なら負）
    pub c3_m2_s2: f64,
    /// 遷移の時間 [s]（遷移の楕円の半周）
    pub transfer_time_s: f64,
}

impl PatchedConic {
    /// ΔV の合計 [m/s]。
    pub fn total_m_s(&self) -> f64 {
        self.departure.delta_v_m_s + self.arrival.map_or(0.0, |a| a.delta_v_m_s)
    }
}

/// 超過速度 `v_inf` の双曲線の近点 `r_p` と、長半径 `a` の軌道の同じ点の速さの差 [m/s]。
fn hyperbolic_dv(mu: f64, r_p: f64, v_inf: f64, orbit: Apsides) -> f64 {
    let v_hyp = (v_inf * v_inf + 2.0 * mu / r_p).sqrt();
    (v_hyp - vis_viva_m_s(mu, r_p, orbit.semi_major_axis_m())).abs()
}

/// 半径 `r1` から `r2` への遷移の楕円の端の速さ（`r1` 側・`r2` 側）と半周の時間。
fn ellipse(mu: f64, r1: f64, r2: f64) -> (f64, f64, f64) {
    let a = 0.5 * (r1 + r2);
    (
        vis_viva_m_s(mu, r1, a),
        vis_viva_m_s(mu, r2, a),
        PI * (a * a * a / mu).sqrt(),
    )
}

fn radius_of(body: CentralBody) -> Result<f64, AstroError> {
    body.orbit_radius_m()
        .ok_or_else(|| AstroError::InvalidInput(format!("{body:?} does not orbit another body")))
}

/// 中心天体 `from` の軌道 `departure` から、天体 `to` の軌道 `capture`（`None` なら捕獲しない）へのパッチドコニックの遷移。
/// 出発は `departure` の近点、捕獲は `capture` の近点で噴射する。`from` と `to` が親子か兄弟でなければ `InvalidInput`。
pub fn patched_conic(
    from: CentralBody,
    departure: Apsides,
    to: CentralBody,
    capture: Option<Apsides>,
) -> Result<PatchedConic, AstroError> {
    departure.check()?;
    if let Some(c) = capture {
        c.check()?;
    }
    let rp = departure.periapsis_m;
    if to.parent() == Some(from) {
        to_child(from, departure, to, capture)
    } else if from.parent() == Some(to) {
        let capture = capture.ok_or_else(|| {
            AstroError::InvalidInput(
                "A transfer back to the parent body requires an arrival orbit".to_owned(),
            )
        })?;
        to_parent(from, departure, to, capture)
    } else if let (Some(p), true) = (from.parent(), from.parent() == to.parent()) {
        let mu = p.gm();
        check_mu(mu)?;
        let (r1, r2) = (radius_of(from)?, radius_of(to)?);
        let (v1, v2, tof) = ellipse(mu, r1, r2);
        let v_dep = (v1 - (mu / r1).sqrt()).abs();
        let v_arr = ((mu / r2).sqrt() - v2).abs();
        let departure_burn = PatchedBurn {
            central: from,
            radius_m: rp,
            v_inf_m_s: Some(v_dep),
            delta_v_m_s: hyperbolic_dv(from.gm(), rp, v_dep, departure),
        };
        Ok(PatchedConic {
            departure: departure_burn,
            arrival: capture.map(|c| capture_burn(to, v_arr, c)),
            c3_m2_s2: v_dep * v_dep,
            transfer_time_s: tof,
        })
    } else {
        Err(AstroError::InvalidInput(format!(
            "Transfer from {from:?} to {to:?} is only supported between parent and child or bodies orbiting the same parent (split it into maneuvers at intermediate bodies)"
        )))
    }
}

/// 到着の天体 `to` で超過速度 `v_inf` の双曲線から軌道 `c` に入る噴射。
fn capture_burn(to: CentralBody, v_inf: f64, c: Apsides) -> PatchedBurn {
    PatchedBurn {
        central: to,
        radius_m: c.periapsis_m,
        v_inf_m_s: Some(v_inf),
        delta_v_m_s: hyperbolic_dv(to.gm(), c.periapsis_m, v_inf, c),
    }
}

/// 親 `from` のまわりの楕円で、子の天体 `to` の軌道まで上げて捕獲する（地球 → 月）。
fn to_child(
    from: CentralBody,
    departure: Apsides,
    to: CentralBody,
    capture: Option<Apsides>,
) -> Result<PatchedConic, AstroError> {
    let mu = from.gm();
    let (rp, r_body) = (departure.periapsis_m, radius_of(to)?);
    if rp >= r_body {
        return Err(AstroError::InvalidInput(
            "The departure orbit is outside the target body's orbit".to_owned(),
        ));
    }
    let (v_p, v_a, tof) = ellipse(mu, rp, r_body);
    let v_inf = ((mu / r_body).sqrt() - v_a).abs();
    Ok(PatchedConic {
        departure: PatchedBurn {
            central: from,
            radius_m: rp,
            v_inf_m_s: None,
            delta_v_m_s: (v_p - vis_viva_m_s(mu, rp, departure.semi_major_axis_m())).abs(),
        },
        arrival: capture.map(|c| capture_burn(to, v_inf, c)),
        c3_m2_s2: v_p * v_p - 2.0 * mu / rp,
        transfer_time_s: tof,
    })
}

/// 子の天体 `from` から脱出し、親 `to` のまわりの楕円で軌道 `capture` の近点まで下げる（月 → 地球）。
fn to_parent(
    from: CentralBody,
    departure: Apsides,
    to: CentralBody,
    capture: Apsides,
) -> Result<PatchedConic, AstroError> {
    let mu = to.gm();
    let (r_body, rp) = (radius_of(from)?, capture.periapsis_m);
    if rp >= r_body {
        return Err(AstroError::InvalidInput(
            "The arrival orbit is outside the departure body's orbit".to_owned(),
        ));
    }
    let (v_a, v_p, tof) = ellipse(mu, r_body, rp);
    let v_inf = ((mu / r_body).sqrt() - v_a).abs();
    let r_dep = departure.periapsis_m;
    Ok(PatchedConic {
        departure: PatchedBurn {
            central: from,
            radius_m: r_dep,
            v_inf_m_s: Some(v_inf),
            delta_v_m_s: hyperbolic_dv(from.gm(), r_dep, v_inf, departure),
        },
        arrival: Some(PatchedBurn {
            central: to,
            radius_m: rp,
            v_inf_m_s: None,
            delta_v_m_s: (v_p - vis_viva_m_s(mu, rp, capture.semi_major_axis_m())).abs(),
        }),
        c3_m2_s2: v_inf * v_inf,
        transfer_time_s: tof,
    })
}
