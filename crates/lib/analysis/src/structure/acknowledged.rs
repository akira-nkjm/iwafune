//! 承知済みの組（`structure.interference` と `structure.cone_clearance` に共通）。
//!
//! 箱で表すと必ず出てしまう所見（ヒンジの縁の重なりなど）を、設計者が理由つきで「承知済み」と記録したもの。
//! 承知済みの所見も**計算して出力に残す**（`acknowledged: true` と理由つき）が、判定は警告にしない（`ok`。見えなくはしない）。
//! どの形態に効くか・どの解析の所見かの絞り込みは射影が済ませる（解析は、いま見ている配置に効く承知だけを受け取る）。
//!
//! 照合の規則はここだけ: 干渉は順序のない組（どちらの並びで書いても同じ）、円錐は（担い手, 遮る部品）の順のある組。

use serde::{Deserialize, Serialize};

/// 承知済みの組 1 つ（いま見ている配置に効くもの）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Acknowledgement {
    /// 1 つめの部品（円錐では担い手）
    pub first: String,
    /// 2 つめの部品（円錐では円錐に入る部品）
    pub second: String,
    /// 承知した理由
    pub reason: String,
    /// 承知を書いた形態（無ければすべての配置に効く承知。出力に残して、取り消す側が同じ承知を指せるようにする）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub configuration: Option<String>,
}

/// 照合の結果: 承知済みならその承知。
pub(crate) fn find<'a>(
    list: &'a [Acknowledgement],
    first: &str,
    second: &str,
    ordered: bool,
) -> Option<&'a Acknowledgement> {
    list.iter().find(|a| {
        (a.first == first && a.second == second)
            || (!ordered && a.first == second && a.second == first)
    })
}

/// 承知済みの判定文の頭（判定は `ok` にして、承知であることと理由を文に残す）。
pub(crate) fn note(reason: &str) -> String {
    format!("Acknowledged: {}. ", reason.trim().trim_end_matches('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ack(first: &str, second: &str) -> Vec<Acknowledgement> {
        vec![Acknowledgement {
            first: first.to_owned(),
            second: second.to_owned(),
            reason: "r".to_owned(),
            configuration: None,
        }]
    }

    #[test]
    fn an_unordered_pair_matches_either_way_and_an_ordered_pair_only_one_way() {
        let list = ack("A", "B");
        let reason = |a: Option<&Acknowledgement>| a.map(|a| a.reason.clone());
        assert_eq!(reason(find(&list, "B", "A", false)).as_deref(), Some("r"));
        assert_eq!(reason(find(&list, "A", "B", false)).as_deref(), Some("r"));
        assert_eq!(reason(find(&list, "A", "B", true)).as_deref(), Some("r"));
        assert!(find(&list, "B", "A", true).is_none());
        assert!(find(&list, "A", "C", false).is_none());
    }
}
