//! Design → 表示用の型（製品ツリー・モード表・ネット・ハーネス・定義・ケース）。解決済みの値を使わないもの。

use crate::dto::{
    AttrInput, AttributeDefView, CaseView, DefsView, HarnessView, KindDefView, ModeHeader, ModeRow,
    ModeSegmentView, ModeSequenceView, ModeStartView, ModeTable, NetView, QuantityView, TreeNode,
    TreeNodeKind,
};
use crate::query::net_class;
use crate::query::values::authored_quantity;
use fune_model::builtin;
use fune_model::design::{AnalysisSystem, BudgetKind, Design, EntityKey, NodeRef};
use fune_model::path::PortRef;
use fune_model::resolve::Resolved;
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
        label: builtin::labels()
            .section
            .get(g.id.as_str())
            .filter(|_| g.parent.is_none())
            .map(|l| l.label.clone())
            .unwrap_or_else(|| g.id.to_string()),
        label_en: builtin::labels()
            .section
            .get(g.id.as_str())
            .filter(|_| g.parent.is_none())
            .and_then(|l| l.label_en.clone()),
        system: design
            .system_of(g.id.as_str())
            .map(fune_projection::system::to_analysis),
        parent: g.parent.as_ref().map(|n| n.id().to_owned()),
        children: children.get(g.id.as_str()).cloned().unwrap_or_default(),
    });
    let parts = design.parts.values().map(|p| TreeNode {
        id: p.id.to_string(),
        node: TreeNodeKind::Part,
        label: p.label.clone().unwrap_or_else(|| p.id.to_string()),
        label_en: None,
        system: design
            .system_of(p.id.as_str())
            .map(fune_projection::system::to_analysis),
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

/// 種別の語彙から、停止の状態と主な「オン」の状態を選ぶ（`(off_state, main_state)`）。
/// 停止は語彙にある `off`（無ければ None）。主なオンは語彙にある `on`、無ければ停止以外の最初の状態、それも無ければ None。
/// モード表の切り替え・記号が使う（Web は状態の名前を知らない）。
pub(crate) fn stop_and_main_states(states: &[String]) -> (Option<String>, Option<String>) {
    let has = |name: &str| states.iter().find(|s| s.as_str() == name).cloned();
    let off = has(builtin::STATE_OFF);
    let main = has(builtin::STATE_MAIN)
        .or_else(|| states.iter().find(|s| Some(*s) != off.as_ref()).cloned());
    (off, main)
}

/// 部品 × モードの表。
pub(crate) fn mode_table(design: &Design) -> ModeTable {
    let modes = design
        .ordered_modes()
        .into_iter()
        .map(|m| ModeHeader {
            max_duration: m
                .max_duration
                .as_ref()
                .map(|q| authored_quantity(q, Some("Time"))),
            when: m.when,
            start: m.start.as_ref().map(start_view),
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
            let states = design.part_states(p).to_vec();
            let (off_state, main_state) = stop_and_main_states(&states);
            ModeRow {
                part: p.id.to_string(),
                kind: design.part_kind_id(p).map(str::to_owned),
                states,
                off_state,
                main_state,
                shares: cells
                    .iter()
                    .filter(|(_, c)| !c.duty.is_empty())
                    .map(|(m, c)| (m.clone(), duty_shares(c)))
                    .collect(),
                eclipse_cells: design
                    .modes
                    .values()
                    .filter_map(|m| {
                        m.eclipse
                            .get(p.id.as_str())
                            .map(|a| (m.id.to_string(), a.clone()))
                    })
                    .collect(),
                cells,
            }
        })
        .collect();
    let power = design.analysis.power.as_ref();
    let sequences = power
        .map(|p| {
            p.sequence_names()
                .into_iter()
                .filter_map(|n| p.sequence(n))
                .map(|s| ModeSequenceView {
                    name: s.name.to_owned(),
                    legacy: s.legacy,
                    anchor: s.anchor.map(start_view),
                    segments: s
                        .segments
                        .iter()
                        .map(|g| ModeSegmentView {
                            mode: g.mode.clone(),
                            duration: authored_quantity(&g.duration, Some("Time")),
                        })
                        .collect(),
                })
                .collect()
        })
        .unwrap_or_default();
    ModeTable {
        modes,
        rows,
        sequences,
        active_sequence: power.and_then(|p| p.active_sequence_name().map(str::to_owned)),
    }
}

/// 食の出入りからの位置の指定の表示用。
fn start_view(r: &fune_model::design::StartRef) -> ModeStartView {
    ModeStartView {
        from: r.from,
        offset: r
            .offset
            .as_ref()
            .map(|q| authored_quantity(q, Some("Time"))),
    }
}

/// ネット（つながるポートは部品の ports から集める）。
pub(crate) fn nets(design: &Design, resolved: &Resolved) -> Vec<NetView> {
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
        .map(|n| {
            let members = members.remove(n.id.as_str()).unwrap_or_default();
            NetView {
                id: n.id.to_string(),
                kind: n.kind,
                is_return: n.is_return(),
                power_class: net_class::power_class(n, &members, net_class::roles(resolved)),
                hub: net_class::hub(n, &members, net_class::roles(resolved)),
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
                members,
                field_paths: net_field_paths(n.id.as_str()),
                file: file_of(design, EntityKey::Net(n.id.to_string())),
            }
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
            unit_vector: a.unit_vector,
            fov_axis: a.fov_axis.as_ref().map(ToString::to_string),
            builtin: !design.attributes.contains_key(id) && builtin::attributes().contains_key(id),
            description: a.description.clone(),
            input: crate::fixes::attr_input(design, id).unwrap_or(AttrInput::Text),
        })
        .collect();
    let kinds = design
        .all_kinds()
        .into_iter()
        .map(|(id, k)| KindDefView {
            id: id.to_owned(),
            label: k.label.clone(),
            label_en: k.label_en.clone(),
            role: design.kind_role(id),
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
        systems: AnalysisSystem::ALL
            .iter()
            .map(|s| fune_projection::system::to_analysis(*s))
            .collect(),
        budget_kinds: BudgetKind::ALL.to_vec(),
        mode_systems: crate::mode_systems::mode_systems(),
        mode_levels: crate::mode_systems::mode_levels(),
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
            phase: c.phase.as_ref().map(ToString::to_string),
            epoch: c.epoch.clone(),
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

/// ネットのフィールドの対象のパス（フィールド名 → `nets.P12V.rail`）。名前は `NetFieldName::NAMES`、書き方は `Target::net` の表示
/// （診断の `DiagnosticView.path` と同じなので、弾かれた理由がその欄に付く）。
fn net_field_paths(net: &str) -> BTreeMap<String, String> {
    fune_model::design::NetFieldName::NAMES
        .iter()
        .map(|field| {
            let target = fune_model::Target::net(net, Some(field));
            ((*field).to_owned(), target.to_string())
        })
        .collect()
}

/// 部品のフィールドのパス（フィールド名 → `RW_X.count`）。名前は `fune_model::design::PartFieldName::NAMES`、記法は `ParamPath::Field`。
pub(crate) fn field_paths(part: &str) -> BTreeMap<String, String> {
    fune_model::design::PartFieldName::NAMES
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
        let labels: Vec<String> = nets(&design, &Resolved::default())
            .into_iter()
            .map(|n| n.label)
            .collect();
        assert_eq!(labels, ["N1", "バス"]);
        assert_eq!(harnesses(&design)[0].label, "H1");
    }

    fn pick(states: &[&str]) -> (Option<String>, Option<String>) {
        let states: Vec<String> = states.iter().map(|s| s.to_string()).collect();
        stop_and_main_states(&states)
    }

    /// 停止は `off`、主なオンは `on`（無ければ停止以外の最初）。どちらも無い語彙・空の語彙は None。
    #[test]
    fn stop_and_main_states_follow_the_vocabulary() {
        let some = |s: &str| Some(s.to_string());
        assert_eq!(
            pick(&["off", "idle", "on", "peak"]),
            (some("off"), some("on"))
        );
        assert_eq!(pick(&["rx", "tx", "off"]), (some("off"), some("rx")));
        assert_eq!(pick(&["active", "idle"]), (None, some("active")));
        assert_eq!(pick(&["off"]), (some("off"), None));
        assert_eq!(pick(&[]), (None, None));
    }
}
