//! 通信の回線の表示（`GET /design` の `links`）。回線を開いて直す・消すための一覧で、書き込みは `SetSetting`・`RemoveCommLink`・`RenameCommLink`。

use fune_model::design::LinkDirection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 通信の回線 1 本（`analysis/comm.toml` の `[link.<名前>]`。名前順）。読み込みに要る 3 つの欄だけ（ほかの値は設定の表）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct LinkView {
    pub id: String,
    pub direction: LinkDirection,
    /// 衛星側の送受信機（部品 ID）
    pub part: String,
    /// 地上局側の帯域
    pub band: String,
    /// 設定ファイル（ルートからの相対パス）
    pub file: String,
    /// この回線の設定（`SettingView.group`。`analysis.comm.link.<名前>`）
    pub group: String,
}
