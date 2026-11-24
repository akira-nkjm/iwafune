//! 日付つきの惑星間遷移の多体の確認（`propulsion.transfer_check`）。Lambert（パッチドコニック）で求めた遷移を、
//! 太陽と惑星の点質量の重力で数値伝搬し直して、到着の日の到着天体とのずれ・最接近・到着の超過速度の差を見る。
//! 伝搬と最接近の探索は `sat_astro::transfer::check_cruise`（既存の数値伝搬・力モデル・積分器・暦を使う）。
//! ここはマヌーバの遷移を渡して結果を量と判定に直すだけ。
//!
//! 単純化（結果の注記 [`TransferCheckNote::PointMassCruise`] に出す）: 出発天体の中心から Lambert の速度で始め、
//! 出発と到着の天体の重力は入れない（両端の影響圏の中はパッチドコニックの噴射で扱う）。ほかの惑星は点質量の第三体。
//!
//! 判定は暦が DE440 のときだけ（解析式の暦では、ずれの大きさが暦の誤差と同程度で意味を持たないので、
//! 数値は出すが判定しない。[`TransferCheckNote::AnalyticEphemerisNotJudged`]）:
//! - 到着の日のずれが到着天体の影響圏の半径以内なら `Ok`、外なら警告（目標への補正が要る）。
//! - 最接近が影響圏の外なら警告（探査機が到着天体の影響圏に入らない）。

use super::input::PropulsionInput;
use super::maneuvers::nominal;
use super::sequence::Leg;
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Channel, ChannelKind, Check, Series,
    System, Verdict, quantity_text,
};
use sat_astro::ephem::{CentralBody, EphemerisSource};
use sat_astro::time::Epoch;
use sat_astro::transfer::{CruiseCheck, CruiseSample, check_cruise, check_cruise_leg};
use sat_units::typed::{Length, Qty, Time, Velocity};
use serde::{Deserialize, Serialize};

/// 結果の注記。**文は持たない**（service の語彙が引く）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TransferCheckNote {
    /// 出発天体の中心から Lambert の速度で始め、出発・到着の天体の重力は入れず、ほかの惑星を点質量で入れた
    PointMassCruise,
    /// 天体の位置が解析式（DE440 でない）なので、ずれの大きさは暦の誤差と同程度で、判定しない
    AnalyticEphemerisNotJudged,
    /// スイングバイのある遷移なので脚ごとに伝搬した（各脚は自分の Lambert の速度から始まり、通過の前後はつなげていない）
    LegsNotChained,
    /// 最接近が探した期間の端で、期間の中に極小が無い（まだ近づいている）
    ClosestApproachAtEnd,
}

/// 遷移 1 つの確認の結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TransferCheckResult {
    /// マヌーバの ID
    pub maneuver: String,
    /// 通過のある遷移の脚の番号（1 から。通過のない遷移は無し。`from`・`to`・日付はその脚のもの）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub leg: Option<u32>,
    /// 出発・到着の天体
    pub from: CentralBody,
    pub to: CentralBody,
    /// 出発の日・到着の日
    pub departure: Epoch,
    pub arrival: Epoch,
    /// 天体の位置の出所
    pub ephemeris: EphemerisSource,
    /// 判定したか（DE440 のときだけ）
    pub judged: bool,
    /// 到着の日の、到着天体の中心からのずれ
    pub miss_distance: Qty<Length>,
    /// 到着天体の影響圏の半径
    pub arrival_soi_radius: Option<Qty<Length>>,
    /// ずれ / 影響圏の半径
    pub miss_in_soi_radii: Option<f64>,
    /// 最接近の距離と時刻（出発の日からの経過・日付）
    pub closest_approach_distance: Qty<Length>,
    pub closest_approach_time: Qty<Time>,
    pub closest_approach_epoch: Epoch,
    /// 到着の超過速度の大きさ（Lambert・伝搬）と、そのベクトルの差の大きさ
    pub v_inf_arrival_lambert: Qty<Velocity>,
    pub v_inf_arrival_propagated: Qty<Velocity>,
    pub v_inf_arrival_difference: Qty<Velocity>,
    /// 第三体に入れた天体
    pub perturbers: Vec<CentralBody>,
    pub notes: Vec<TransferCheckNote>,
    pub checks: Vec<Check>,
    /// 軌跡（出発の日からの経過。黄道面の日心の x・y と、到着天体までの距離）
    pub series: Series,
}

/// `propulsion.transfer_check` の出力。日付つきの遷移が無ければ `transfers` は空。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TransferCheckOutput {
    pub transfers: Vec<TransferCheckResult>,
}

/// 判定（DE440 のときだけ）。ずれ `miss_m`・最接近 `closest_m` を影響圏の半径 `soi_m` と比べる。
pub(super) fn judge(
    maneuver: &str,
    to: CentralBody,
    (miss_m, closest_m, soi_m): (f64, f64, f64),
) -> Vec<Check> {
    let length = quantity_text::<Length>;
    let miss = Check::new(
        "transfer_check_arrival_miss",
        "Arrival miss distance",
        if miss_m <= soi_m {
            Verdict::Ok
        } else {
            Verdict::Warning
        },
        if miss_m <= soi_m {
            format!(
                "The propagated cruise of {maneuver} is {} from {to:?} on the arrival date, within its sphere of influence ({})",
                length(miss_m),
                length(soi_m),
            )
        } else {
            format!(
                "The propagated cruise of {maneuver} is {} from {to:?} on the arrival date, outside its sphere of influence ({}), so targeting or trajectory correction maneuvers are needed (not counted in the delta-V)",
                length(miss_m),
                length(soi_m),
            )
        },
    );
    let approach = Check::new(
        "transfer_check_closest_approach",
        "Closest approach",
        if closest_m <= soi_m {
            Verdict::Ok
        } else {
            Verdict::Warning
        },
        if closest_m <= soi_m {
            format!(
                "The closest approach of {maneuver} to {to:?} is {}, inside its sphere of influence",
                length(closest_m),
            )
        } else {
            format!(
                "The closest approach of {maneuver} to {to:?} is {}, outside its sphere of influence ({}): the spacecraft does not reach it without a correction",
                length(closest_m),
                length(soi_m),
            )
        },
    );
    vec![miss, approach]
}

/// 軌跡の時系列（出発の日からの経過 [s]。黄道面の日心の x・y [m] と到着天体までの距離）。
fn series(samples: &[CruiseSample]) -> Series {
    let column = |f: &dyn Fn(&CruiseSample) -> f64| samples.iter().map(f).collect::<Vec<f64>>();
    let xy = |subject: &str, pick: fn(&CruiseSample) -> [f64; 2]| {
        [
            Channel::of::<Length>(
                format!("{subject}_x"),
                ChannelKind::HeliocentricX,
                column(&|s| pick(s)[0]),
            )
            .about(subject),
            Channel::of::<Length>(
                format!("{subject}_y"),
                ChannelKind::HeliocentricY,
                column(&|s| pick(s)[1]),
            )
            .about(subject),
        ]
    };
    let mut channels = Vec::new();
    channels.extend(xy("spacecraft", |s| s.spacecraft_xy_m));
    channels.extend(xy("departure", |s| s.from_xy_m));
    channels.extend(xy("arrival", |s| s.to_xy_m));
    channels.push(Channel::of::<Length>(
        "range_to_arrival",
        ChannelKind::ArrivalRange,
        column(&|s| s.range_to_arrival_m),
    ));
    Series {
        t: column(&|s| s.t_s),
        channels,
    }
}

/// 区間の遷移の確認（日付つきの遷移が無ければ空）。通過のない遷移は 1 つ、通過のある遷移は脚ごとに 1 つ
/// （通過の前後をつなげた伝搬はしない。注記 [`TransferCheckNote::LegsNotChained`]）。
fn check_leg(leg: &Leg<'_>) -> Result<Vec<TransferCheckResult>, AnalysisError> {
    let (Some(dated), Some(t)) = (leg.planned.dated.as_ref(), leg.maneuver.transfer.as_ref())
    else {
        return Ok(Vec::new());
    };
    let (from, to) = (leg.planned.from.central, leg.planned.to.central);
    if dated.flybys.is_empty() {
        let c = check_cruise(dated, from, to, t.departure)?;
        let args = (from, to, t.departure, t.arrival);
        return Ok(vec![result(&leg.maneuver.id, None, args, dated, &c)]);
    }
    // 脚 i は、節点 i（出発・通過）から節点 i+1（通過・到着）まで
    let mut nodes = vec![(from, t.departure)];
    nodes.extend(t.flybys.iter().map(|f| (f.body, f.date)));
    nodes.push((to, t.arrival));
    dated
        .legs
        .iter()
        .zip(nodes.windows(2))
        .enumerate()
        .map(|(i, (arc, pair))| {
            let ((a, ta), (b, tb)) = (pair[0], pair[1]);
            let c = check_cruise_leg(arc, a, b, ta, dated.ephemeris)?;
            let leg_no = u32::try_from(i + 1).ok();
            Ok(result(&leg.maneuver.id, leg_no, (a, b, ta, tb), dated, &c))
        })
        .collect()
}

fn result(
    maneuver: &str,
    leg: Option<u32>,
    (from, to, departure, arrival): (CentralBody, CentralBody, Epoch, Epoch),
    dated: &sat_astro::transfer::DatedTransfer,
    c: &CruiseCheck,
) -> TransferCheckResult {
    let judged = c.ephemeris == EphemerisSource::De440;
    let miss = c.miss_distance_m();
    let mut notes = vec![TransferCheckNote::PointMassCruise];
    if leg.is_some() {
        notes.push(TransferCheckNote::LegsNotChained);
    }
    if !judged {
        notes.push(TransferCheckNote::AnalyticEphemerisNotJudged);
    }
    if c.closest.at_boundary {
        notes.push(TransferCheckNote::ClosestApproachAtEnd);
    }
    let checks = match c.arrival_soi_m.filter(|_| judged) {
        Some(soi) => judge(maneuver, to, (miss, c.closest.distance_m, soi)),
        None => Vec::new(),
    };
    // 脚の到着の超過速度の Lambert の値（最後の脚は遷移の到着、途中の脚はその脚の到着の速度 − 到着天体の速度）
    let v_inf_lambert = match leg.and_then(|n| dated.legs.get(n as usize - 1)) {
        Some(arc) => arc.v_arrival_m_s - arc.to_state.v_eci,
        None => dated.v_inf_arrival_m_s,
    };
    TransferCheckResult {
        maneuver: maneuver.to_owned(),
        leg,
        from,
        to,
        departure,
        arrival,
        ephemeris: c.ephemeris,
        judged,
        miss_distance: Qty::from_raw(miss),
        arrival_soi_radius: c.arrival_soi_m.map(Qty::from_raw),
        miss_in_soi_radii: c.arrival_soi_m.map(|soi| miss / soi),
        closest_approach_distance: Qty::from_raw(c.closest.distance_m),
        closest_approach_time: Qty::from_raw(c.closest.t_s),
        closest_approach_epoch: departure.plus_seconds(c.closest.t_s),
        v_inf_arrival_lambert: Qty::from_raw(v_inf_lambert.norm()),
        v_inf_arrival_propagated: Qty::from_raw(c.v_inf_arrival_m_s.norm()),
        v_inf_arrival_difference: Qty::from_raw((c.v_inf_arrival_m_s - v_inf_lambert).norm()),
        perturbers: c.perturbers.clone(),
        notes,
        checks,
        series: series(&c.samples),
    }
}

/// 日付つきの遷移の多体の確認。
pub struct TransferCheck;

impl Analysis for TransferCheck {
    const ID: &'static str = "propulsion.transfer_check";
    const TITLE: &'static str = "遷移の多体の確認";
    const TITLE_EN: &'static str = "Transfer check (n-body)";
    const AREA: AnalysisArea = AnalysisArea::System(System::Propulsion);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "日付つきの惑星間遷移を太陽と惑星の重力で数値伝搬し直し、到着の日のずれ・最接近・到着の超過速度の差と軌跡を出す（判定は DE440 のときだけ）";
    const DESCRIPTION_EN: &'static str = "Re-propagates each dated interplanetary transfer under the gravity of the Sun and planets: miss distance on the arrival date, closest approach, arrival v-infinity difference and the trajectory (judged only with DE440)";

    type Input = PropulsionInput;
    type Output = TransferCheckOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let seq = nominal(input)?;
        let transfers = seq
            .legs
            .iter()
            .map(check_leg)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(TransferCheckOutput { transfers })
    }
}
