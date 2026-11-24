//! 軌道の設定（設計の `conditions/orbit.toml` とフェーズの `[orbit]`）の、選んで入れる欄の選択肢。
//! 表（設定の表・フェーズの上書きの表）は、API が返した選択肢を並べるだけで、欄の中身は見ない。
//! `source` は語彙（`OrbitSource`）の値、`maneuver` は日付つきの `[transfer]` を持つマヌーバ。

use fune_model::builtin::enum_label;
use fune_model::design::{Design, OrbitSource};
use fune_projection::Choice;

/// 軌道の欄 `key`（`source`・`maneuver`）の選択肢。選ぶ欄でなければ空（自由入力）。
pub(crate) fn orbit_choices(design: &Design, key: &str) -> Vec<Choice> {
    match key {
        "source" => OrbitSource::ALL
            .iter()
            .map(|s| {
                let label = enum_label("orbit_source", s.as_str());
                Choice {
                    value: s.as_str().to_owned(),
                    label: label.map_or_else(|| s.as_str().to_owned(), |l| l.label.clone()),
                    label_en: label.and_then(|l| l.label_en.clone()),
                }
            })
            .collect(),
        "maneuver" => transfer_maneuvers(design),
        _ => Vec::new(),
    }
}

/// 日付つきの遷移（`[transfer]`）を持つマヌーバ（巡航の軌道を取れるもの。実行の順）。
fn transfer_maneuvers(design: &Design) -> Vec<Choice> {
    design
        .ordered_maneuvers()
        .into_iter()
        .filter(|m| m.transfer.is_some())
        .map(|m| Choice {
            value: m.id.to_string(),
            label: m.label.clone().unwrap_or_else(|| m.id.to_string()),
            label_en: None,
        })
        .collect()
}
