//! 日付つきの遷移の巡航の区間（パッチドコニックの太陽中心の部分）。出発の天体の影響圏を出てから到着の天体の影響圏に入るまで。
//!
//! 影響圏の中は双曲線の脱出・接近の弧が受け持ち（フェーズでは解析しない）、日心の二体の弧が成り立つのはその外だけ。
//! 弧は [`cruise_orbit`] と同じ楕円（長半径・離心率・面は同じ）で、元期を影響圏を出る時刻に進め、期間をその時刻から影響圏に
//! 入る時刻までにする。出る時刻は、弧と出発の天体の距離が影響圏の半径（[`CentralBody::soi_radius_m`]）を初めて越える時刻、
//! 入る時刻は、弧と到着の天体の距離が影響圏の半径を最後に越える時刻（どちらも二分法 `math::bisect`、天体の位置は暦）。

use super::dated::{cruise_orbit, spec_from_state};
use crate::AstroError;
use crate::ephem::CentralBody;
use crate::math::bisect;
use crate::orbit::OrbitSpec;
use crate::time::Epoch;

/// 距離を調べる格子の間隔 [s]（影響圏を横切るのに数時間〜数日かかるので 1 時間で挟める）。
const SCAN_STEP_S: f64 = 3600.0;
/// 影響圏の境の時刻の許容幅 [s]。
const EDGE_TOL_S: f64 = 1e-3;

/// 巡航の区間（影響圏の外の日心の弧）。
#[derive(Debug, Clone)]
pub struct CruiseLeg {
    /// 区間の始め（出発の天体の影響圏を出る時刻）の状態を元期にした二体の軌道（[`cruise_orbit`] と同じ楕円）
    pub orbit: OrbitSpec,
    /// 区間の長さ [s]（到着の天体の影響圏に入る時刻 − 出る時刻）
    pub span_s: f64,
    /// 出発の日から影響圏を出るまで [s]
    pub leave_s: f64,
    /// 出発の日から影響圏に入るまで [s]
    pub enter_s: f64,
}

/// 巡航の区間を求める。どちらかの天体が影響圏を持たない、弧が出発の天体の影響圏を出ない・到着の天体の影響圏の外に
/// 留まらない（出る時刻が入る時刻より前でない）ときは理由つきのエラー。
pub fn cruise_leg(
    from: CentralBody,
    to: CentralBody,
    depart: Epoch,
    arrive: Epoch,
) -> Result<CruiseLeg, AstroError> {
    let full = cruise_orbit(from, to, depart, arrive)?;
    let tof = arrive.seconds_since(depart);
    let (Some(r_from), Some(r_to)) = (from.soi_radius_m(), to.soi_radius_m()) else {
        return Err(AstroError::InvalidInput(format!(
            "The cruise leg is bounded by spheres of influence, but {from:?} or {to:?} has none"
        )));
    };
    // 弧の上の探査機と天体の距離 − 影響圏の半径（日心の位置。天体は暦）
    let margin = |body: CentralBody, soi: f64, t: f64| -> f64 {
        let at = depart.plus_seconds(t);
        match (full.state_at(t), body.state_about_parent(at)) {
            (Ok(sc), Some(b)) => (sc.r_eci - b.r_eci).norm() - soi,
            _ => f64::NAN,
        }
    };
    let out = |t: f64| margin(from, r_from, t);
    let inn = |t: f64| margin(to, r_to, t);
    let steps = (tof / SCAN_STEP_S).ceil() as usize;
    let at = |k: usize| (k as f64 * SCAN_STEP_S).min(tof);
    let leave = (1..=steps)
        .find(|&k| out(at(k)) > 0.0)
        .and_then(|k| bisect(out, at(k - 1), at(k), EDGE_TOL_S, 200));
    let enter = (0..steps)
        .map(|k| steps - k)
        .find(|&k| inn(at(k - 1)) > 0.0)
        .and_then(|k| bisect(inn, at(k - 1), at(k), EDGE_TOL_S, 200));
    let (Some(leave_s), Some(enter_s)) = (leave, enter) else {
        return Err(AstroError::InvalidInput(format!(
            "The transfer arc from {from:?} to {to:?} does not leave the sphere of influence of the departure body or does not stay outside that of the arrival body"
        )));
    };
    if enter_s <= leave_s {
        return Err(AstroError::InvalidInput(format!(
            "The transfer arc from {from:?} to {to:?} leaves the departure sphere of influence only after it enters the arrival one"
        )));
    }
    let start = full.state_at(leave_s)?;
    let mut orbit = spec_from_state(&start, full.central.gm(), depart.plus_seconds(leave_s))?;
    orbit.central = full.central;
    Ok(CruiseLeg {
        orbit,
        span_s: enter_s - leave_s,
        leave_s,
        enter_s,
    })
}
