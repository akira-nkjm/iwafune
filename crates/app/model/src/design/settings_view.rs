//! 型付きの設定（`conditions/`・`analysis/`・`budget/`）を、パラメーターのパス記法（`analysis.power.dod_max`）で読む。
//!
//! キーは serde の名前（= ファイルのキー）そのもの。`budget` は `mass`・`power`・`data`（`budget/allocation.toml`）と
//! `policy`（`budget/policy.toml`）に分ける（パス → ファイルの対応は `fune_store::layout::setting_location` が持つ）。

use super::Design;
use crate::path::{SettingPath, SettingScope};
use crate::value::{SettingValue, quantity_from_json, quaternion_from_json, vector_from_json};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as Json};
use ts_rs::TS;

/// 設定の葉 1 つ。`value` が None なら書かれていない（任意の値の欠け）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct SettingEntry {
    pub path: SettingPath,
    pub value: Option<SettingValue>,
}

impl Design {
    /// 設定を種類ごとに JSON にする（キーはパス記法の区切りと同じ）。
    pub fn settings_json(&self, scope: SettingScope) -> Json {
        let to = |v: Result<Json, serde_json::Error>| v.unwrap_or(Json::Null);
        match scope {
            SettingScope::Conditions => {
                // 未入力の期間と任意の季節刻み・高度低下の設定も設定の表に出す。値の既定値は付けない。
                let mut conditions = self.conditions.clone();
                let orbit = conditions.orbit.get_or_insert_default();
                orbit.eclipse_season.get_or_insert_default();
                orbit.decay.get_or_insert_default();
                let offered = orbit.source_kind().offered_keys();
                let mut v = to(serde_json::to_value(&conditions));
                // 軌道の出どころと、その出どころが使う欄（`OrbitSource::offered_keys`）は、未入力でも設定の表に欄を出す
                // （選んで・打って入れる。値の既定値は付けない）。
                if let Some(orbit) = v.get_mut("orbit").and_then(Json::as_object_mut) {
                    for key in offered {
                        orbit.entry(key).or_insert(Json::Null);
                    }
                }
                // 地上局の `id` はファイル名で決まるので設定の値ではない。
                if let Some(ground) = v.get_mut("ground").and_then(Json::as_object_mut) {
                    for station in ground.values_mut().filter_map(Json::as_object_mut) {
                        station.remove("id");
                    }
                }
                v
            }
            SettingScope::Analysis => {
                // 共通設定は未作成でも設定一覧に欄を出す。
                let mut analysis = self.analysis.clone();
                analysis.common.get_or_insert_default();
                to(serde_json::to_value(&analysis))
            }
            SettingScope::Budget => {
                let mut map = match to(serde_json::to_value(&self.budget.allocation)) {
                    Json::Object(m) => m,
                    _ => Map::new(),
                };
                map.insert(
                    "policy".into(),
                    to(serde_json::to_value(&self.budget.policy)),
                );
                Json::Object(map)
            }
        }
    }

    /// 1 つの設定の値（書かれていない・葉でなければ None）。
    pub fn setting(&self, path: &SettingPath) -> Option<SettingValue> {
        let mut v = self.settings_json(path.scope);
        for segment in &path.segments {
            v = match v {
                Json::Object(mut map) => map.remove(segment)?,
                Json::Array(mut items) => {
                    let i: usize = segment.parse().ok()?;
                    (i < items.len()).then(|| items.swap_remove(i))?
                }
                _ => return None,
            };
        }
        leaf(&v).flatten()
    }

    /// すべての設定の葉（種類 → キーの順）。配列の表は `analysis.power.sequence.0.mode` のように番号で区切る。
    pub fn setting_entries(&self) -> Vec<SettingEntry> {
        let mut out = Vec::new();
        for scope in [
            SettingScope::Conditions,
            SettingScope::Analysis,
            SettingScope::Budget,
        ] {
            let root = SettingPath {
                scope,
                segments: Vec::new(),
            };
            walk(&root, &self.settings_json(scope), &mut out);
        }
        out
    }
}

/// 型付きの設定 1 つ（`value` の JSON）の葉を、`root` を先頭にしたパスで並べる（フェーズの `[orbit]` を設計の軌道と同じ表の形で
/// 検査するときの入口。`setting_entries` と同じ歩き方）。
pub fn entries_under(root: &SettingPath, value: &Json) -> Vec<SettingEntry> {
    let mut out = Vec::new();
    walk(root, value, &mut out);
    out
}

/// 葉なら Some（中身が null なら Some(None)）。表なら None。
fn leaf(v: &Json) -> Option<Option<SettingValue>> {
    if let Some(q) = quantity_from_json(v) {
        return Some(Some(SettingValue::Quantity(q)));
    }
    if let Some(q) = quaternion_from_json(v) {
        return Some(Some(SettingValue::Quaternion(q)));
    }
    if let Some(vec) = vector_from_json(v) {
        return Some(Some(SettingValue::Vector(vec)));
    }
    Some(Some(match v {
        Json::Null => return Some(None),
        Json::Bool(b) => SettingValue::Bool(*b),
        Json::Number(n) => match (n.as_i64(), n.is_f64()) {
            (Some(i), false) => SettingValue::Integer(i),
            _ => SettingValue::Number(n.as_f64().unwrap_or_default()),
        },
        Json::String(s) => SettingValue::Text(s.clone()),
        Json::Array(items) if items.iter().all(Json::is_string) => SettingValue::List(
            items
                .iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect(),
        ),
        _ => return None,
    }))
}

fn walk(path: &SettingPath, v: &Json, out: &mut Vec<SettingEntry>) {
    let child = |key: String| {
        let mut p = path.clone();
        p.segments.push(key);
        p
    };
    if let Some(value) = leaf(v) {
        if !path.segments.is_empty() {
            out.push(SettingEntry {
                path: path.clone(),
                value,
            });
        }
        return;
    }
    match v {
        Json::Object(map) => map
            .iter()
            .for_each(|(k, c)| walk(&child(k.clone()), c, out)),
        Json::Array(items) => items
            .iter()
            .enumerate()
            .for_each(|(i, c)| walk(&child(i.to_string()), c, out)),
        _ => {}
    }
}
