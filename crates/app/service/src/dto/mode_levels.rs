//! モードの表の「状態・電力」の列の出し方の型（決め方は `mode_systems::mode_levels`）。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// モードの表の部品ごとの列（見出しの左の列）が何を出すか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ModeLevels {
    /// 状態ごとの消費電力（`off 0 W / on 3 W`）
    PowerLevels,
    /// 機器の状態の名前だけ
    States,
}

/// 系ごと（`system` が null なら設計のモードのページ）の列の出し方。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ModeLevelsView {
    pub system: Option<sat_analysis::spec::System>,
    pub levels: ModeLevels,
}
