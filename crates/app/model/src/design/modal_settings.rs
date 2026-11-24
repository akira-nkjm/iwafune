//! 粗い有限要素の固有値解析（`structure.modal_fem`）の設定（`analysis/structure.toml` の `[modal]`）。
//! serde の形がそのままファイルの形。値はすべて任意で、既定値は解析が持つ。
//!
//! 設定のパスは `analysis.structure.modal.mesh_size`・`analysis.structure.modal.joint_tolerance`・
//! `analysis.structure.modal.tie.<部品>`（`SetSetting` でそのまま編集できる）。

use crate::value::Quantity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// 粗い有限要素の固有値解析の設定。書かなければ解析の既定。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ModalSettings {
    /// 要素の大きさ（既定は最短の辺の 1/4 と 25 mm の小さいほう）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh_size: Option<Quantity>,
    /// パネルの辺・面の位置を一致とみなす許容差（既定 1 mm）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub joint_tolerance: Option<Quantity>,
    /// 部品 → つなぐパネル（既定は取付先のパネル、無ければ最も近いパネル）。
    /// 部品とパネルの名前の変更には追従しない（解析のたびに指す先を確かめ、無ければ足りない入力にする）
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tie: BTreeMap<String, String>,
}
