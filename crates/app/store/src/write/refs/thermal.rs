//! 熱の節点の名前を書いた設定の書き換え。既定の 1 部品 1 節点では部品の ID が節点の名前なので、部品の名前の変更が
//! 明示の結合（`[conductive.<節点>]`・`[radiative.<節点>]` のキーと値の表のキー）と発熱の場所（`[heat_node]` の値）の節点の名前に及ぶ。

use super::super::Edits;
use super::super::doc::{rename_key, table_opt};
use crate::error::StoreError;
use crate::layout;
use fune_model::design::AnalysisSystem;
use toml_edit::{Item, Value};

/// 節点の名前 `from` を `to` に書き換える（結合の表の両端と `[heat_node]` の値）。
pub(super) fn rename_node_name(edits: &mut Edits, from: &str, to: &str) -> Result<(), StoreError> {
    edits.edit(&layout::analysis_file(AnalysisSystem::Thermal), |doc| {
        for kind in ["conductive", "radiative"] {
            let Some(table) = table_opt(doc, &[kind]) else {
                continue;
            };
            rename_key(table, from, to);
            for (_, row) in table.iter_mut() {
                if let Some(row) = row.as_table_mut() {
                    rename_key(row, from, to);
                }
            }
        }
        if let Some(table) = table_opt(doc, &["heat_node"]) {
            for (_, item) in table.iter_mut() {
                if let Item::Value(v) = item
                    && v.as_str() == Some(from)
                {
                    let decor = v.decor().clone();
                    *v = Value::from(to);
                    *v.decor_mut() = decor;
                }
            }
        }
        Ok(())
    })
}
