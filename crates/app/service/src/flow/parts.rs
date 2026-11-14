//! 部品のカードと行。値は `query::values::value_view`。まとめたカード（`group`）の行は、部品の値がどれも同じならその値、
//! スカラーで違えば範囲（最小 – 最大。`units::quantity_auto`）、それ以外は「値を持つ部品の数」（`FlowRow.varies`）。

use super::cards::CardKey;
use crate::dto::{FlowMember, FlowNode, FlowNodeKind, FlowRow, ValueData};
use crate::query::values::value_view;
use crate::workspace::Snapshot;
use fune_model::Target;

/// 部品のカード（1 部品なら部品の名前、まとめたカードなら役割の名前と部品の一覧）。
pub(super) fn node(s: &Snapshot, key: &CardKey, ids: &[String]) -> FlowNode {
    let first = ids.first().and_then(|id| s.resolved.parts.get(id.as_str()));
    let members: Vec<FlowMember> = ids
        .iter()
        .map(|id| FlowMember {
            id: id.clone(),
            label: s
                .resolved
                .parts
                .get(id.as_str())
                .and_then(|p| p.label.clone())
                .unwrap_or_else(|| id.clone()),
            label_en: id.clone(),
        })
        .collect();
    let (label, label_en, sub, target) = match members.as_slice() {
        [one] => (
            one.label.clone(),
            one.id.clone(),
            Some(one.id.clone()),
            Some(Target::Part { id: one.id.clone() }),
        ),
        _ => {
            let role = first
                .and_then(|p| p.role)
                .and_then(fune_model::builtin::role_label);
            let ja = role
                .as_ref()
                .map_or_else(|| ids.join(", "), |l| l.label.clone());
            let en = role.and_then(|l| l.label_en).unwrap_or_else(|| ja.clone());
            (ja, en, None, None)
        }
    };
    FlowNode {
        id: key.node_id(),
        kind: FlowNodeKind::Part,
        label,
        label_en,
        sub,
        system: first
            .and_then(|p| p.system)
            .map(fune_projection::system::to_analysis),
        target,
        rows: Vec::new(),
        analysis: None,
        verdict: None,
        status: None,
        members: if ids.len() > 1 { members } else { Vec::new() },
    }
}

/// 属性の行（まとめたカードでは部品の値をまとめる）。
pub(super) fn row(s: &Snapshot, ids: &[String], attr: &str, target: &Target) -> FlowRow {
    let values: Vec<ValueData> = ids
        .iter()
        .filter_map(|id| s.resolved.parts.get(id.as_str())?.attr.get(attr))
        .map(|rv| value_view(attr, rv).value)
        .filter(|v| !matches!(v, ValueData::Unset))
        .collect();
    let displays: Vec<String> = values
        .iter()
        .filter_map(crate::query::values::display_of)
        .collect();
    let (display, varies) = combined(&values, &displays, ids.len());
    let def = s.design.attribute(attr);
    FlowRow {
        id: attr.to_owned(),
        label: def.map_or_else(|| attr.to_owned(), |d| d.label.clone()),
        label_en: def
            .and_then(|d| d.label_en.clone())
            .unwrap_or_else(|| attr.to_owned()),
        set: !displays.is_empty(),
        display,
        derived: false,
        varies,
        verdict: None,
        target: (ids.len() == 1).then(|| target.clone()),
    }
}

/// 部品の値をまとめた表示。どれも同じならその値、スカラーで違えば範囲、それ以外は値を持つ部品の数。
fn combined(
    values: &[ValueData],
    displays: &[String],
    members: usize,
) -> (Option<String>, Option<u32>) {
    let Some(first) = displays.first() else {
        return (None, None);
    };
    if displays.len() == members && displays.iter().all(|d| d == first) {
        return (Some(first.clone()), None);
    }
    let scalars: Vec<(f64, &str)> = values
        .iter()
        .filter_map(|v| match v {
            ValueData::Scalar { nominal, .. } => Some((nominal.si, nominal.dimension.as_str())),
            _ => None,
        })
        .collect();
    let dim = scalars.first().map(|(_, d)| *d);
    if scalars.len() == members && scalars.iter().all(|(_, d)| Some(*d) == dim) {
        let lo = scalars
            .iter()
            .map(|(v, _)| *v)
            .fold(f64::INFINITY, f64::min);
        let hi = scalars
            .iter()
            .map(|(v, _)| *v)
            .fold(f64::NEG_INFINITY, f64::max);
        let shown = |v: f64| crate::units::quantity_auto(v, dim.unwrap_or_default()).display;
        return (Some(format!("{} – {}", shown(lo), shown(hi))), None);
    }
    (None, u32::try_from(displays.len()).ok())
}
