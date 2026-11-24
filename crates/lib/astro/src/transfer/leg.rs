//! 日付つきの遷移の巡航の区間（パッチドコニックの太陽中心の部分）。出発の天体の影響圏を出てから到着の天体の影響圏に入るまで。
//! 途中のスイングバイ（通過）のある遷移は、通過の日を境にした脚ごとの弧の連鎖（[`cruise_trajectory`]）。影響圏で切るのは最初の脚の
//! 始めと最後の脚の終わりだけで、通過の天体の影響圏の中は解かない（通過は一瞬で、境で位置は連続・速度が跳ぶ）。
//!
//! 影響圏の中は双曲線の脱出・接近の弧が受け持ち（フェーズでは解析しない）、日心の二体の弧が成り立つのはその外だけ。
//! 弧は [`cruise_orbit`] と同じ楕円（長半径・離心率・面は同じ）で、元期を影響圏を出る時刻に進め、期間をその時刻から影響圏に
//! 入る時刻までにする。出る時刻は、弧と出発の天体の距離が影響圏の半径（[`CentralBody::soi_radius_m`]）を初めて越える時刻、
//! 入る時刻は、弧と到着の天体の距離が影響圏の半径を最後に越える時刻（どちらも二分法 `math::bisect`、天体の位置は暦）。

use super::dated::{LambertArc, arc_orbit, lambert_chain, spec_from_state};
use super::flyby::FlybyStop;
use crate::AstroError;
use crate::ephem::CentralBody;
use crate::math::bisect;
use crate::orbit::{OrbitSpec, Trajectory};
use crate::time::Epoch;

/// 距離を調べる格子の間隔 [s]（影響圏を横切るのに数時間〜数日かかるので 1 時間で挟める）。
const SCAN_STEP_S: f64 = 3600.0;
/// 影響圏の境の時刻の許容幅 [s]。
const EDGE_TOL_S: f64 = 1e-3;

/// 巡航の区間（影響圏の外の日心の弧）。通過のある遷移では脚ごと（始めの境は前の脚の終わり）。
#[derive(Debug, Clone)]
pub struct CruiseLeg {
    /// 区間の始め（出発の天体の影響圏を出る時刻）の状態を元期にした二体の軌道（[`cruise_orbit`] と同じ楕円）
    pub orbit: OrbitSpec,
    /// 区間の長さ [s]（到着の天体の影響圏に入る時刻 − 出る時刻）
    pub span_s: f64,
    /// 脚の出発の日から影響圏を出るまで [s]（最初の脚以外は 0）
    pub leave_s: f64,
    /// 脚の出発の日から影響圏に入るまで [s]（最後の脚以外は脚の飛行時間）
    pub enter_s: f64,
}

/// 巡航の区間を求める（通過のない遷移）。どちらかの天体が影響圏を持たない、弧が出発の天体の影響圏を出ない・到着の天体の影響圏の外に
/// 留まらない（出る時刻が入る時刻より前でない）ときは理由つきのエラー。[`cruise_legs`] の脚が 1 本の場合。
pub fn cruise_leg(
    from: CentralBody,
    to: CentralBody,
    depart: Epoch,
    arrive: Epoch,
) -> Result<CruiseLeg, AstroError> {
    cruise_legs(from, to, depart, arrive, &[])?
        .pop()
        .ok_or_else(|| AstroError::InvalidInput("A transfer needs at least one leg".to_owned()))
}

/// 通過（`flybys`。日付の順）のある遷移の巡航の区間を、脚（[`lambert_chain`]）ごとに求める。影響圏で切るのは最初の脚の始め
/// （出発の天体）と最後の脚の終わり（到着の天体）だけで、通過の日の側は脚の全体を使う。通過のなければ [`cruise_leg`] と同じ 1 本。
pub fn cruise_legs(
    from: CentralBody,
    to: CentralBody,
    depart: Epoch,
    arrive: Epoch,
    flybys: &[FlybyStop],
) -> Result<Vec<CruiseLeg>, AstroError> {
    let arcs = lambert_chain(from, to, depart, arrive, flybys)?;
    let mut nodes = vec![(from, depart)];
    nodes.extend(flybys.iter().map(|f| (f.body, f.date)));
    nodes.push((to, arrive));
    let last = arcs.len() - 1;
    arcs.iter()
        .enumerate()
        .map(|(i, arc)| {
            let ((a, start), (b, _)) = (nodes[i], nodes[i + 1]);
            let full = arc_orbit(arc, a, b, start)?;
            let clip = Clip {
                start: (i == 0).then_some(a),
                end: (i == last).then_some(b),
            };
            clipped_leg(arc, &full, start, (a, b), clip)
        })
        .collect()
}

/// 巡航の軌道（弧の連鎖）: [`cruise_legs`] の脚を、通過の日を境に並べた [`Trajectory`]。元期は出発の天体の影響圏を出る時刻、
/// 中心は共通の親、全体の期間は到着の天体の影響圏に入るまで。**通過のない遷移は弧 1 本で、従来の
/// `Trajectory::Elements(leg.orbit).with_span(leg.span_s)` そのもの**（[`Trajectory::chain`] が弧 1 本をそのまま返す）。
/// 境で位置が連続（通過の天体の位置を通る）であることは [`Trajectory::chain`] が検査する（許容 `BREAK_POSITION_TOL_M`）。
pub fn cruise_trajectory(
    from: CentralBody,
    to: CentralBody,
    depart: Epoch,
    arrive: Epoch,
    flybys: &[FlybyStop],
) -> Result<Trajectory, AstroError> {
    let legs = cruise_legs(from, to, depart, arrive, flybys)?;
    let dates: Vec<Epoch> = flybys.iter().map(|f| f.date).collect();
    let arcs = legs
        .into_iter()
        .enumerate()
        .map(|(i, leg)| {
            // 脚の終わりは次の通過の日（日付から決めて、隣り合う弧が丸めの食い違いなく接する）。最後の脚は影響圏に入るまで
            let span_s = dates
                .get(i)
                .map_or(leg.span_s, |d| d.seconds_since(leg.orbit.epoch));
            Trajectory::Elements(leg.orbit).with_span(span_s)
        })
        .collect();
    Trajectory::chain(arcs)
}

/// 脚の両端を影響圏で切るか（切る側の天体。`None` は切らず、脚の端まで使う）。
struct Clip {
    start: Option<CentralBody>,
    end: Option<CentralBody>,
}

/// 脚の弧 `arc`（`full` は出発の日 `depart` を元期にした同じ楕円、`(from, to)` は両端の天体）を `clip` のとおり影響圏で切る。
fn clipped_leg(
    arc: &LambertArc,
    full: &OrbitSpec,
    depart: Epoch,
    (from, to): (CentralBody, CentralBody),
    clip: Clip,
) -> Result<CruiseLeg, AstroError> {
    let tof = arc.transfer_time_s;
    let soi = |body: Option<CentralBody>| -> Result<Option<(CentralBody, f64)>, AstroError> {
        body.map(|b| {
            b.soi_radius_m().map(|r| (b, r)).ok_or_else(|| {
                AstroError::InvalidInput(format!(
                    "The cruise leg is bounded by spheres of influence, but {from:?} or {to:?} has none"
                ))
            })
        })
        .transpose()
    };
    let (out_of, into) = (soi(clip.start)?, soi(clip.end)?);
    // 弧の上の探査機と天体の距離 − 影響圏の半径（日心の位置。天体は暦）
    let margin = |body: CentralBody, soi: f64, t: f64| -> f64 {
        let at = depart.plus_seconds(t);
        match (full.state_at(t), body.state_about_parent(at)) {
            (Ok(sc), Some(b)) => (sc.r_eci - b.r_eci).norm() - soi,
            _ => f64::NAN,
        }
    };
    let steps = (tof / SCAN_STEP_S).ceil() as usize;
    let at = |k: usize| (k as f64 * SCAN_STEP_S).min(tof);
    let leave = match out_of {
        None => Some(0.0),
        Some((body, r)) => {
            let out = |t: f64| margin(body, r, t);
            (1..=steps)
                .find(|&k| out(at(k)) > 0.0)
                .and_then(|k| bisect(out, at(k - 1), at(k), EDGE_TOL_S, 200))
        }
    };
    let enter = match into {
        None => Some(tof),
        Some((body, r)) => {
            let inn = |t: f64| margin(body, r, t);
            (0..steps)
                .map(|k| steps - k)
                .find(|&k| inn(at(k - 1)) > 0.0)
                .and_then(|k| bisect(inn, at(k - 1), at(k), EDGE_TOL_S, 200))
        }
    };
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
    // 始めを切らない脚は、出発の日の状態の楕円そのまま（元期は出発の日）
    let orbit = if out_of.is_some() {
        let mut o = spec_from_state(
            &full.state_at(leave_s)?,
            full.central.gm(),
            depart.plus_seconds(leave_s),
        )?;
        o.central = full.central;
        o
    } else {
        full.clone()
    };
    Ok(CruiseLeg {
        orbit,
        span_s: enter_s - leave_s,
        leave_s,
        enter_s,
    })
}
