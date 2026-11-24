//! 要求（`requirements/<ID>.toml`。docs/model.md §7.9）。平らな一覧で、親子・配分は持たない。
//!
//! 要求は「何の量を・どんな制約で・どのケースで・どの出典から・何で確かめるか」を持つ。確かめる先（[`Verification`]）は
//! 解析の判定（解析 ID + 判定のキー）かバジェットの判定で、判定そのものは解析とバジェットが出す
//! （適合表は `fune_service::requirements`）。serde の形がそのままファイルの形（`id` はファイル名から決まるので書かない）。

use super::defs::BudgetKind;
use super::source::Source;
use super::system::Case;
use crate::enum_text::enum_text;
use crate::id::Id;
use crate::value::Quantity;
pub use sat_checkkey::EntityKind;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

enum_text! {
    /// 制約の向き。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
    pub enum ConstraintKind: "constraint kind" {
        /// 値以下（`value`）
        AtMost = "at_most",
        /// 値以上（`value`）
        AtLeast = "at_least",
        /// 値に等しい（`value`。`tolerance` があれば ± の幅）
        Equal = "equal",
        /// 範囲（`min` 以上 `max` 以下）
        Range = "range",
    }
}

enum_text! {
    /// 要求が解析の入力として与える値（判定の基準ではなく、解析が読む入力。`Requirement::input`）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
    pub enum RequirementInput: "requirement input" {
        /// ミッション期間（「ミッションは少なくとも…続く」。`at_least` の値。季節の解析・高度低下・電源の逆算などが読む）
        MissionDuration = "mission_duration",
    }
}

impl RequirementInput {
    /// 入力の値の次元名（要求の `quantity`）。
    pub fn quantity(self) -> &'static str {
        match self {
            Self::MissionDuration => "Time",
        }
    }

    /// 入力の値を書く制約の向き。
    pub fn kind(self) -> ConstraintKind {
        match self {
            Self::MissionDuration => ConstraintKind::AtLeast,
        }
    }
}

/// 制約（向きと値）。向きごとに使う欄が決まっている（`check` の `requirement.constraint_*`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Constraint {
    pub kind: ConstraintKind,
    /// 境界の値（`at_most`・`at_least`・`equal`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Quantity>,
    /// 許容の幅（`equal` だけ。`value ± tolerance`。同じ次元で 0 以上）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<Quantity>,
    /// 範囲の下限（`range` だけ）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<Quantity>,
    /// 範囲の上限（`range` だけ）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<Quantity>,
}

impl Constraint {
    /// 書かれている量とそのキー（宣言の順）。次元の検査と一覧の表示が使う。
    pub fn quantities(&self) -> Vec<(&'static str, &Quantity)> {
        [
            ("value", &self.value),
            ("tolerance", &self.tolerance),
            ("min", &self.min),
            ("max", &self.max),
        ]
        .into_iter()
        .filter_map(|(k, q)| q.as_ref().map(|q| (k, q)))
        .collect()
    }

    /// この向きで書いてよい欄（必須のものと任意のもの）のキー。
    pub fn allowed_keys(kind: ConstraintKind) -> &'static [&'static str] {
        match kind {
            ConstraintKind::AtMost | ConstraintKind::AtLeast => &["value"],
            ConstraintKind::Equal => &["value", "tolerance"],
            ConstraintKind::Range => &["min", "max"],
        }
    }

    /// この向きで必ず書く欄のキー。
    pub fn required_keys(kind: ConstraintKind) -> &'static [&'static str] {
        match kind {
            ConstraintKind::AtMost | ConstraintKind::AtLeast | ConstraintKind::Equal => &["value"],
            ConstraintKind::Range => &["min", "max"],
        }
    }
}

/// 量化（どのケースで成り立つ要求か）。書かなければ、確かめる解析の既定のケース（`cases/` が無ければ解析の既定の条件）で見る。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Quantification {
    /// `cases/` のすべてのケースで成り立つ
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub all_cases: bool,
    /// 成り立つケース（`all_cases` と一緒には書かない）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cases: Vec<Id<Case>>,
}

impl Quantification {
    /// 何も書かれていないか（ファイルに `[quantify]` を書かない）。
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// 確かめる先。「解析 + 判定のキー」か「バジェット」のどちらか 1 つ。
/// 判定のキーは解析が実行のたびに作る文字列（例: `dod`・`margin:s_down@TOKYO_GS`）なので、実在は適合表を作るときに確かめる。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Verification {
    /// 解析の ID（`power.energy_balance`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analysis: Option<String>,
    /// その解析の判定（`checks`）のキー。同じキーの判定が複数あれば（モードごとなど）最も悪いものを使う
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<String>,
    /// バジェットの判定（`budget/allocation.toml` の上限との比較）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<BudgetKind>,
}

impl Verification {
    /// 判定のキー `check` が `kind` のエンティティ `name` を名指ししているか（部品・通信の回線・地上局・熱の節点。
    /// 名前は欄ごとに完全一致で見る: 局 `tokyo` は `…@tokyo2` に当たらない）。
    ///
    /// キーの書式は解析のものなので、読みも書き換えも `sat_checkkey` が持つ（解析のキーの組み立てと同じ表）。
    /// 解析の出力するキーとの一致は fune-projection のテストが確かめる。
    pub fn check_names(&self, kind: EntityKind, name: &str) -> bool {
        self.check
            .as_deref()
            .is_some_and(|key| sat_checkkey::names(key, kind).contains(&name))
    }

    /// `kind` のエンティティ `from` を名指す判定のキーを、`to` を名指すものに替えたキー（名指していなければ None）。
    pub fn check_with_renamed(&self, kind: EntityKind, from: &str, to: &str) -> Option<String> {
        sat_checkkey::renamed(self.check.as_deref()?, kind, from, to)
    }

    /// 何も書かれていないか（ファイルに `[verify]` を書かない）。
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// 要求（`requirements/<ID>.toml`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    #[serde(default)]
    pub id: Id<Requirement>,
    /// 短い名前
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// 要求の文（「〜しなければならない」）
    pub statement: String,
    /// 英語の文（GUI の英語表示）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_en: Option<String>,
    /// 制約する量の次元名（`Mass`・`Power`・`Temperature`。制約の値の単位はこの次元でなければならない）
    pub quantity: String,
    /// 出典（`sources/<ID>.toml`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Id<Source>>,
    /// 解析の入力として与える値（`mission_duration`。制約の値が入力の値になる。設計全体で同じ入力を与える要求は 1 つ）。
    /// 判定の基準を与える `verify` とは別の欄（`verify.check` を入力の指定には使わない）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<RequirementInput>,
    pub constraint: Constraint,
    #[serde(default, skip_serializing_if = "Quantification::is_empty")]
    pub quantify: Quantification,
    #[serde(default, skip_serializing_if = "Verification::is_empty")]
    pub verify: Verification,
}

impl ConstraintKind {
    /// 比較の記号（一覧と CLI の表の短い表示）。
    pub fn symbol(self) -> &'static str {
        match self {
            Self::AtMost => "≤",
            Self::AtLeast => "≥",
            Self::Equal => "=",
            Self::Range => "∈",
        }
    }
}

/// 解析の ID の一覧（`sat_analysis::catalog::catalog()` と同じ集合。fune-model は sat-analysis に依存できないので、
/// 写しを持ち、一致は fune-projection のテスト `requirement_analysis_ids` が確かめる）。
pub const ANALYSIS_IDS: &[&str] = &[
    "aocs.rw_unloading",
    "aocs.unload_time",
    "aocs.sensor_fov",
    "aocs.slew_time",
    "cdh.data_budget",
    "comm.link_budget",
    "comm.visibility",
    "comm.data_throughput",
    "orbit.eclipse_season",
    "orbit.decay",
    "orbit.geometry",
    "power.mode_load",
    "power.solar_sunlit",
    "power.orbit_generation",
    "power.energy_balance",
    "power.sizing",
    "power.thrust_balance",
    "propulsion.maneuvers",
    "propulsion.dv_budget",
    "propulsion.propellant",
    "propulsion.thruster_usage",
    "propulsion.orbit_history",
    "propulsion.transfer_check",
    "propulsion.launch_window",
    "propulsion.mass_properties",
    "structure.mass_properties",
    "structure.static_load",
    "structure.modal_estimate",
    "structure.modal_fem",
    "structure.launch_fit",
    "structure.interference",
    "thermal.steady",
    "thermal.transient",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_key_names_entities_by_whole_name() {
        let verify = |key: &str| Verification {
            check: Some(key.to_string()),
            ..Default::default()
        };
        assert!(verify("margin:s_down@tokyo").check_names(EntityKind::Station, "tokyo"));
        // 電力束密度の判定（`pfd:<回線>@<局>`）も同じ書式
        assert!(verify("pfd:s_down@tokyo").check_names(EntityKind::Station, "tokyo"));
        assert!(!verify("pfd:s_down@tokyo2").check_names(EntityKind::Station, "tokyo"));
        assert!(!verify("margin:s_down@xtokyo").check_names(EntityKind::Station, "tokyo"));
        assert!(!verify("dod").check_names(EntityKind::Station, "tokyo"));
        assert!(!Verification::default().check_names(EntityKind::Station, "tokyo"));
        assert!(verify("intrusion:STT").check_names(EntityKind::Part, "STT"));
        assert!(!verify("intrusion:STT").check_names(EntityKind::ThermalNode, "STT"));
        assert_eq!(
            verify("range_low:BAT:operating")
                .check_with_renamed(EntityKind::ThermalNode, "BAT", "BATT")
                .as_deref(),
            Some("range_low:BATT:operating")
        );
    }

    #[test]
    fn empty_tables_are_not_written() {
        let r = Requirement {
            id: Id::new("R1"),
            label: None,
            statement: "s".into(),
            statement_en: None,
            quantity: "Mass".into(),
            source: None,
            input: None,
            constraint: Constraint {
                kind: ConstraintKind::AtMost,
                value: Some(Quantity::new(12.0, "kg")),
                tolerance: None,
                min: None,
                max: None,
            },
            quantify: Quantification::default(),
            verify: Verification::default(),
        };
        let json = serde_json::to_value(&r).unwrap();
        assert!(json.get("quantify").is_none() && json.get("verify").is_none());
        assert_eq!(r.constraint.quantities().len(), 1);
    }

    #[test]
    fn constraint_keys_follow_the_kind() {
        assert_eq!(
            Constraint::required_keys(ConstraintKind::Range),
            ["min", "max"]
        );
        assert_eq!(
            Constraint::allowed_keys(ConstraintKind::Equal),
            ["value", "tolerance"]
        );
    }
}
