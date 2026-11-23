//! 日付つきの惑星間遷移（マヌーバの `transfer`）の結果と判定。遷移の計算は `sat_astro::transfer::dated_transfer`
//! （`plan` が呼ぶ）、ここは結果の量（C3・両端の超過速度・出発の赤緯・飛行時間・暦の出所）と、判定を作る。
//!
//! 判定（どちらも警告）:
//! - 出発の漸近線の赤緯の絶対値が出発の駐車軌道の傾斜角より大きい: 軌道面を変えずには漸近線に出られない（面の変更は数えない）。
//! - 前のマヌーバを最初の軌道の元期から続けて数えると、そのマヌーバの終わりが出発の日より後になる
//!   （実行の順の時刻の置き方は [`super::history`] と同じ。待ちの時間はフェーズ（#9）で足す）。

use super::input::PropulsionInput;
use super::sequence::{Leg, Sequence};
use crate::spec::{Check, Verdict, quantity_text};
use sat_astro::ephem::EphemerisSource;
use sat_astro::time::Epoch;
use sat_units::typed::{Angle, Qty, SpecificEnergy, Time, Velocity};
use serde::{Deserialize, Serialize};

/// 日付つきの遷移の結果（マヌーバ 1 つ）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DatedTransferResult {
    /// マヌーバの ID
    pub maneuver: String,
    /// 出発の日・到着の日
    pub departure: Epoch,
    pub arrival: Epoch,
    /// 飛行時間（到着 − 出発）
    pub time_of_flight: Qty<Time>,
    /// 出発の特性エネルギー C3 = |v∞|²
    pub c3: Qty<SpecificEnergy>,
    /// 出発・到着の双曲線超過速度の大きさ
    pub v_inf_departure: Qty<Velocity>,
    pub v_inf_arrival: Qty<Velocity>,
    /// 出発の漸近線の赤緯（出発天体の赤道に対して）
    pub departure_declination: Qty<Angle>,
    /// 天体の位置の出所（出発・到着の両方が DE440 のときだけ `de440`。ΔV は出所で少し変わる）
    pub ephemeris: EphemerisSource,
}

/// 区間の日付つきの遷移の結果（日付の無いマヌーバは `None`）。
pub(super) fn result(leg: &Leg<'_>) -> Option<DatedTransferResult> {
    let d = leg.planned.dated?;
    let t = leg.maneuver.transfer?;
    Some(DatedTransferResult {
        maneuver: leg.maneuver.id.clone(),
        departure: t.departure,
        arrival: t.arrival,
        time_of_flight: Qty::from_raw(d.transfer_time_s),
        c3: Qty::from_raw(d.c3_m2_s2),
        v_inf_departure: Qty::from_raw(d.v_inf_departure_m_s.norm()),
        v_inf_arrival: Qty::from_raw(d.v_inf_arrival_m_s.norm()),
        departure_declination: Qty::from_raw(d.departure_declination_rad),
        ephemeris: d.ephemeris,
    })
}

/// `i` 番目のマヌーバの始まりの時刻（最初の軌道の元期からの経過 [s]。前のマヌーバの終わり。最初は 0）。
fn start_s(seq: &Sequence<'_>, i: usize) -> f64 {
    let Some(prev) = i.checked_sub(1).and_then(|p| seq.legs.get(p)) else {
        return 0.0;
    };
    seq.history
        .segments
        .iter()
        .rev()
        .find(|s| s.maneuver.as_deref() == Some(prev.maneuver.id.as_str()))
        .map_or(0.0, |s| s.start.value())
}

/// `i` 番目のマヌーバの日付つきの遷移の判定（日付の無いマヌーバは空）。
pub(super) fn checks(seq: &Sequence<'_>, input: &PropulsionInput, i: usize) -> Vec<Check> {
    let Some(leg) = seq.legs.get(i) else {
        return Vec::new();
    };
    let (Some(d), Some(t)) = (leg.planned.dated, leg.maneuver.transfer) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let inclination = leg.planned.from.inclination_rad;
    if d.departure_declination_rad.abs() > inclination {
        out.push(Check::new(
            "dated_transfer_plane",
            "Departure asymptote declination",
            Verdict::Warning,
            format!(
                "The departure asymptote of {} has a declination of {}, above the parking orbit inclination of {}, so the parking orbit plane cannot reach it without a plane change (not counted)",
                leg.maneuver.id,
                quantity_text::<Angle>(d.departure_declination_rad.abs()),
                quantity_text::<Angle>(inclination),
            ),
        ));
    }
    let ready: Epoch = input.initial_orbit.epoch.plus_seconds(start_s(seq, i));
    if ready.seconds_since(t.departure) > 0.0 {
        out.push(Check::new(
            "dated_transfer_timing",
            "Departure date",
            Verdict::Warning,
            format!(
                "The maneuvers before {} end at {} when laid end to end from the first orbit epoch, which is {} after the departure date {}",
                leg.maneuver.id,
                ready.to_iso(),
                quantity_text::<Time>(ready.seconds_since(t.departure)),
                t.departure.to_iso(),
            ),
        ));
    }
    out
}
