//! 要求の判定のキー（`verify.check`）が名指すエンティティの削除の拒否。キーに入る名前の索引は model の `Design::references`
//! （`RefSite::RequirementCheck`。書式は `sat_checkkey`）で、名前の変更はそのキーを書き換える（`fune_store::write::refs`）。
//! 削除はキーを書き換えられない（名指す先が無くなる）ので、要求を直してからにするよう拒む。

use crate::error::OpError;
use fune_model::Design;
use fune_model::design::{RefSite, Referent};
use fune_model::path::{SettingPath, SettingScope};

/// 参照元 `sites` に、判定のキーでエンティティ `id` を名指している要求があれば拒む。
/// 地上局・回線・部品の削除と、熱の節点を消す設定の書き込みが使う。
pub(super) fn refuse_if_checked_by_requirements(
    what: &str,
    id: &str,
    sites: &[RefSite],
    verb: &str,
) -> Result<(), OpError> {
    let ids: Vec<&str> = sites
        .iter()
        .filter_map(|s| match s {
            RefSite::RequirementCheck { requirement, .. } => Some(requirement.as_str()),
            _ => None,
        })
        .collect();
    if ids.is_empty() {
        return Ok(());
    }
    Err(OpError::invalid(format!(
        "{what} `{id}` cannot be {verb}: `verify.check` of requirement(s) {} refers to it. \
         Update or remove those requirements first",
        ids.join(", ")
    )))
}

/// `analysis.thermal.nodes` の節点を消す設定の書き込み（値が無い書き込み）なら、その節点を温度範囲の判定のキーで
/// 名指している要求がないか見る（節点は設定のキーで、名前を変える Op は無い。消すとキーの指す先が無くなり、
/// 要求の限度が温度の解析に届かなくなる）。節点の指定が無い（`nodes` ごと消す）ときはすべての節点を見る。
pub(super) fn refuse_removing_checked_thermal_node(
    design: &Design,
    setting: &SettingPath,
    removing: bool,
) -> Result<(), OpError> {
    let s = &setting.segments;
    let names_nodes = setting.scope == SettingScope::Analysis
        && s.first().map(String::as_str) == Some("thermal")
        && s.get(1).map(String::as_str) == Some("nodes");
    if !removing || !names_nodes {
        return Ok(());
    }
    let nodes: Vec<&str> = match s.get(2) {
        Some(name) => vec![name.as_str()],
        None => design
            .analysis
            .thermal
            .iter()
            .flat_map(|t| t.nodes.keys().map(String::as_str))
            .collect(),
    };
    for node in nodes {
        let sites = design.references(Referent::ThermalNode(node));
        refuse_if_checked_by_requirements("Thermal node", node, &sites, "removed")?;
    }
    Ok(())
}
