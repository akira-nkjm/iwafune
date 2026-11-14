//! カードの並びと、畳める部品のまとまり。1 つの属性だけを同じ解析が読む部品が `FOLD_MIN` 個以上あれば 1 つのまとまりにし
//! （電源の負荷の電力など）、その値の大きい順（状態ごとの値は全状態の最大。× 員数）に並べる。ほかの部品は ID 順で前に置く。

use super::cards::CardKey;
use super::group::{Signature, signature};
use crate::dto::FlowFold;
use fune_model::Target;
use fune_model::resolve::{Resolved, SiValue};
use std::collections::{BTreeMap, BTreeSet};

/// まとまりにする部品の数の下限。
const FOLD_MIN: usize = 5;

/// カードの並び（部品 → 設定 → モード → ケース）と、畳める部品のまとまり。
pub(super) fn order(
    resolved: &Resolved,
    cards: &BTreeMap<CardKey, Vec<(String, Target)>>,
    readers: &BTreeMap<Target, BTreeSet<String>>,
) -> (Vec<CardKey>, Vec<FlowFold>) {
    let mut groups: BTreeMap<Signature, Vec<&CardKey>> = BTreeMap::new();
    for (key, rows) in cards {
        if matches!(key, CardKey::Part(_)) {
            groups
                .entry(signature(rows, readers))
                .or_default()
                .push(key);
        }
    }
    let mut folded: Vec<Vec<&CardKey>> = Vec::new();
    let mut alone: BTreeSet<&CardKey> = BTreeSet::new();
    for (sig, mut members) in groups {
        if sig.0.len() == 1 && members.len() >= FOLD_MIN {
            members.sort_by(|a, b| {
                weight(resolved, b, cards)
                    .total_cmp(&weight(resolved, a, cards))
                    .then_with(|| a.cmp(b))
            });
            folded.push(members);
        } else {
            alone.extend(members);
        }
    }
    let mut order: Vec<CardKey> = alone.into_iter().cloned().collect();
    let mut folds = Vec::new();
    for (i, members) in folded.into_iter().enumerate() {
        folds.push(FlowFold {
            id: format!("fold:{i}"),
            nodes: members.iter().map(|k| k.node_id()).collect(),
        });
        order.extend(members.into_iter().cloned());
    }
    order.extend(
        cards
            .keys()
            .filter(|k| !matches!(k, CardKey::Part(_)))
            .cloned(),
    );
    (order, folds)
}

/// 並べる重み: 読む属性（1 つ）の値（状態ごとの値は全状態の最大）× 員数の、カードの部品の合計。
fn weight(
    resolved: &Resolved,
    key: &CardKey,
    cards: &BTreeMap<CardKey, Vec<(String, Target)>>,
) -> f64 {
    let CardKey::Part(ids) = key else {
        return 0.0;
    };
    let attrs: Vec<&str> = cards
        .get(key)
        .into_iter()
        .flatten()
        .map(|(a, _)| a.as_str())
        .collect();
    ids.iter()
        .filter_map(|id| resolved.parts.get(id.as_str()))
        .map(|p| {
            let largest = attrs
                .iter()
                .filter_map(|attr| match p.value(attr)? {
                    SiValue::Scalar(s) => Some(s.value),
                    SiValue::ByState(m) => m.values().map(|v| v.value).reduce(f64::max),
                    _ => None,
                })
                .fold(0.0, f64::max);
            fune_projection::parts::extensive(p, largest)
        })
        .sum()
}
