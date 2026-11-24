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
    System, Verdict, quantity_text, ratio_margin,
};
use sat_astro::ephem::{CentralBody, EphemerisSource};
use sat_astro::time::Epoch;
use sat_astro::transfer::{CruiseCheck, CruiseSample, check_cruise};
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
    /// 最接近が探した期間の端で、期間の中に極小が無い（まだ近づいている）
    ClosestApproachAtEnd,
}

/// 遷移 1 つの確認の結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TransferCheckResult {
    /// マヌーバの ID
    pub maneuver: String,
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
    )
    // 余裕 = 影響圏の半径 / ずれ − 1
    .with_ratio_margin(ratio_margin(soi_m, miss_m));
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
    )
    .with_ratio_margin(ratio_margin(soi_m, closest_m));
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

/// 区間の遷移の確認（日付つきの遷移が無ければ `None`）。
fn check_leg(leg: &Leg<'_>) -> Result<Option<TransferCheckResult>, AnalysisError> {
    let (Some(dated), Some(t)) = (leg.planned.dated, leg.maneuver.transfer) else {
        return Ok(None);
    };
    let (from, to) = (leg.planned.from.central, leg.planned.to.central);
    let c = check_cruise(&dated, from, to, t.departure)?;
    Ok(Some(result(
        &leg.maneuver.id,
        (from, to),
        t.departure,
        t.arrival,
        &dated,
        &c,
    )))
}

fn result(
    maneuver: &str,
    (from, to): (CentralBody, CentralBody),
    departure: Epoch,
    arrival: Epoch,
    dated: &sat_astro::transfer::DatedTransfer,
    c: &CruiseCheck,
) -> TransferCheckResult {
    let judged = c.ephemeris == EphemerisSource::De440;
    let miss = c.miss_distance_m();
    let mut notes = vec![TransferCheckNote::PointMassCruise];
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
    TransferCheckResult {
        maneuver: maneuver.to_owned(),
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
        v_inf_arrival_lambert: Qty::from_raw(dated.v_inf_arrival_m_s.norm()),
        v_inf_arrival_propagated: Qty::from_raw(c.v_inf_arrival_m_s.norm()),
        v_inf_arrival_difference: Qty::from_raw(
            (c.v_inf_arrival_m_s - dated.v_inf_arrival_m_s).norm(),
        ),
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
