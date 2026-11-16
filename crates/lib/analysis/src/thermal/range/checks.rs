//! 温度範囲の比較の判定文（key `range:<節点>:<種類>`）。比較と判定は親の `thermal::range`、ここは文を作るだけ。

use super::ThermalMarginUsed;
use super::{ThermalPowerState, ThermalRangeExclusion, ThermalRangeMargin};
use crate::spec::{Check, quantity_text};
use sat_units::typed::Temperature;

/// 種類ごとの判定（key は `range:<節点>:<種類>`）。予測の最低・最高は比較の余裕から戻す。電源の状態で外した部品があれば
/// 判定文に書く。温度の表示は sat-units に委譲する。
pub(crate) fn range_checks(
    node: &str,
    margins: &[ThermalRangeMargin],
    margin: &ThermalMarginUsed,
) -> Vec<Check> {
    let t = quantity_text::<Temperature>;
    let u = margin.uncertainty.value();
    margins
        .iter()
        .map(|m| {
            let (lo, hi) = m.predicted();
            let predicted = if u > 0.0 {
                format!(
                    "{}〜{}、不確かさ込み {}〜{}",
                    t(lo),
                    t(hi),
                    t(lo - u),
                    t(hi + u)
                )
            } else {
                format!("{}〜{}", t(lo), t(hi))
            };
            Check::new(
                &format!("range:{node}:{}", m.kind.key()),
                &format!("{node} の温度（{}）", m.kind.noun()),
                m.verdict,
                format!(
                    "{predicted}（{} {}〜{}{}）",
                    m.kind.noun(),
                    t(m.range.min.value()),
                    t(m.range.max.value()),
                    excluded_text(&m.excluded)
                ),
            )
        })
        .collect()
}

/// 外した部品の文（「。モード SAFE で電源オフの STX・UHF を除く」。無ければ空）。モードと電源の組ごとに部品をまとめ、
/// 同じ部品の組を外したモードは 1 つの文にする（「モード NOMINAL・MISSION で…」）。
pub(super) fn excluded_text(excluded: &[ThermalRangeExclusion]) -> String {
    let mut by_mode: Vec<((&str, ThermalPowerState), Vec<&str>)> = Vec::new();
    for e in excluded {
        let key = (e.mode.as_str(), e.power_state);
        match by_mode.iter_mut().find(|(k, _)| *k == key) {
            Some((_, parts)) => parts.push(&e.part),
            None => by_mode.push((key, vec![&e.part])),
        }
    }
    let mut groups: Vec<(Vec<&str>, ThermalPowerState, Vec<&str>)> = Vec::new();
    for ((mode, power), parts) in by_mode {
        match groups
            .iter_mut()
            .find(|(_, p, ps)| *p == power && *ps == parts)
        {
            Some((modes, _, _)) => modes.push(mode),
            None => groups.push((vec![mode], power, parts)),
        }
    }
    groups
        .iter()
        .map(|(modes, power, parts)| {
            format!(
                "。モード {} で{}の {} を除く",
                modes.join("・"),
                power.noun(),
                parts.join("・")
            )
        })
        .collect()
}
