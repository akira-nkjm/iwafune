//! 解析のフロー図の表示（部品・条件と設定・解析のカードと、そのつながり）。カード・行・つながりはどれも service が
//! 作る（つながりは射影が読んだもの `fune_projection::ReadLog` と、解析が中で使う解析 `sat_analysis::catalog::uses` から）。
//! カードの置き方（位置）は Web が決める。
use fune_model::Target;
use sat_analysis::spec::{AnalysisKind, System, Verdict};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// フロー図の問い合わせ（`GET /flow/{system}?case=…`、`fune flow <系> --case …`）。省略時のケース解決は解析の実行と同じ。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct FlowQuery {
    pub case: Option<String>,
}

/// フロー図全体。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct FlowView {
    /// カード（部品 → 条件と設定 → 解析の順。同じ種類の中は表示の順）
    pub nodes: Vec<FlowNode>,
    /// 値の行 → 解析、解析 → 解析のつながり
    pub edges: Vec<FlowEdge>,
    /// 畳める部品のまとまり（同じ値を同じ解析が読む部品が多いとき。負荷など）
    pub folds: Vec<FlowFold>,
}

/// カードの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum FlowNodeKind {
    /// 部品（解析が読む属性だけを行にする）
    Part,
    /// 設定・条件のファイル（`conditions/orbit.toml`・`analysis/power.toml` など）
    Setting,
    /// 運用モード
    Modes,
    /// 解析ケース
    Case,
    /// 解析（行は主な出力）
    Analysis,
}

/// カード 1 枚。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FlowNode {
    /// 図の中で一意な ID（つながりの端に使う。Web は分解しない）
    pub id: String,
    pub kind: FlowNodeKind,
    /// 表示名（部品の表示名・設定のファイル・解析の題名）
    pub label: String,
    pub label_en: String,
    /// 補足（部品の ID と種別・解析の ID）
    pub sub: Option<String>,
    /// 色に使う系（部品の系・解析の系）
    pub system: Option<System>,
    /// 開き先の対象（部品・設定・モード・ケース）
    pub target: Option<Target>,
    pub rows: Vec<FlowRow>,
    /// 解析のカードだけ: 解析の ID（結果は同じページの解析の一覧から引く）
    pub analysis: Option<String>,
    /// 解析のカードだけ: 判定（判定が無ければ null）
    pub verdict: Option<Verdict>,
    /// 解析のカードだけ: 実行の状態
    pub status: Option<crate::dto::AnalysisRunStatus>,
    /// 解析のカードだけ: 定常か時系列か（解析の登録の `AnalysisInfo.kind`）
    pub analysis_kind: Option<AnalysisKind>,
    /// 解析のカードだけ: 解析の説明（解析の登録の `AnalysisInfo.description` / `description_en`）
    pub description: Option<String>,
    pub description_en: Option<String>,
    /// 部品をまとめたカードの部品（同じ役割で、同じ属性を同じ解析が読む部品。1 部品のカードは空）
    pub members: Vec<FlowMember>,
}

/// まとめたカードの部品 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FlowMember {
    pub id: String,
    pub label: String,
    pub label_en: String,
}

/// カードの行 1 つ（部品の属性・設定のキー・モード・解析の主な出力）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FlowRow {
    /// カードの中で一意な ID
    pub id: String,
    pub label: String,
    pub label_en: String,
    /// 値の表示文字列（数と量は `sat_units::format` で作ったもの。列挙の値は表示名、範囲は「〜」）。null は表示する値が無い
    /// （`set` で未設定かを見る）
    pub display: Option<String>,
    /// 英語の表示文字列（言語で変わる値だけ。列挙の表示名・範囲の「–」など。null なら `display` を使う）
    pub display_en: Option<String>,
    /// 値が書かれているか（false なら解析の既定値を使う、または足りない）
    pub set: bool,
    /// 書かれた値から求めた値か（軌道の周期など。解析が読んだ値ではないので、つながりを持たない）
    pub derived: bool,
    /// まとめたカードで、部品ごとに値が違い 1 つの表示にできないとき: 値を持つ部品の数（`display` は null）
    pub varies: Option<u32>,
    /// 解析の出力の行だけ: 判定
    pub verdict: Option<Verdict>,
    /// 値の対象（部品の属性・設定のキーなど）
    pub target: Option<Target>,
}

/// つながりの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum FlowEdgeKind {
    /// 値の行 → それを読む解析
    Input,
    /// 解析 → その結果を中で使う解析
    Result,
}

/// つながり 1 本。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FlowEdge {
    /// 図の中で一意な ID
    pub id: String,
    pub kind: FlowEdgeKind,
    /// 始点のカード
    pub source: String,
    /// 始点の行（値の行。解析 → 解析は null）
    pub source_row: Option<String>,
    /// 終点のカード（解析）
    pub target: String,
}

/// 畳める部品のまとまり。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FlowFold {
    /// 図の中で一意な ID（畳んだカードの ID に使う）
    pub id: String,
    /// まとまりのカード（大きい順。電力なら全状態の最大 × 員数の大きい順）
    pub nodes: Vec<String>,
}
