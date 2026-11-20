//! 書き出す表・図・値のデータモデル。サービスが解析の結果から組み立て、`data.json` として書き出す。
//! ここは設計や解析の知識を持たない（表示用の文字列と SI 値を持つだけ）。

use serde::{Deserialize, Serialize};

/// `data.json` の形の版。形を変えたら上げる（Typst のモジュールが読む）。
pub const SCHEMA: u32 = 1;

/// 表・図の言語。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "ReportLang"))]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Ja,
    #[default]
    En,
}

/// 書き出す項目（6 つ）。綴りは `--item` と `data.json` の `item` で共通。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "ReportItem"))]
#[serde(rename_all = "kebab-case")]
pub enum Item {
    /// 質量表
    Mass,
    /// モードごとの電力表
    ModePower,
    /// 回線バジェットの入力表と結果表（回線ごと）
    LinkBudget,
    /// 温度の結果表
    ThermalResult,
    /// 温度の時間変化の図
    ThermalHistory,
    /// 発電・電池の状態の時間変化の図
    PowerHistory,
}

impl Item {
    /// すべての項目（書き出しの順）。
    pub const ALL: [Item; 6] = [
        Item::Mass,
        Item::ModePower,
        Item::LinkBudget,
        Item::ThermalResult,
        Item::ThermalHistory,
        Item::PowerHistory,
    ];

    /// 綴り（`mass`・`mode-power` など）。
    pub fn as_str(self) -> &'static str {
        match self {
            Item::Mass => "mass",
            Item::ModePower => "mode-power",
            Item::LinkBudget => "link-budget",
            Item::ThermalResult => "thermal-result",
            Item::ThermalHistory => "thermal-history",
            Item::PowerHistory => "power-history",
        }
    }

    /// 綴りから読む。
    pub fn parse(text: &str) -> Option<Item> {
        Item::ALL.into_iter().find(|i| i.as_str() == text)
    }

    /// 解析ケースごとに繰り返す項目か（質量・モードの電力・回線はケースによらない）。
    pub fn per_case(self) -> bool {
        matches!(
            self,
            Item::ThermalResult | Item::ThermalHistory | Item::PowerHistory
        )
    }
}

/// 列の揃え。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    Left,
    Right,
    Center,
}

/// 行の種類（Typst の側で罫線・太字を変える）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RowStyle {
    /// ふつうの行
    Body,
    /// 見出し行（先頭のセルだけを全列にまたがせる。系の名前など）
    Group,
    /// 小計
    Subtotal,
    /// 合計
    Total,
    /// 余裕の行
    Margin,
}

/// 表のセル。`text` は整形済みの文字列（単位は列の見出しにある）、`si` は SI の値（数でないセルは無い）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub si: Option<f64>,
    /// 値に添える短い説明（機器の状態など。表では値の下に小さく出す）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl Cell {
    /// 文字だけのセル。
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            si: None,
            detail: None,
        }
    }

    /// 数のセル。
    pub fn number(text: impl Into<String>, si: f64) -> Self {
        Self {
            text: text.into(),
            si: Some(si),
            detail: None,
        }
    }

    /// 空のセル。
    pub fn empty() -> Self {
        Self::text("")
    }
}

/// 表の列。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub header: String,
    /// 見出しに添える単位（無ければ省く）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub align: Align,
}

/// 表の行。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub style: RowStyle,
    pub cells: Vec<Cell>,
}

/// 表 1 つ（`#mass-table()` など 1 回の呼び出しで 1 つ）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub item: Item,
    /// 解析ケースの ID（ケースによる項目だけ。ケースを指定しなかったときも、使われた既定のケースの ID）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case: Option<String>,
    /// 回線バジェットの回線（`<名前>@<地上局>`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// 同じ項目の中の種類（回線バジェットの `input` と `result`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// 表の上に置く題
    pub caption: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    /// 表の下の注記（マージンの扱い・有効数字・モデルの revision）
    pub notes: Vec<String>,
    /// 数えられなかったときの理由（あれば表は空）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<String>,
}

/// 図の中の 1 本の系列の要約（`data.json` に値の全部は入れない。図は SVG）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeriesSummary {
    pub label: String,
    pub points: usize,
    pub min: f64,
    pub max: f64,
}

/// 図の 1 段の要約。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelSummary {
    /// 縦軸の題（単位つき）
    pub y_label: String,
    pub series: Vec<SeriesSummary>,
}

/// 図 1 つ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Figure {
    pub item: Item,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case: Option<String>,
    /// 図の下に置く題
    pub caption: String,
    /// SVG のパス（`data.json` からの相対。数えられなければ無い）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub x_label: String,
    pub panels: Vec<PanelSummary>,
    /// 影をつけた区間の数（蝕）
    pub shaded: usize,
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<String>,
}

/// 文章から 1 つの値を引くための項目（Typst の `#value("…")`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueEntry {
    /// 構造のキー（例: `part.RW_X.mass`・`kpi.thermal.steady.max_temperature`・`budget.mass.total`）
    pub key: String,
    pub label: String,
    /// 単位つきの整形済み文字列
    pub text: String,
    /// 単位を除いた整形済みの数値（本文で単位を別に書くとき）
    pub number: String,
    pub unit: String,
    pub si: f64,
}

/// `data.json` の全体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportData {
    pub schema: u32,
    /// 設計の revision（古い表の見分けに使う）
    pub revision: String,
    pub project: String,
    pub lang: Lang,
    /// 書き出した項目
    pub items: Vec<Item>,
    /// 書き出した解析ケース（既定のケースを使ったときは、その ID）
    pub cases: Vec<String>,
    pub tables: Vec<Table>,
    pub figures: Vec<Figure>,
    pub values: Vec<ValueEntry>,
}
