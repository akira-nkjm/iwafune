//! 要求の表示（`GET /requirements`・`GET /design` の `requirements`・`fune requirement ls`）、`fune requirement add|set` の引数
//! （`CommandText::RequirementAdd`・`RequirementSet`。読み方は `edit::requirement`）、要求適合表
//! （`GET /requirements/compliance`・`fune requirement compliance`。作り方は `requirements`）。

use super::QuantityView;
use fune_model::design::{ConstraintKind, Quantification, Verification};
use sat_analysis::spec::Verdict;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// `fune requirement add|set <ID> --statement … --quantity Mass --kind at_most --value "12 kg" …` の引数（文字列のまま）。
/// 書かなかった旗（省略・null・空の文字列）は書かない。量（`"12 kg"`）・真偽・ケースの列を読むのは service。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(optional_fields = nullable)]
pub struct RequirementText {
    pub id: String,
    /// 要求の文
    pub statement: String,
    /// 英語の文
    pub statement_en: Option<String>,
    /// 短い名前
    pub label: Option<String>,
    /// 制約する量の次元名（`Mass`・`Power`・`Temperature`）
    pub quantity: String,
    /// 出典の ID
    pub source: Option<String>,
    /// `at_most` / `at_least` / `equal` / `range`
    pub kind: String,
    /// 境界の値（`"12 kg"`。`at_most`・`at_least`・`equal`）
    pub value: Option<String>,
    /// 許容の幅（`"0.5 K"`。`equal` だけ）
    pub tolerance: Option<String>,
    /// 範囲の下限（`range` だけ）
    pub min: Option<String>,
    /// 範囲の上限（`range` だけ）
    pub max: Option<String>,
    /// すべてのケースで成り立つ（`"true"` / `"false"`）
    pub all_cases: Option<String>,
    /// 成り立つケース（カンマ区切り。`"hot_imaging,cold_safe"`）
    pub cases: Option<String>,
    /// 確かめる解析の ID
    pub analysis: Option<String>,
    /// その解析の判定のキー
    pub check: Option<String>,
    /// 確かめるバジェット（`mass` / `power` / `data`）
    pub budget: Option<String>,
}

/// 制約の表示（向きと、値を整形した文）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ConstraintView {
    pub kind: ConstraintKind,
    /// 例: `"≤ 12.0 kg"`・`"= 20.0 degC ± 2.0 K"`・`"0.0 degC … 40.0 degC"`（書かれた単位のまま）
    pub text: String,
}

/// 要求 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct RequirementView {
    pub id: String,
    /// 表示名（無ければ ID）
    pub label: String,
    pub statement: String,
    pub statement_en: Option<String>,
    /// 制約する量の次元名
    pub quantity: String,
    pub source: Option<String>,
    pub constraint: ConstraintView,
    pub quantify: Quantification,
    pub verify: Verification,
    /// 編集の欄の初期値（`fune requirement set` に打つのと同じ文字列）
    pub text: RequirementText,
    /// 欄の名前（`RequirementText` の欄）→ 対象のパス（`requirements.R_MASS.verify.check`。診断の `DiagnosticView.path` と同じ）
    pub field_paths: BTreeMap<String, String>,
    pub file: String,
}

/// 確かめられなかった理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum NotVerifiedReason {
    /// 確かめる先が書かれていない
    NoLink,
    /// 解析・バジェットの入力が足りない（`detail` に不足の入力のパス。バジェットの上限が無いときはその旨）
    MissingInputs,
    /// 解析が失敗した（`detail` に理由）
    Failed,
    /// 解析は動いたが、その判定のキーの判定が出なかった（`detail` に出た判定のキー）
    KeyNotProduced,
    /// 確かめる先の解析かバジェットが無い、またはケースが無い（`detail` に名前）
    UnknownTarget,
}

/// 1 つのケースでの確かめの結果。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ComplianceCase {
    /// 評価したケースの ID（ケースを指定せず解析の既定で評価したときは、解析が選んだケース。無ければ null）
    pub case: Option<String>,
    /// 判定（確かめられなければ null）
    pub verdict: Option<Verdict>,
    /// 確かめられなかった理由（判定があれば null）
    pub reason: Option<NotVerifiedReason>,
    /// 理由の補足（英語）
    pub detail: Option<String>,
    /// 最も悪い判定の文（解析の判定の `message`）
    pub message: Option<String>,
    /// 英語の文（語彙から引いた判定だけ）
    pub message_en: Option<String>,
    /// 評価した値（バジェットの合計。解析の判定は値を持たないので null）
    pub value: Option<QuantityView>,
}

/// 適合表の 1 行（要求 1 つ）。判定は確かめたケースのうち、違反 > 確かめられない > 警告 > 良の順に最も悪いもの。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ComplianceRow {
    pub id: String,
    pub label: String,
    pub statement: String,
    pub statement_en: Option<String>,
    pub quantity: String,
    pub constraint: ConstraintView,
    pub source: Option<String>,
    pub verify: Verification,
    /// 判定（確かめられなければ null）
    pub verdict: Option<Verdict>,
    /// 確かめられなかった理由（`verdict` があれば null）
    pub reason: Option<NotVerifiedReason>,
    pub detail: Option<String>,
    /// 行の判定を決めたケースの文
    pub message: Option<String>,
    pub message_en: Option<String>,
    /// 行の判定を決めたケースの評価した値
    pub value: Option<QuantityView>,
    /// ケースごとの結果（量化の順）
    pub cases: Vec<ComplianceCase>,
}

/// 適合表の件数（判定の数え方は service だけが持つ）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ComplianceSummary {
    pub total: u32,
    pub ok: u32,
    pub warning: u32,
    pub violation: u32,
    pub not_verified: u32,
}

/// 要求適合表（`GET /requirements/compliance`）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ComplianceReport {
    pub rows: Vec<ComplianceRow>,
    pub summary: ComplianceSummary,
}
