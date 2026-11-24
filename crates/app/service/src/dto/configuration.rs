//! 形態の表示（`GET /configurations`・`GET /design` の `configurations`・`fune configuration ls|show`）と、
//! `fune configuration add|set` に打った引数（`CommandText::ConfigurationAdd`・`ConfigurationSet`。読み方は `edit::configuration`）。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `fune configuration add|set <ID> --label … --description …` の引数（文字列のまま）。
/// 書かなかった旗（省略・null・空の文字列）は書かない。部品ごとの姿勢（`[pose.<部品>]`）はここには無い
/// （`fune part place --configuration`・`fune configuration unset-pose`。`set` は今のまま残す）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(optional_fields = nullable)]
pub struct ConfigurationText {
    pub id: String,
    pub label: Option<String>,
    pub description: Option<String>,
}

/// 形態 1 つ（ID 順）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ConfigurationView {
    pub id: String,
    /// 表示名（無ければ ID）
    pub label: String,
    pub description: Option<String>,
    /// 姿勢を上書きしている部品（ID 順。`[pose.<部品>]`）
    pub overridden: Vec<String>,
    /// 編集の欄の初期値（`fune configuration set` に打つのと同じ文字列）
    pub text: ConfigurationText,
    pub file: String,
}
