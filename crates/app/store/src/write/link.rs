//! 通信の回線（`analysis/comm.toml` の `[link.<名前>]`）の作成と削除。回線は向き・部品・帯域が無いと読み込めないので、
//! 1 つずつ `SetSetting` で書くことはできず、最小の 3 つのキーをまとめて書く。ほかの値の書き換えは設定の書き込み（`entities::set_setting`）。
//! 回線の名前の変更は `refs::rename_link`。

use super::doc::table_opt;
use super::entities::set_setting;
use crate::error::StoreError;
use crate::layout;
use fune_model::design::{AnalysisSystem, LinkDirection};
use fune_model::value::SettingValue;
use toml_edit::Item;

/// 回線を作る（`direction`・`part`・`band` の順に書く。ファイルが無ければ作る）。名前の重複の検査は呼ぶ側（fune-ops）。
pub fn create_link(
    edits: &mut crate::Edits,
    id: &str,
    direction: LinkDirection,
    part: &str,
    band: &str,
) -> Result<(), StoreError> {
    let file = layout::analysis_file(AnalysisSystem::Comm);
    for (key, value) in [
        ("direction", direction.as_str()),
        ("part", part),
        ("band", band),
    ] {
        let keys = ["link", id, key].map(str::to_owned);
        set_setting(
            edits,
            &file,
            &keys,
            Some(&SettingValue::Text(value.to_owned())),
        )?;
    }
    Ok(())
}

/// 回線を消す。最後の回線なら `link` の表ごと。`data_downlink` がその回線を指していれば、それも外す（無指定は
/// 「名前順で最初の下りの回線」）。無ければ false。
pub fn remove_link(edits: &mut crate::Edits, id: &str) -> Result<bool, StoreError> {
    let file = layout::analysis_file(AnalysisSystem::Comm);
    edits.edit(&file, |doc| {
        let Some(links) = table_opt(doc, &["link"]) else {
            return Ok(false);
        };
        let removed = links.remove(id).is_some();
        if doc.get("data_downlink").and_then(Item::as_str) == Some(id) {
            doc.remove("data_downlink");
        }
        super::doc::drop_if_empty(doc, &["link"]);
        Ok(removed)
    })
}
