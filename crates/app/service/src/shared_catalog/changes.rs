//! 取得・更新の前後の共有の品の差分（追加・削除・値の変わった品）。値の表示文字列はカタログ品の表示（`query::catalog`）と同じ。

use crate::dto::{CatalogItemChange, CatalogItemChangeKind, CatalogValueChange};
use crate::query::catalog::item_view;
use fune_model::design::{CatalogItem, Design};
use std::collections::{BTreeMap, BTreeSet};

/// 前後の設計で、共有カタログの品の変わったもの（ID 順）。
pub(super) fn between(before: &Design, after: &Design) -> Vec<CatalogItemChange> {
    let shared = |d: &Design| -> BTreeSet<String> {
        d.catalog
            .keys()
            .filter(|id| d.shared_catalog_of(id).is_some())
            .cloned()
            .collect()
    };
    let ids: BTreeSet<String> = shared(before).union(&shared(after)).cloned().collect();
    ids.into_iter()
        .filter_map(|id| {
            let old = before.catalog.get(&id).map(|i| fields(before, i));
            let new = after.catalog.get(&id).map(|i| fields(after, i));
            let (change, values) = match (old, new) {
                (None, Some(_)) => (CatalogItemChangeKind::Added, Vec::new()),
                (Some(_), None) => (CatalogItemChangeKind::Removed, Vec::new()),
                (Some(a), Some(b)) => {
                    let values = diff(&a, &b);
                    if values.is_empty() {
                        return None;
                    }
                    (CatalogItemChangeKind::Changed, values)
                }
                (None, None) => return None,
            };
            Some(CatalogItemChange { id, change, values })
        })
        .collect()
}

/// 品の欄 → 表示文字列（フィールドは名前、属性の値は `attr.<属性>`、値の出典は `attr.<属性>.from`、ポートは `ports.<名前>`）。
fn fields(design: &Design, item: &CatalogItem) -> BTreeMap<String, String> {
    let view = item_view(design, item);
    let mut out = BTreeMap::new();
    let mut put = |key: String, value: Option<String>| {
        if let Some(v) = value {
            out.insert(key, v);
        }
    };
    put("kind".into(), Some(view.kind));
    put("label".into(), view.label);
    put("label_en".into(), view.label_en);
    put("manufacturer".into(), view.manufacturer);
    put("part_number".into(), view.part_number);
    put("maturity".into(), view.maturity.map(|m| m.to_string()));
    put("datasheet".into(), view.datasheet);
    put("source".into(), view.source);
    for (port, kind) in view.ports {
        put(format!("ports.{port}"), Some(kind.to_string()));
    }
    for (attr, value) in view.values {
        put(format!("attr.{attr}"), value.display);
    }
    for (attr, value) in &item.attr {
        let from = value.from.as_ref().map(|f| {
            [f.source.as_deref(), f.at.as_deref(), f.note.as_deref()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · ")
        });
        put(format!("attr.{attr}.from"), from.filter(|f| !f.is_empty()));
    }
    out
}

fn diff(a: &BTreeMap<String, String>, b: &BTreeMap<String, String>) -> Vec<CatalogValueChange> {
    let keys: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    keys.into_iter()
        .filter(|k| a.get(*k) != b.get(*k))
        .map(|k| CatalogValueChange {
            key: k.clone(),
            before: a.get(k).cloned(),
            after: b.get(k).cloned(),
        })
        .collect()
}
