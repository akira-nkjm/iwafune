//! 承知済みの組（`analysis/structure.toml` の `[[acknowledged]]`）。箱で表すと必ず出てしまう所見（ヒンジの縁の重なりなど）を、
//! 理由つきで「承知済み」と記録する。`structure.interference` と `structure.cone_clearance` に**共通の 1 つの一覧**。
//! 承知しても所見は出力に残り、判定が警告にならないだけ（意味と照合の規則は `sat_analysis::structure::acknowledged`）。

use crate::enum_text::enum_text;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

enum_text! {
    /// 承知する所見の種類（どの解析の所見か）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
    pub enum FindingKind: "finding kind" {
        /// `structure.interference` の、箱の重なり
        Interference = "interference",
        /// `structure.cone_clearance` の、噴流・視野の円錐に入る部品
        ConeClearance = "cone_clearance",
    }
}

impl FindingKind {
    /// 順のある組か（円錐は（担い手, 遮る部品）の順で意味が違う。干渉は順序のない組）。
    pub fn is_ordered(self) -> bool {
        matches!(self, Self::ConeClearance)
    }
}

/// 承知済みの組 1 つ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AcknowledgedFinding {
    /// 所見の種類
    pub finding: FindingKind,
    /// 1 つめの部品（円錐では担い手）
    pub first: String,
    /// 2 つめの部品（円錐では円錐に入る部品）
    pub second: String,
    /// 効く形態（無ければすべての形態と基準の配置）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub configuration: Option<String>,
    /// 承知した理由（必須）
    pub reason: String,
}

impl AcknowledgedFinding {
    /// 同じ組・同じ形態の承知か（理由は比べない。順序のない組は並びを問わない）。追加・取り消しが同じ承知を指す規則はここだけ。
    pub fn same_target(&self, other: &Self) -> bool {
        self.finding == other.finding
            && self.configuration == other.configuration
            && self.same_pair(other.finding, &other.first, &other.second)
    }

    /// `first`・`second` の組が、この承知の組と同じか（`kind` の順序の規則で）。
    pub fn same_pair(&self, kind: FindingKind, first: &str, second: &str) -> bool {
        (self.first == first && self.second == second)
            || (!kind.is_ordered() && self.first == second && self.second == first)
    }
}
