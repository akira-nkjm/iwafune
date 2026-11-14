//! 部品をまとめる: 同じ役割（kind の role）で、同じ属性を同じ解析が読む部品が 2 つ以上あれば 1 枚のカードにする
//! （太陽電池パネル・リアクションホイールなど）。どの役割でも同じ規則。

use super::cards::CardKey;
use fune_model::Target;
use fune_model::builtin::Role;
use fune_model::resolve::Resolved;
use std::collections::{BTreeMap, BTreeSet};

/// カードの読まれ方（行の ID と、それを読む解析）。
pub(super) type Signature = (Vec<String>, BTreeSet<String>);

type Cards = BTreeMap<CardKey, Vec<(String, Target)>>;

/// 部品のカードの読まれ方。
pub(super) fn signature(
    rows: &[(String, Target)],
    readers: &BTreeMap<Target, BTreeSet<String>>,
) -> Signature {
    let ids = rows.iter().map(|(r, _)| r.clone()).collect();
    let by = rows
        .iter()
        .flat_map(|(_, t)| readers.get(t).into_iter().flatten().cloned())
        .collect();
    (ids, by)
}

/// 同じ役割で読まれ方の同じ部品のカードを 1 枚にまとめる（行は最初の部品のもの。読む解析は同じ）。
pub(super) fn group_parts(
    resolved: &Resolved,
    cards: Cards,
    readers: &BTreeMap<Target, BTreeSet<String>>,
) -> Cards {
    let mut groups: BTreeMap<(Role, Signature), Vec<String>> = BTreeMap::new();
    for (key, rows) in &cards {
        let CardKey::Part(ids) = key else { continue };
        let [id] = ids.as_slice() else { continue };
        if let Some(role) = resolved.parts.get(id.as_str()).and_then(|p| p.role) {
            groups
                .entry((role, signature(rows, readers)))
                .or_default()
                .push(id.clone());
        }
    }
    let mut out = cards;
    for members in groups.into_values().filter(|m| m.len() >= 2) {
        let mut rows = None;
        for id in &members {
            let taken = out.remove(&CardKey::Part(vec![id.clone()]));
            rows = rows.or(taken);
        }
        if let Some(rows) = rows {
            out.insert(CardKey::Part(members), rows);
        }
    }
    out
}
