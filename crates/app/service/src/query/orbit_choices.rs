//! 軌道の設定（設計の `conditions/orbit.toml` とフェーズの `[orbit]`）の、選んで入れる欄の選択肢。
//! 表（設定の表・フェーズの上書きの表）は、API が返した選択肢を並べるだけで、欄の中身は見ない。
//! `source` は語彙（`OrbitSource`）の値、`maneuver` は日付つきの `[transfer]` を持つマヌーバ、
//! ラグランジュ点の軌道の `primary`・`secondary`・`point`・`branch` は語彙の値。

use fune_model::builtin::enum_label;
use fune_model::design::{
    CentralBodyKind, Design, HaloBranchKind, LibrationPointKind, OrbitSource,
};
use fune_model::enum_text::EnumText;
use fune_projection::Choice;

/// 軌道の欄 `key` の選択肢。選ぶ欄は `source`（語彙）・`maneuver`（日付つきの遷移を持つマヌーバ）・
/// ラグランジュ点の軌道の `primary`・`secondary`（天体の語彙）・`point`・`branch`（語彙）。ほかは空（自由入力）。
pub(crate) fn orbit_choices(design: &Design, key: &str) -> Vec<Choice> {
    match key {
        "source" => vocabulary::<OrbitSource>("orbit_source"),
        "primary" | "secondary" => vocabulary::<CentralBodyKind>("central_body"),
        "point" => vocabulary::<LibrationPointKind>("libration_point"),
        "branch" => vocabulary::<HaloBranchKind>("halo_branch"),
        "maneuver" => transfer_maneuvers(design),
        _ => Vec::new(),
    }
}

/// 語彙（列挙 `T`、表示名は labels.toml の `ty`）の値を、宣言の順に選択肢にする。
fn vocabulary<T: EnumText>(ty: &str) -> Vec<Choice> {
    T::ALL
        .iter()
        .map(|v| {
            let label = enum_label(ty, v.as_str());
            Choice {
                value: v.as_str().to_owned(),
                label: label.map_or_else(|| v.as_str().to_owned(), |l| l.label.clone()),
                label_en: label.and_then(|l| l.label_en.clone()),
            }
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// ラグランジュ点の軌道の欄は語彙から選ぶ（天体は中心天体の語彙、点と枝は専用の語彙）。ほかの欄は自由入力。
    #[test]
    fn libration_keys_offer_their_vocabularies() {
        let design = Design::default();
        let values = |key: &str| -> Vec<String> {
            orbit_choices(&design, key)
                .into_iter()
                .map(|c| c.value)
                .collect()
        };
        assert_eq!(values("point"), ["l1", "l2", "l3", "l4", "l5"]);
        assert_eq!(values("branch"), ["north", "south"]);
        assert_eq!(values("primary").len(), 10);
        assert_eq!(values("secondary"), values("primary"));
        let l2 = orbit_choices(&design, "point")
            .into_iter()
            .find(|c| c.value == "l2")
            .expect("l2");
        assert!(l2.label_en.is_some_and(|l| l.starts_with("L2")));
        assert!(values("amplitude_z").is_empty() && values("epoch").is_empty());
    }
}
