//! 幾何の所見に関わる部品ごとの状態（`dto::PartFinding`）。**どの部品が所見に関わり、どの状態（承知でない所見が残る／すべて承知済み）かを決めるのはここだけ。**
//! 3D の配置は部品の ID で色を引くだけにする。所見に関わらない部品は載せない（Web は色を付けない）。
//! 所見は `structure.interference` の重なっている 2 つの部品と、`structure.cone_clearance` の担い手と円錐に入る部品。

use crate::dto::{AnalysisRunResult, AnalysisRunStatus, FindingState, PartFinding};
use sat_analysis::spec::Analysis;
use sat_analysis::structure::{
    ConeClearance, ConeClearanceOutput, Interference, InterferenceOutput,
};
use std::collections::BTreeMap;

/// 実行の結果が干渉か円錐の遮りのときだけ、所見に関わる部品の状態を返す（部品の ID 順。所見が無ければ空）。
pub(crate) fn part_findings(result: &AnalysisRunResult) -> Option<Vec<PartFinding>> {
    if result.status != AnalysisRunStatus::Ok {
        return None;
    }
    let output = result.output.clone()?;
    // 所見 1 件 = (関わる 2 つの部品, 承知済みか)
    let findings: Vec<(String, String, bool)> = if result.id == Interference::ID {
        let o: InterferenceOutput = serde_json::from_value(output).ok()?;
        o.overlaps
            .into_iter()
            .map(|p| (p.first, p.second, p.acknowledged))
            .collect()
    } else if result.id == ConeClearance::ID {
        let o: ConeClearanceOutput = serde_json::from_value(output).ok()?;
        o.cones
            .into_iter()
            .flat_map(|c| {
                let id = c.id;
                c.obstructions
                    .into_iter()
                    .map(move |x| (id.clone(), x.part, x.acknowledged))
            })
            .collect()
    } else {
        return None;
    };
    Some(of_findings(&findings))
}

/// 所見から部品の状態を作る。承知でない所見が 1 つでも残る部品は `Open`、関わる所見がすべて承知済みなら `Acknowledged`。
fn of_findings(findings: &[(String, String, bool)]) -> Vec<PartFinding> {
    let mut by_part: BTreeMap<&str, FindingState> = BTreeMap::new();
    for (first, second, acknowledged) in findings {
        let state = if *acknowledged {
            FindingState::Acknowledged
        } else {
            FindingState::Open
        };
        for part in [first, second] {
            let slot = by_part.entry(part).or_insert(state);
            if state == FindingState::Open {
                *slot = FindingState::Open;
            }
        }
    }
    by_part
        .into_iter()
        .map(|(part, state)| PartFinding {
            part: part.to_owned(),
            state,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(a: &str, b: &str, ack: bool) -> (String, String, bool) {
        (a.to_owned(), b.to_owned(), ack)
    }

    /// 承知でない所見が 1 つでもある部品は `Open`（ほかの所見が承知済みでも）。承知済みの所見だけの部品は `Acknowledged`。
    #[test]
    fn a_part_with_any_open_finding_is_open() {
        let parts = of_findings(&[f("A", "B", true), f("B", "C", false), f("D", "E", true)]);
        let state = |id: &str| parts.iter().find(|p| p.part == id).map(|p| p.state);
        assert_eq!(state("A"), Some(FindingState::Acknowledged));
        assert_eq!(state("B"), Some(FindingState::Open));
        assert_eq!(state("C"), Some(FindingState::Open));
        assert_eq!(state("D"), Some(FindingState::Acknowledged));
        assert_eq!(state("Z"), None);
        let ids: Vec<&str> = parts.iter().map(|p| p.part.as_str()).collect();
        assert_eq!(ids, ["A", "B", "C", "D", "E"]);
    }
}
