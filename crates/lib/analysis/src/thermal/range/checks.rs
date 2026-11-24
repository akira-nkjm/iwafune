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
            checks.push(
                Check::new(
                    &sat_checkkey::thermal_range(side.key_prefix(), node, m.kind.key()),
                    &format!("{node} {word} temperature ({})", m.kind.noun()),
                    m.side_verdict(side, margin),
                    format!(
                        "{value} ({} {limit_word} limit {}{}{})",
                        m.kind.noun(),
                        t(limit),
                        required_text(m, side),
                        excluded_text(&m.excluded)
                    ),
                )
                .with_margin(m.side_margin(side, margin)),
            );
        }
    }
    checks
}

/// 要求が限界に効いたかどうかの文（効いた: 「、要求 R で厳しくした」、部品の範囲より緩くて効かなかった: 「。要求 R（…）は部品の範囲より緩く、効かない」）。
/// 部品の範囲と同じ値の要求は書かない。
fn required_text(m: &ThermalRangeMargin, side: ThermalRangeSide) -> String {
    m.required
        .iter()
        .filter(|u| u.is_side(side))
        .map(|u| {
            if u.tightens {
                format!(", tightened by requirement {}", u.requirement)
            } else {
                format!(
                    ". Requirement {} ({}) is looser than the part range, so it has no effect",
                    u.requirement,
                    quantity_text::<Temperature>(u.limit.value())
                )
            }
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
                ". Excludes {1} {2} in mode {0}",
                modes.join(", "),
                power.noun(),
                parts.join(", ")
            )
        })
        .collect()
}
