//! 設計ファイルの整形の入出力。
use serde::{Deserialize, Serialize};
use ts_rs::TS;
/// 整形の要求。check は書き込まず変更予定だけ返す。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct FormatDesignRequest {
    #[serde(default)]
    pub check: bool,
}
/// 変更が必要だったファイル（相対パス・パス順）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FormatDesignResult {
    pub changed: Vec<String>,
    pub check: bool,
}
