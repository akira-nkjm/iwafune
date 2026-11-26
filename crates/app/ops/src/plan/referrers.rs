//! 削除の拒否の文面: 消そうとしたものを指している場所を、種類ごとに ID を挙げて 1 つの文にする。
//! 参照元の索引は model の `Design::references`（[`RefSite`]）。地上局・回線・部品の判定のキーは `check_keys`、
//! 定義（種別・属性）とカタログ品は専用の文面を持つ。ここは解析ケース・フェーズ・モード・形態・出典・マヌーバが使う。

use crate::error::OpError;
use fune_model::design::RefSite;

/// 参照元 1 つの呼び名（`(場所の説明, 参照元のエンティティの ID)`）。ID が無い場所（設定のキーなど）は `None`。
/// 説明が同じ参照元は 1 つにまとめて ID を並べる。
fn describe(site: &RefSite) -> Option<(&'static str, Option<&str>)> {
    Some(match site {
        RefSite::CaseMode { case } => ("`mode` of case(s)", Some(case)),
        RefSite::CasePhase { case } => ("`phase` of case(s)", Some(case)),
        RefSite::CommonDefaultCase => ("`default_case` in analysis/common.toml", None),
        RefSite::RequirementCase { requirement } => {
            ("`quantify.cases` of requirement(s)", Some(requirement))
        }
        RefSite::RequirementSource { requirement } => {
            ("`source` of requirement(s)", Some(requirement))
        }
        RefSite::PowerModeShare => ("`mode_share` in analysis/power.toml", None),
        RefSite::PowerSequence { .. } => ("`[[sequence]]` in analysis/power.toml", None),
        RefSite::PowerNamedSequence { sequence, .. } => (
            "named power sequence(s) in analysis/power.toml",
            Some(sequence),
        ),
        RefSite::PowerBaseMode => ("`base_mode` in analysis/power.toml", None),
        RefSite::AttitudeLaw => ("`[law]` in analysis/aocs.toml", None),
        RefSite::ThermalCaseMode { case } => ("`mode` of thermal case(s)", Some(case)),
        RefSite::ManeuverMode { maneuver } => ("`constraints.mode` of maneuver(s)", Some(maneuver)),
        RefSite::PhaseMode { phase } => ("`modes` of phase(s)", Some(phase)),
        RefSite::PhaseLaw { phase } => ("`[law]` of phase(s)", Some(phase)),
        RefSite::PhaseConfiguration { phase } => ("`configuration` of phase(s)", Some(phase)),
        RefSite::PhaseOrbitManeuver { phase } => ("`[orbit].maneuver` of phase(s)", Some(phase)),
        RefSite::LaunchConfiguration => ("`configuration` in conditions/launch.toml", None),
        RefSite::AcknowledgedConfiguration { .. } => {
            ("`[[acknowledged]]` in analysis/structure.toml", None)
        }
        RefSite::OrbitManeuver => ("`maneuver` in conditions/orbit.toml", None),
        RefSite::PartValueSource { part, .. } => ("attribute value sources of part(s)", Some(part)),
        RefSite::CatalogValueSource { item, .. } => {
            ("attribute value sources of catalog item(s)", Some(item))
        }
        RefSite::CatalogItemSource { item } => ("`source` of catalog item(s)", Some(item)),
        RefSite::CatalogItemDatasheet { item } => ("`datasheet` of catalog item(s)", Some(item)),
        _ => return None,
    })
}

/// `sites` のうち呼び名のあるものを、種類ごと（初めて現れた順）・ID ごと（重複なし）にまとめた文。無ければ `None`。
fn referrers_text(sites: &[RefSite]) -> Option<String> {
    let mut groups: Vec<(&str, Vec<&str>)> = Vec::new();
    for (label, id) in sites.iter().filter_map(describe) {
        let at = match groups.iter().position(|(l, _)| *l == label) {
            Some(at) => at,
            None => {
                groups.push((label, Vec::new()));
                groups.len() - 1
            }
        };
        if let Some(id) = id
            && !groups[at].1.contains(&id)
        {
            groups[at].1.push(id);
        }
    }
    if groups.is_empty() {
        return None;
    }
    let parts: Vec<String> = groups
        .iter()
        .map(|(label, ids)| {
            if ids.is_empty() {
                (*label).to_string()
            } else {
                format!("{label} {}", ids.join(", "))
            }
        })
        .collect();
    Some(parts.join("; "))
}

/// 参照元 `sites` があれば、`what` の `id` を消すのを拒む。文面は「X cannot be removed: referred to by <種類ごとの参照元>」。
/// 参照元の索引は呼び出し側が `Design::references` で引く（書き換えで外せない参照元だけに絞って渡す）。
pub(super) fn refuse_if_referenced(what: &str, id: &str, sites: &[RefSite]) -> Result<(), OpError> {
    match referrers_text(sites) {
        None => Ok(()),
        Some(text) => Err(OpError::invalid(format!(
            "{what} `{id}` cannot be removed: referred to by {text}. Change those first"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_kinds_and_deduplicates_ids() {
        let sites = vec![
            RefSite::CaseMode { case: "a".into() },
            RefSite::PowerBaseMode,
            RefSite::CaseMode { case: "b".into() },
            RefSite::CaseMode { case: "a".into() },
            RefSite::PhaseMode { phase: "P".into() },
            RefSite::PhaseLaw { phase: "P".into() },
            RefSite::PowerSequence { index: 0 },
            RefSite::PowerSequence { index: 3 },
        ];
        assert_eq!(
            referrers_text(&sites).unwrap(),
            "`mode` of case(s) a, b; `base_mode` in analysis/power.toml; `modes` of phase(s) P; \
             `[law]` of phase(s) P; `[[sequence]]` in analysis/power.toml"
        );
        assert!(referrers_text(&[RefSite::ModeRow { mode: "m".into() }]).is_none());
    }
}
