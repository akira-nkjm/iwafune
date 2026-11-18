//! 温度範囲の比較の判定文（key `range_high:<節点>:<種類>` と `range_low:<節点>:<種類>`）。比較と判定は親の `thermal::range`、ここは文を作るだけ。

use super::ThermalMarginUsed;
use super::{ThermalPowerState, ThermalRangeExclusion, ThermalRangeMargin, ThermalRangeSide};
use crate::spec::{Check, quantity_text};
use sat_units::typed::Temperature;

/// 種類ごと・側ごとの判定（key は `range_high:<節点>:<種類>` と `range_low:<節点>:<種類>`。上限側、下限側の順）。
/// 予測の最低・最高は比較の余裕から戻す。電源の状態で外した部品があれば判定文に書く。温度の表示は sat-units に委譲する。
pub(crate) fn range_checks(
    node: &str,
    margins: &[ThermalRangeMargin],
    margin: &ThermalMarginUsed,
) -> Vec<Check> {
    let t = quantity_text::<Temperature>;
    let u = margin.uncertainty.value();
    let mut checks = Vec::new();
    for m in margins {
        let (lo, hi) = m.predicted();
        for side in [ThermalRangeSide::High, ThermalRangeSide::Low] {
            // 側ごとの予測値、不確かさを含めた値、当てた限界の値と呼び名
            let (word, predicted, with_u, limit_word, limit) = match side {
                ThermalRangeSide::High => ("maximum", hi, hi + u, "upper", m.range.max.value()),
                ThermalRangeSide::Low => ("minimum", lo, lo - u, "lower", m.range.min.value()),
            };
            let value = if u > 0.0 {
                format!("{word} {}, with uncertainty {}", t(predicted), t(with_u))
            } else {
                format!("{word} {}", t(predicted))
            };
            checks.push(Check::new(
                &format!("{}:{node}:{}", side.key_prefix(), m.kind.key()),
                &format!("{node} {word} temperature ({})", m.kind.noun()),
                m.side_verdict(side, margin),
                format!(
                    "{value} ({} {limit_word} limit {}{})",
                    m.kind.noun(),
                    t(limit),
                    excluded_text(&m.excluded)
                ),
            ));
        }
    }
    checks
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
                ". Excludes {1} {2} in mode {0}",
                modes.join(", "),
                power.noun(),
                parts.join(", ")
            )
        })
        .collect()
}
