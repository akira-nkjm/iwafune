//! Design → 表示用の型（製品ツリー・モード表・ネット・ハーネス・定義・ケース）。解決済みの値を使わないもの。

use crate::dto::{
    AttributeDefView, CaseView, DefsView, HarnessView, KindDefView, ModeHeader, ModeRow, ModeTable,
    NetView, QuantityView, TreeNode, TreeNodeKind,
};
use crate::query::values::authored_quantity;
use fune_model::builtin;
use fune_model::design::{Design, EntityKey, NodeRef};
use fune_model::path::PortRef;
use std::collections::BTreeMap;

/// エンティティのファイル（ルートからの相対パス）。
pub(crate) fn file_of(design: &Design, key: EntityKey) -> String {
    design
        .source_map
        .file_of(&key)
        .unwrap_or_default()
        .to_owned()
}

/// 製品ツリー（Group と Part）。親から子の順（ID 順）。
pub(crate) fn tree(design: &Design) -> Vec<TreeNode> {
    let mut children: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for g in design.groups.values() {
        if let Some(p) = &g.parent {
            children
                .entry(p.id().to_owned())
                .or_default()
                .push(g.id.to_string());
        }
    }
    for p in design.parts.values() {
        if let Some(parent) = &p.parent {
            children
                .entry(parent.id().to_owned())
                .or_default()
                .push(p.id.to_string());
        }
    }
    let groups = design.groups.values().map(|g| TreeNode {
        id: g.id.to_string(),
        node: TreeNodeKind::Group,
        label: g.label.clone().unwrap_or_else(|| g.id.to_string()),
        parent: g.parent.as_ref().map(|n| n.id().to_owned()),
        children: children.get(g.id.as_str()).cloned().unwrap_or_default(),
    });
    let parts = design.parts.values().map(|p| TreeNode {
        id: p.id.to_string(),
        node: TreeNodeKind::Part,
        label: p.label.clone().unwrap_or_else(|| p.id.to_string()),
        parent: p.parent.as_ref().map(|n| n.id().to_owned()),
        children: children.get(p.id.as_str()).cloned().unwrap_or_default(),
    });
    let mut nodes: Vec<TreeNode> = groups.chain(parts).collect();
    let depth = |n: &TreeNode| {
        let node = match n.node {
            TreeNodeKind::Group => NodeRef::Group(fune_model::id::Id::new(&n.id)),
            TreeNodeKind::Part => NodeRef::Part(fune_model::id::Id::new(&n.id)),
        };
        design.ancestors(&node).len()
    };
    nodes.sort_by(|a, b| depth(a).cmp(&depth(b)).then_with(|| a.id.cmp(&b.id)));
    nodes
}

/// 部品 × モードの表。
pub(crate) fn mode_table(design: &Design) -> ModeTable {
    let modes = design
        .modes
        .values()
        .map(|m| ModeHeader {
            id: m.id.to_string(),
            label: m.label.clone().unwrap_or_else(|| m.id.to_string()),
            description: m.description.clone(),
            file: file_of(design, EntityKey::Mode(m.id.to_string())),
        })
        .collect();
    let rows = design
        .parts
        .values()
        .filter(|p| !design.part_states(p).is_empty())
        .map(|p| {
            let cells: BTreeMap<String, fune_model::design::StateAssign> = design
                .modes
                .values()
                .filter_map(|m| {
                    m.states
                        .get(p.id.as_str())
                        .map(|a| (m.id.to_string(), a.clone()))
                })
                .collect();
            ModeRow {
                part: p.id.to_string(),
                kind: design.part_kind_id(p).map(str::to_owned),
                states: design.part_states(p).to_vec(),
                shares: cells
                    .iter()
                    .filter(|(_, c)| !c.duty.is_empty())
                    .map(|(m, c)| (m.clone(), duty_shares(c)))
                    .collect(),
                cells,
            }
        })
        .collect();
    ModeTable { modes, rows }
}

/// ネット（つながるポートは部品の ports から集める）。
pub(crate) fn nets(design: &Design) -> Vec<NetView> {
    let mut members: BTreeMap<&str, Vec<PortRef>> = BTreeMap::new();
    for p in design.parts.values() {
        for (port, binding) in &p.ports {
            if let Some(net) = &binding.net {
                members
                    .entry(net.as_str())
                    .or_default()
                    .push(PortRef::new(p.id.as_str(), port.as_str()));
            }
        }
    }
    design
        .nets
        .values()
        .map(|n| NetView {
            id: n.id.to_string(),
            kind: n.kind,
            is_return: n.is_return(),
            label: n.label.clone().unwrap_or_else(|| n.id.to_string()),
            source: n.source.clone(),
            rail: n
                .rail
                .as_ref()
                .map(|q| authored_quantity(q, Some("Voltage"))),
            limit: n
                .limit
                .as_ref()
                .map(|q| authored_quantity(q, Some("ElectricCurrent"))),
            protocol: n.protocol.clone(),
            bitrate: n
                .bitrate
                .as_ref()
                .map(|q| authored_quantity(q, Some("DataRate"))),
            master: n.master.clone(),
            members: members.remove(n.id.as_str()).unwrap_or_default(),
            file: file_of(design, EntityKey::Net(n.id.to_string())),
        })
        .collect()
}

/// ハーネス。
pub(crate) fn harnesses(design: &Design) -> Vec<HarnessView> {
    design
        .harnesses
        .values()
        .map(|h| HarnessView {
            id: h.id.to_string(),
            label: h.label.clone().unwrap_or_else(|| h.id.to_string()),
            from: h.from.clone(),
            to: h.to.clone(),
            nets: h.nets.iter().map(ToString::to_string).collect(),
            wire: h.wire.clone(),
            cores: h.cores.unwrap_or(1),
            length: h
                .length
                .as_ref()
                .map(|q| authored_quantity(q, Some("Length"))),
            file: file_of(design, EntityKey::Harness(h.id.to_string())),
        })
        .collect()
}

/// 属性定義と種別定義（組み込み＋プロジェクト独自）。
pub(crate) fn defs(design: &Design) -> DefsView {
    let attributes = design
        .all_attributes()
        .into_iter()
        .map(|(id, a)| AttributeDefView {
            id: id.to_owned(),
            label: a.label.clone(),
            label_en: a.label_en.clone(),
            value_type: a.value_type,
            section: design.attribute_section(id).to_owned(),
            dimension: a.dimension.clone(),
            units: a.units.clone(),
            display_unit: Some(a.display_unit().to_owned()).filter(|u| !u.is_empty()),
            by_state: a.by_state,
            builtin: !design.attributes.contains_key(id) && builtin::attributes().contains_key(id),
            description: a.description.clone(),
        })
        .collect();
    let kinds = design
        .all_kinds()
        .into_iter()
        .map(|(id, k)| KindDefView {
            id: id.to_owned(),
            label: k.label.clone(),
            label_en: k.label_en.clone(),
            role: design.kind_role(id).map(str::to_owned),
            parent: k.extends.as_ref().map(ToString::to_string),
            states: design.kind_states(id).to_vec(),
            required: k.required.iter().map(ToString::to_string).collect(),
            optional: k.optional.iter().map(ToString::to_string).collect(),
            ports: k.ports.keys().cloned().collect(),
            builtin: !design.kinds.contains_key(id) && builtin::kinds().contains_key(id),
        })
        .collect();
    DefsView {
        attributes,
        kinds,
        labels: builtin::labels().clone(),
    }
}

/// 解析ケース。
pub(crate) fn cases(design: &Design) -> Vec<CaseView> {
    design
        .cases
        .values()
        .map(|c| CaseView {
            id: c.id.to_string(),
            label: c.label.clone().unwrap_or_else(|| c.id.to_string()),
            description: c.description.clone(),
            modes: c.mode.iter().map(ToString::to_string).collect(),
            file: file_of(design, EntityKey::Case(c.id.to_string())),
        })
        .collect()
}

/// モード表のセルの割合（状態 → `%` の表示。整形は `crate::units::percent`）。
/// セルの文言（`rx（tx 10.0 %）`）は CLI と Web が言語ごとに作る。
fn duty_shares(c: &fune_model::design::StateAssign) -> BTreeMap<String, QuantityView> {
    c.duty
        .iter()
        .map(|(s, f)| (s.clone(), crate::units::percent(*f)))
        .collect()
}

/// 部品のフィールドのパス（フィールド名 → `RW_X.count`）。名前は `fune_model::path::PART_FIELDS`、記法は `ParamPath::Field`。
pub(crate) fn field_paths(part: &str) -> BTreeMap<String, String> {
    fune_model::path::PART_FIELDS
        .iter()
        .map(|field| {
            let path = fune_model::path::ParamPath::Field {
                part: part.to_owned(),
                field: (*field).to_owned(),
            };
            ((*field).to_owned(), path.to_string())
        })
        .collect()
}

/// 全属性と状態のフィールドパス。未設定の欄も同じ記法で編集できる。
pub(crate) fn parameter_paths(
    design: &Design,
    entity: &str,
    states: &[String],
) -> BTreeMap<String, BTreeMap<String, String>> {
    design
        .all_attributes()
        .keys()
        .map(|attr| {
            let paths = std::iter::once(None)
                .chain(states.iter().map(Some))
                .map(|state| {
                    let path = fune_model::path::ParamPath::Attr {
                        part: entity.to_owned(),
                        attr: (*attr).to_owned(),
                        state: state.cloned(),
                    }
                    .to_string();
                    (state.cloned().unwrap_or_default(), path)
                })
                .collect();
            ((*attr).to_owned(), paths)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 表示名の無いネット・ハーネスは ID で埋める（製品ツリー・モードの見出し・ケースと同じ）。
    #[test]
    fn labels_fall_back_to_ids() {
        let mut design = Design::default();
        for net in [
            json!({ "id": "N1", "kind": "power" }),
            json!({ "id": "N2", "kind": "data", "label": "バス" }),
        ] {
            let net: fune_model::design::Net = serde_json::from_value(net).unwrap();
            design.nets.insert(net.id.clone(), net);
        }
        let harness: fune_model::design::Harness =
            serde_json::from_value(json!({ "id": "H1", "from": "A.P1", "to": "B.P2" })).unwrap();
        design.harnesses.insert(harness.id.clone(), harness);
        let labels: Vec<String> = nets(&design).into_iter().map(|n| n.label).collect();
        assert_eq!(labels, ["N1", "バス"]);
        assert_eq!(harnesses(&design)[0].label, "H1");
    }
}
