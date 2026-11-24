//! 日付つきの惑星間遷移の途中のスイングバイ（マヌーバの `[[transfer.flybys]]`）の結果と判定。幾何・近点の噴射の計算は
//! `sat_astro::transfer::flyby_geometry`（`plan` が `dated_transfer_via` 経由で呼ぶ）、ここは通過ごとの量
//! （入り・出の超過速度・回頭角・必要な近点の半径と高度・近点の噴射）と、判定を作る。
//!
//! 判定（どちらも警告）:
//! - 回頭角が、近点の高度の下限での最大の回頭角を超える: この簡易モデルでは日付のままに曲がれない。足りない回頭の分の ΔV（簡易の見積もり）は
//!   遷移の ΔV に含めてある。
//! - 動力なしと宣言した（`powered = false`）のに、入りと出の超過速度の大きさが合わない: 差の ΔV は数えない。
//!
//! 通過は一瞬で、天体の影響圏の中の時間・双曲線の弧は扱わない（遷移の軌道の `source = "transfer"` はスイングバイのある遷移を断る）。

use super::input::{DatedTransferInput, FlybyInput};
use crate::spec::{Check, Verdict, quantity_text};
use sat_astro::ephem::CentralBody;
use sat_astro::time::Epoch;
use sat_astro::transfer::{BALLISTIC_DV_M_S, DatedTransfer, FlybyPass};
use sat_units::typed::{Angle, Length, Qty, Velocity};
use serde::{Deserialize, Serialize};

/// スイングバイ 1 回の結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FlybyResult {
    /// 通過した天体
    pub body: CentralBody,
    /// 通過の日
    pub date: Epoch,
    /// 入り・出の超過速度の大きさ
    pub v_inf_in: Qty<Velocity>,
    pub v_inf_out: Qty<Velocity>,
    /// 回頭角（入りと出の超過速度のなす角）
    pub turn_angle: Qty<Angle>,
    /// 回頭角から決まる近点の半径・高度（天体の中心・赤道半径から。曲がらない通過は無し = 無限大）。
    /// 実現できないときは必要な値で、下限より小さい（表面より下のこともある）
    #[serde(default)]
    pub periapsis_radius: Option<Qty<Length>>,
    #[serde(default)]
    pub periapsis_altitude: Option<Qty<Length>>,
    /// 近点の高度の下限（書かれた値、または天体の種類ごとの慣例の既定）
    pub min_altitude: Qty<Length>,
    /// 下限の高度での最大の回頭角
    pub max_turn_angle: Qty<Angle>,
    /// 近点の噴射の ΔV（入りと出の近点の速さの差。動力なしなら 0）
    pub delta_v: Qty<Velocity>,
    /// 実現できないときの、足りない回頭の分の ΔV（実現できるなら 0）
    pub shortfall_delta_v: Qty<Velocity>,
    /// 近点の噴射を、このマヌーバのスラスタが行うか（`false` なら ΔV に数えない）
    pub powered: bool,
    /// 回頭角が下限の高度で実現できるか
    pub feasible: bool,
}

/// 通過 `f`（入力 `input`）の結果。
fn one(f: &FlybyPass, input: Option<&FlybyInput>) -> FlybyResult {
    let radius = f.body.radius_m();
    FlybyResult {
        body: f.body,
        date: f.epoch,
        v_inf_in: Qty::from_raw(f.v_inf_in_m_s.norm()),
        v_inf_out: Qty::from_raw(f.v_inf_out_m_s.norm()),
        turn_angle: Qty::from_raw(f.turn_angle_rad),
        periapsis_radius: f.periapsis_radius_m.map(Qty::from_raw),
        periapsis_altitude: f.periapsis_radius_m.map(|r| Qty::from_raw(r - radius)),
        min_altitude: Qty::from_raw(f.min_radius_m - radius),
        max_turn_angle: Qty::from_raw(f.max_turn_rad),
        delta_v: Qty::from_raw(f.powered_dv_m_s),
        shortfall_delta_v: Qty::from_raw(f.shortfall_dv_m_s),
        powered: input.is_none_or(|i| i.powered),
        feasible: f.feasible,
    }
}

/// 遷移の通過の結果（日付の順。通過がなければ空）。
pub(super) fn results(d: &DatedTransfer, t: &DatedTransferInput) -> Vec<FlybyResult> {
    d.flybys
        .iter()
        .enumerate()
        .map(|(i, f)| one(f, t.flybys.get(i)))
        .collect()
}

/// 通過の判定（マヌーバ `maneuver` の `d`。警告だけ）。
pub(super) fn checks(maneuver: &str, d: &DatedTransfer, t: &DatedTransferInput) -> Vec<Check> {
    let mut out = Vec::new();
    for (f, input) in d.flybys.iter().zip(&t.flybys) {
        let when = f.epoch.to_iso();
        if !f.feasible {
            out.push(Check::new(
                "swing_by_turn",
                "Swing-by turn angle",
                Verdict::Warning,
                format!(
                    "The {:?} flyby of {maneuver} on {when} needs a turn of {}, above the {} possible at the minimum altitude of {}; \
                     no periapsis at or above that altitude gives this turn in the patched-conic model with these dates. \
                     The missing turn costs at least {} more (a change of the outgoing excess velocity, included in the delta-v)",
                    f.body,
                    quantity_text::<Angle>(f.turn_angle_rad),
                    quantity_text::<Angle>(f.max_turn_rad),
                    quantity_text::<Length>(f.min_radius_m - f.body.radius_m()),
                    quantity_text::<Velocity>(f.shortfall_dv_m_s),
                ),
            ));
        }
        if !input.powered && f.feasible && f.powered_dv_m_s > BALLISTIC_DV_M_S {
            out.push(Check::new(
                "swing_by_unpowered",
                "Swing-by excess speeds",
                Verdict::Warning,
                format!(
                    "The {:?} flyby of {maneuver} on {when} is declared unpowered, but the excess speeds differ ({} in, {} out); \
                     the periapsis burn of {} is not counted",
                    f.body,
                    quantity_text::<Velocity>(f.v_inf_in_m_s.norm()),
                    quantity_text::<Velocity>(f.v_inf_out_m_s.norm()),
                    quantity_text::<Velocity>(f.powered_dv_m_s),
                ),
            ));
        }
    }
    out
}
