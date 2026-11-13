//! 既存の負荷・発電・充放電の集計を流れ図に写す。
use super::label;
use crate::dto::*;
use sat_analysis::power::{
    energy_balance::{EnergyBalanceCaseResult, LoadGrowth},
    mode_load::ModeLoadMode,
};
use sat_units::typed::{NamedKind, Power};
use std::collections::BTreeMap;

pub(super) fn build(
    loads: &ModeLoadMode,
    case: &EnergyBalanceCaseResult,
    eclipse: bool,
    growth: LoadGrowth,
) -> PowerFlowView {
    let mut groups: BTreeMap<String, f64> = BTreeMap::new();
    for part in &loads.parts {
        let value = match (eclipse, growth) {
            (false, LoadGrowth::Included) => part.mean_with_growth,
            (false, LoadGrowth::Excluded) => part.mean,
            (true, LoadGrowth::Included) => part.mean_eclipse_with_growth,
            (true, LoadGrowth::Excluded) => part.mean_eclipse,
        };
        if let Some(v) = value {
            *groups.entry(part.group.clone()).or_default() += v.value();
        }
    }
    let source = if eclipse {
        case.load_eclipse.value()
    } else {
        case.sunlit_average_generation.value()
    };
    let mut rows: Vec<_> = groups
        .into_iter()
        .map(|(id, p)| {
            let system =
                serde_json::from_value::<System>(serde_json::Value::String(id.clone())).ok();
            let name = system
                .and_then(|s| {
                    let model = fune_projection::system::from_analysis(s);
                    let names = fune_model::builtin::labels();
                    names
                        .section
                        .get(model.as_str())
                        .map(|l| label(&l.label, l.label_en.as_deref().unwrap_or(&l.label)))
                })
                .unwrap_or_else(|| label(&id, &id));
            (id, name, system, p)
        })
        .collect();
    if !eclipse {
        rows.push((
            "charge".into(),
            label("電池へ充電", "Battery charge"),
            Some(System::Power),
            case.sunlit_average_charge.value(),
        ));
        rows.push((
            "shunt".into(),
            label("シャント", "Shunt"),
            Some(System::Power),
            case.sunlit_average_shunt.value(),
        ));
    }
    let max = rows
        .iter()
        .map(|r| r.3)
        .fold(source, f64::max)
        .max(case.sunlit_average_discharge.value())
        .max(f64::EPSILON);
    let height = (rows.len() as f64 * 64.0 + 64.0).max(240.0);
    let mid = height / 2.0;
    let mut nodes = vec![
        PowerFlowNode {
            id: "source".into(),
            label: if eclipse {
                label("電池", "Battery")
            } else {
                label("太陽電池", "Solar Cells")
            },
            system: Some(System::Power),
            x: 100.0,
            y: mid,
            value: Some(crate::units::quantity_auto(source, Power::name())),
        },
        PowerFlowNode {
            id: "bus".into(),
            label: label("バス", "Bus"),
            system: Some(System::Power),
            x: 330.0,
            y: mid,
            value: None,
        },
    ];
    let mut edges = vec![PowerFlowEdge {
        x1: 160.0,
        y1: mid,
        x2: 250.0,
        y2: mid,
        width: source / max * 22.0,
        system: Some(System::Power),
    }];
    if !eclipse && case.sunlit_average_discharge.value() > 0.0 {
        nodes.push(PowerFlowNode {
            id: "discharge".into(),
            label: label("電池から放電", "Battery discharge"),
            system: Some(System::Power),
            x: 100.0,
            y: 40.0,
            value: Some(crate::units::quantity_auto(
                case.sunlit_average_discharge.value(),
                Power::name(),
            )),
        });
        edges.push(PowerFlowEdge {
            x1: 160.0,
            y1: 40.0,
            x2: 250.0,
            y2: mid,
            width: case.sunlit_average_discharge.value() / max * 22.0,
            system: Some(System::Power),
        });
    }
    for (i, (id, label, system, power)) in rows.into_iter().enumerate() {
        let y = i as f64 * 64.0 + 48.0;
        nodes.push(PowerFlowNode {
            id,
            label,
            system,
            x: 650.0,
            y,
            value: Some(crate::units::quantity_auto(power, Power::name())),
        });
        edges.push(PowerFlowEdge {
            x1: 410.0,
            y1: mid,
            x2: 570.0,
            y2: y,
            width: power / max * 22.0,
            system,
        });
    }
    edges.retain(|e| e.width > 0.0);
    PowerFlowView {
        height,
        nodes,
        edges,
    }
}
