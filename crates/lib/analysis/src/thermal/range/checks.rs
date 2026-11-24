//! 温度範囲の比較の判定文（key `range_high:<節点>:<種類>` と `range_low:<節点>:<種類>`）。比較と判定は親の `thermal::range`、ここは文を作るだけ。

use super::ThermalMarginUsed;
use super::{ThermalPowerState, ThermalRangeExclusion, ThermalRangeMargin, ThermalRangeSide};
use crate::spec::{Check, quantity_text};
use crate::thermal::cell::ThermalCell;
use sat_units::typed::Temperature;
use std::collections::BTreeSet;

/// 判定する節点 1 つ（板の升なら `cell` を持つ）の範囲との比較。
pub(crate) struct NodeJudged<'a> {
    pub id: &'a str,
    pub cell: Option<&'a ThermalCell>,
    pub ranges: &'a [ThermalRangeMargin],
}

/// 節点の並びの判定。板の升の節点は **部品の名前で 1 つにまとめ**、上限の側は最も高くなった升、下限の側は最も低くなった升
/// （種類ごとに、その側の余裕が最も小さい升）で判定して、判定文に升を書く（key は `range_high:<部品>:<種類>`）。
/// 板でない節点はそのまま `range_checks`。順は最初に現れた位置。
pub(crate) fn node_checks<'a>(
    nodes: impl Iterator<Item = NodeJudged<'a>>,
    margin: &ThermalMarginUsed,
) -> Vec<Check> {
    let nodes: Vec<NodeJudged<'a>> = nodes.collect();
    let mut done: BTreeSet<&str> = BTreeSet::new();
    let mut checks = Vec::new();
    for n in &nodes {
        let Some(cell) = n.cell else {
            checks.extend(range_checks(n.id, n.ranges, margin));
            continue;
        };
        if !done.insert(&cell.part) {
            continue;
        }
        let group: Vec<&NodeJudged> = nodes
            .iter()
            .filter(|m| m.cell.is_some_and(|c| c.part == cell.part))
            .collect();
        for kind in group[0].ranges.iter().map(|m| m.kind) {
            let of_kind =
                |g: &&NodeJudged<'a>| g.ranges.iter().find(|m| m.kind == kind).map(|m| (g.id, m));
            let worst = |f: fn(&ThermalRangeMargin) -> f64| {
                group
                    .iter()
                    .filter_map(of_kind)
                    .min_by(|a, b| f(a.1).total_cmp(&f(b.1)))
            };
            let (Some((low_id, low)), Some((high_id, high))) = (
                worst(|m| m.margin_low.value()),
                worst(|m| m.margin_high.value()),
            ) else {
                continue;
            };
            let mut merged = high.clone();
            merged.margin_low = low.margin_low;
            checks.extend(range_checks_for(
                &cell.part,
                Some((high_id, low_id)),
                &[merged],
                margin,
            ));
        }
    }
    checks
}

/// 種類ごと・側ごとの判定（key は `range_high:<節点>:<種類>` と `range_low:<節点>:<種類>`。上限側、下限側の順）。
/// 予測の最低・最高は比較の余裕から戻す。電源の状態で外した部品があれば判定文に書く。温度の表示は sat-units に委譲する。
pub(crate) fn range_checks(
    node: &str,
    margins: &[ThermalRangeMargin],
    margin: &ThermalMarginUsed,
) -> Vec<Check> {
    range_checks_for(node, None, margins, margin)
}

/// `range_checks`。`cells` があれば、板の部品 `node` を (最も高い升, 最も低い升) で判定したもので、判定文に升を書く。
fn range_checks_for(
    node: &str,
    cells: Option<(&str, &str)>,
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
            let cell_text = cells.map_or_else(String::new, |(hot, cold)| match side {
                ThermalRangeSide::High => format!(", hottest cell {hot}"),
                ThermalRangeSide::Low => format!(", coldest cell {cold}"),
            });
            checks.push(Check::new(
                &sat_checkkey::thermal_range(side.key_prefix(), node, m.kind.key()),
                &format!("{node} {word} temperature ({})", m.kind.noun()),
                m.side_verdict(side, margin),
                format!(
                    "{value} ({} {limit_word} limit {}{}{}{cell_text})",
                    m.kind.noun(),
                    t(limit),
                    required_text(m, side),
                    excluded_text(&m.excluded)
                ),
            ));
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
