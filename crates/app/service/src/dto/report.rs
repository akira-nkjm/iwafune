//! 設計書に入れる表と図の書き出しの型（`POST /report/export`、`fune report export`）。

use fune_report::{ReportFile, ReportItem, ReportLang};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 書き出しの依頼。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct ReportRequest {
    /// 書き出す項目（空なら 6 項目すべて）
    pub items: Vec<ReportItem>,
    /// 解析ケースの ID（`cases/<ID>.toml`）。ケースによる項目（温度の表と図・発電と電池の図）をケースごとに繰り返す。
    /// 空なら、解析の既定のケース（設計に 1 つも無ければケースなし）で 1 回だけ
    pub cases: Vec<String>,
    /// 見出し・題・注記の言語（既定は英語）
    pub lang: ReportLang,
}

/// 書き出した結果。ファイルは出力先からの相対パスと本文で、書き出すのは呼び出し側（CLI は `--out` に書き、HTTP は返すだけ）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ReportExport {
    /// 設計の revision（`data.json` と表の脚注にも入っている）
    pub revision: String,
    /// 書き出すファイル（`data.json`・`iwafune.typ`・`example.typ`・`figs/*.svg`。パスの順は固定）
    pub files: Vec<ReportFile>,
    /// 数えられなかった項目（表や図は「算出できません」の注記つきで書き出している）
    pub unavailable: Vec<ReportUnavailable>,
}

/// 数えられなかった項目 1 件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ReportUnavailable {
    pub item: ReportItem,
    /// ケースによる項目のケース（既定のケースのときはその ID）
    pub case: Option<String>,
    /// 理由（不足している入力など）
    pub reason: String,
}
