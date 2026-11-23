//! ヒータの結果（定常・非定常）と判定。**ヒータの判定（key `heater:<部品>`）はここだけ**（警告のデューティの既定値は `range::policy`）。
//! 制御則と定常の定義は [`network::heater`](crate::thermal::network)・`network::heater_steady`、非定常の時間積分は `transient::heater_control`。
//! 定常・非定常で共有する（解析どうしは互いに参照しない）。
//!
//! 判定: 飽和（容量いっぱいでも下側の設定温度を保てない）は違反、デューティが警告の値を超えれば警告、それ以外は OK。

use crate::spec::{Check, Verdict, quantity_text};
use crate::thermal::network::ThermalHeaterState;
use sat_units::typed::{Fraction, Power, Qty, Temperature, Time};
use serde::{Deserialize, Serialize};

/// 定常のヒータ 1 つの結果（1 ケース）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalHeaterSteady {
    /// 部品 ID
    pub id: String,
    pub node: String,
    pub state: ThermalHeaterState,
    /// 節点を下側の設定温度に保つのに出す電力（オフ・無効は 0、飽和は容量）
    pub power: Qty<Power>,
    /// 容量（オンのときの発熱 × 員数）
    pub capacity: Qty<Power>,
    /// 電力 / 容量
    pub duty_fraction: Fraction,
    /// 下側の設定温度（この温度に保つ）
    pub set_point: Qty<Temperature>,
    /// 節点の温度（制御中は設定温度、飽和ならそれ未満）
    pub node_temperature: Qty<Temperature>,
}

/// 非定常のヒータ 1 つの結果（走行全体）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalHeaterRun {
    /// 部品 ID
    pub id: String,
    pub node: String,
    /// 容量（オンのときの発熱 × 員数）
    pub capacity: Qty<Power>,
    pub on_below: Qty<Temperature>,
    pub off_above: Qty<Temperature>,
    /// 走行全体でオンだった時間の割合（初期の立ち上がりを含む）
    pub duty_fraction: Fraction,
    /// 最後の 1 周（走行が 1 周に満たなければ全体）でオンだった時間の割合。判定に使う
    pub duty_fraction_last_orbit: Fraction,
    /// 走行全体の平均電力（= デューティ × 容量。電力の解析の常時オンの値と比べる）
    pub mean_power: Qty<Power>,
    /// オンとオフの切り替わりの回数
    pub switchings: u32,
    /// 連続してオンだった最長の時間（走行の終わりでオンなら、そこまで）
    pub max_on_duration: Qty<Time>,
    /// 最後の 1 周ずっとオンで、節点の温度が一度も下側の設定温度に届かなかった（容量が足りない）
    pub saturated: bool,
}

/// 定常の判定（key `heater:<部品>`）。ケースをまたいで、飽和があればそのケース、なければデューティが最大のケースで判定する。
/// `cases` は (ケース ID, そのケースのヒータの結果)。`warning` はデューティの警告の値（`None` は既定値）。
pub(crate) fn steady_checks(cases: &[(&str, &[ThermalHeaterSteady])], warning: f64) -> Vec<Check> {
    let Some((_, first)) = cases.first() else {
        return Vec::new();
    };
    first
        .iter()
        .map(|h0| {
            let worst = cases
                .iter()
                .filter_map(|(case, hs)| Some((*case, hs.iter().find(|h| h.id == h0.id)?)))
                .max_by(|(_, a), (_, b)| rank(a).total_cmp(&rank(b)))
                .unwrap_or((cases[0].0, h0));
            steady_check(worst.0, worst.1, warning)
        })
        .collect()
}

/// 判定の悪さの順（飽和 > デューティ）。
fn rank(h: &ThermalHeaterSteady) -> f64 {
    let saturated = f64::from(u8::from(h.state == ThermalHeaterState::Saturated));
    saturated * 2.0 + h.duty_fraction.value()
}

fn steady_check(case: &str, h: &ThermalHeaterSteady, warning: f64) -> Check {
    let (p, t) = (quantity_text::<Power>, quantity_text::<Temperature>);
    let key = sat_checkkey::heater(&h.id);
    let label = format!("{} heater duty", h.id);
    let duty = quantity_text::<Fraction>(h.duty_fraction.value());
    let (verdict, message) = match h.state {
        ThermalHeaterState::Saturated => (
            Verdict::Violation,
            format!(
                "Case {case}: saturated. Full power {} cannot hold the lower set point {} (node at {})",
                p(h.capacity.value()),
                t(h.set_point.value()),
                t(h.node_temperature.value())
            ),
        ),
        ThermalHeaterState::Disabled => (
            Verdict::Ok,
            format!("Case {case}: thermostat disabled by the mode"),
        ),
        _ => {
            let verdict = if h.duty_fraction.value() > warning {
                Verdict::Warning
            } else {
                Verdict::Ok
            };
            (
                verdict,
                format!(
                    "Case {case}: duty {duty} ({} of {} capacity) to hold {}; warning above {}",
                    p(h.power.value()),
                    p(h.capacity.value()),
                    t(h.set_point.value()),
                    quantity_text::<Fraction>(warning)
                ),
            )
        }
    };
    Check::new(&key, &label, verdict, message)
}

/// 非定常の判定（key `heater:<部品>`）。最後の 1 周のデューティで判定する。
pub(crate) fn run_checks(heaters: &[ThermalHeaterRun], warning: f64) -> Vec<Check> {
    let (p, t) = (quantity_text::<Power>, quantity_text::<Temperature>);
    heaters
        .iter()
        .map(|h| {
            let duty = quantity_text::<Fraction>(h.duty_fraction_last_orbit.value());
            let (verdict, message) = if h.saturated {
                (
                    Verdict::Violation,
                    format!(
                        "Saturated: on for the whole last orbit and the node never reached the lower set point {}. Capacity {}",
                        t(h.on_below.value()),
                        p(h.capacity.value())
                    ),
                )
            } else {
                let verdict = if h.duty_fraction_last_orbit.value() > warning {
                    Verdict::Warning
                } else {
                    Verdict::Ok
                };
                (
                    verdict,
                    format!(
                        "Duty {duty} over the last orbit, mean power {} of {} capacity, {} switchings; warning above {}",
                        p(h.mean_power.value()),
                        p(h.capacity.value()),
                        h.switchings,
                        quantity_text::<Fraction>(warning)
                    ),
                )
            };
            Check::new(
                &sat_checkkey::heater(&h.id),
                &format!("{} heater duty", h.id),
                verdict,
                message,
            )
        })
        .collect()
}
