//! 部品の表示（一覧の行・1 部品の全情報・1 つの値）。

use crate::dto::{
    AttrSection, DiagnosticCounts, DiagnosticView, PartRow, PartView, PartsQuery, PortView,
    ValueAnswer, ValueQuery,
};
use crate::error::{Result, ServiceError};
use crate::query::values::value_view;
use crate::query::views::file_of;
use crate::units::quantity_auto;
use fune_model::design::{Design, EntityKey, NodeRef};
use fune_model::id::Id;
use fune_model::path::{ParamPath, parse};
use fune_model::resolve::{Origin, Resolved, ResolvedPart};

/// 部品を対象にした診断の件数（`DiagnosticView.part` が部品 ID のもの。部品の診断の件数の唯一の数え方）。
fn part_diagnostics(diagnostics: &[DiagnosticView], id: &str) -> DiagnosticCounts {
    DiagnosticCounts::of(diagnostics.iter().filter(|d| d.part.as_deref() == Some(id)))
}

/// 未設定の値（出所が `unset`）の数（部品の未設定の数の唯一の数え方）。
fn unset_count(rp: &ResolvedPart) -> u32 {
    let n = rp
        .attr
        .values()
        .filter(|v| v.origin == Origin::Unset)
        .count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// 部品の一覧の 1 行。
pub(crate) fn part_row(
    design: &Design,
    resolved: &Resolved,
    diagnostics: &[DiagnosticView],
    rp: &ResolvedPart,
) -> PartRow {
    let part = design.parts.get(rp.id.as_str());
    let unit = super::pose::position_unit(design, part);
    PartRow {
        paths: crate::query::views::parameter_paths(design, &rp.id, &rp.states),
        field_paths: crate::query::views::field_paths(&rp.id),
        id: rp.id.clone(),
        parent: rp.parent.as_ref().map(|n| n.id().to_owned()),
        system: rp.system.map(fune_projection::system::to_analysis),
        kind: rp.kind.clone(),
        role: rp.role,
        maneuver_thruster: part.is_some_and(|p| design.is_maneuver_thruster(p)),
        diagnostics: part_diagnostics(diagnostics, &rp.id),
        unset_count: unset_count(rp),
        catalog: rp.catalog.clone(),
        candidates: part.map(|p| p.candidates.clone()).unwrap_or_default(),
        count: rp.count,
        redundancy: rp.redundancy,
        maturity: rp.maturity,
        values: rp
            .attr
            .iter()
            .map(|(k, v)| (k.clone(), value_view(k, v)))
            .collect(),
        ports: rp
            .ports
            .iter()
            .map(|(id, p)| PortView {
                id: id.clone(),
                net: p.net.clone(),
            })
            .collect(),
        mount: super::pose::mount_view(part, rp, &unit),
        mount_points: super::pose::mount_points(part),
        body: super::pose::body_pose_view(resolved, rp, &unit),
        states: rp.states.clone(),
        file: file_of(design, EntityKey::Part(rp.id.clone())),
        // 置き場所の区間はディレクトリの規約（`fune_store::layout`）が親の連鎖から作る（パスの文字列は分解しない）
        dir: fune_store::layout::dirs_for_parent(design, rp.parent.as_ref()),
    }
}

/// 部品の一覧（絞り込み付き）。
pub(crate) fn list(
    design: &Design,
    resolved: &Resolved,
    diagnostics: &[DiagnosticView],
    q: &PartsQuery,
) -> Vec<PartRow> {
    resolved
        .parts
        .values()
        .filter(|rp| {
            q.kind
                .as_ref()
                .is_none_or(|k| rp.kind.as_deref() == Some(k.as_str()))
        })
        .filter(|rp| q.group.as_ref().is_none_or(|g| in_group(design, &rp.id, g)))
        .map(|rp| part_row(design, resolved, diagnostics, rp))
        .collect()
}

fn in_group(design: &Design, part: &str, group: &str) -> bool {
    design.system_of(part).is_some_and(|s| s.as_str() == group)
        || design
            .ancestors(&NodeRef::Part(Id::new(part)))
            .iter()
            .any(|n| n.id() == group)
}

/// 1 部品の全情報。
pub(crate) fn view(
    design: &Design,
    resolved: &Resolved,
    diagnostics: &[DiagnosticView],
    id: &str,
) -> Result<PartView> {
    let rp = resolved
        .parts
        .get(id)
        .ok_or_else(|| ServiceError::not_found("part", id))?;
    let part = part_row(design, resolved, diagnostics, rp);
    // 系の順と見出しは `fune_model::builtin::sections`、属性の系は `Design::attribute_section`。
    let mut sections: Vec<AttrSection> = fune_model::builtin::sections()
        .into_iter()
        .map(|(key, label)| AttrSection {
            key: key.to_owned(),
            label: label.label.clone(),
            label_en: label.label_en.clone(),
            attrs: Vec::new(),
        })
        .collect();
    for attr in part.values.keys() {
        let key = design.attribute_section(attr);
        let index = sections.iter().position(|s| s.key == key).unwrap_or(0);
        sections[index].attrs.push(attr.clone());
    }
    sections.retain(|s| !s.attrs.is_empty());
    let modes = design
        .modes
        .values()
        .filter_map(|m| m.states.get(id).map(|a| (m.id.to_string(), a.clone())))
        .collect();
    // ハーネスは端のポートの部品（構造）で選ぶ
    let harnesses = design
        .harnesses
        .values()
        .filter(|h| h.from.part == id || h.to.part == id)
        .map(|h| h.id.to_string())
        .collect();
    // 診断は対象の構造で選ぶ（部品のファイルの中の対象か、部品のファイルの位置）
    let diagnostics = diagnostics
        .iter()
        .filter(|d| {
            d.part.as_deref() == Some(id)
                || d.diagnostic.file.as_deref() == Some(part.file.as_str())
        })
        .cloned()
        .collect();
    Ok(PartView {
        part,
        sections,
        modes,
        harnesses,
        diagnostics,
    })
}

/// 1 つの値（`RW_X.power`、`RW_X.power.on`）。
pub(crate) fn get_value(resolved: &Resolved, path: &str, q: &ValueQuery) -> Result<ValueAnswer> {
    let parsed = parse(path).map_err(|e| ServiceError::InvalidInput(e.to_string()))?;
    let ParamPath::Attr { part, attr, state } = parsed else {
        return Err(ServiceError::InvalidInput(format!(
            "Not a part attribute path: {path} (for settings, see `fune setting ls`)"
        )));
    };
    let rp = resolved
        .parts
        .get(part.as_str())
        .ok_or_else(|| ServiceError::not_found("part", part.as_str()))?;
    let rv = rp
        .attr
        .get(&attr)
        .ok_or_else(|| ServiceError::not_found("attribute", format!("{part}.{attr}")))?;
    let state = state.or_else(|| q.state.clone());
    let selected = match &state {
        None => None,
        Some(s) => {
            let v = rv.value.get().and_then(|v| v.at_state(s)).ok_or_else(|| {
                ServiceError::not_found("state value", format!("{part}.{attr}.{s}"))
            })?;
            Some(quantity_auto(
                v.value,
                rv.dimension.as_deref().unwrap_or("Dimensionless"),
            ))
        }
    };
    Ok(ValueAnswer {
        path: format!("{part}.{attr}"),
        value: value_view(&attr, rv),
        selected,
    })
}
