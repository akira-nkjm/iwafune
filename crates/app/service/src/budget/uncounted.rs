//! バジェットで数えなかった部品（値が無い部品）を並べる（`BudgetView.uncounted`）。
//! 合計に足さないだけで 0 とはみなさない。名前を結果に出して、未入力で合計が小さく見えるのを防ぐ。

use crate::dto::BudgetUncounted;
use crate::workspace::Snapshot;
use sat_analysis::power::mode_load::ModeLoadOutput;

/// 質量: `mass` を持たない部品。
pub(super) fn mass(s: &Snapshot) -> Vec<BudgetUncounted> {
    s.resolved
        .parts
        .values()
        .filter(|p| p.scalar("mass").is_none())
        .map(|p| BudgetUncounted {
            part: p.id.clone(),
            label: fune_projection::parts::label(p),
            modes: Vec::new(),
        })
        .collect()
}

/// 電力: モードごとに、割り当てが無い部品と、平均消費電力が出なかった部品（使う状態の電力が無い）。
/// 同じ部品は 1 件にまとめ、数えなかったモードを並べる。
pub(super) fn power(s: &Snapshot, out: &ModeLoadOutput) -> Vec<BudgetUncounted> {
    let mut list: Vec<BudgetUncounted> = Vec::new();
    let mut add =
        |part: &str, label: String, mode: &str| match list.iter_mut().find(|u| u.part == part) {
            Some(u) => u.modes.push(mode.to_owned()),
            None => list.push(BudgetUncounted {
                part: part.to_owned(),
                label,
                modes: vec![mode.to_owned()],
            }),
        };
    for mode in &out.modes {
        for p in mode.parts.iter().filter(|p| p.mean.is_none()) {
            add(&p.part, p.label.clone(), &mode.id);
        }
        for id in &mode.unassigned {
            let label = s
                .resolved
                .parts
                .get(id.as_str())
                .map_or_else(|| id.clone(), fune_projection::parts::label);
            add(id, label, &mode.id);
        }
    }
    list.sort_by(|a, b| a.part.cmp(&b.part));
    list
}
