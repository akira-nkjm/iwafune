use crate::dto::{FlowEdgeKind, FlowNodeKind, FlowView};
use crate::power_page::power_page;
use crate::workspace::Workspace;
use std::path::Path;

fn demo_flow() -> FlowView {
    let ws =
        Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
            .unwrap();
    power_page(&ws, None).unwrap().flow
}

fn targets_of(flow: &FlowView, node: &str, row: &str) -> Vec<String> {
    let mut out: Vec<String> = flow
        .edges
        .iter()
        .filter(|e| e.source == node && e.source_row.as_deref() == Some(row))
        .map(|e| e.target.clone())
        .collect();
    out.sort();
    out
}

#[test]
fn every_edge_connects_existing_cards_and_rows() {
    let flow = demo_flow();
    let mut ids: Vec<&str> = flow.nodes.iter().map(|n| n.id.as_str()).collect();
    ids.sort();
    let before = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), before, "カードの ID が重複");
    for e in &flow.edges {
        let source = flow.nodes.iter().find(|n| n.id == e.source).unwrap();
        let target = flow.nodes.iter().find(|n| n.id == e.target).unwrap();
        assert_eq!(target.kind, FlowNodeKind::Analysis, "{}", e.id);
        match (&e.kind, &e.source_row) {
            (FlowEdgeKind::Input, Some(row)) => {
                assert!(source.rows.iter().any(|r| r.id == *row), "{}", e.id);
                let r = source.rows.iter().find(|r| r.id == *row).unwrap();
                assert!(!r.derived, "求めた値はつながりを持たない: {}", e.id);
            }
            (FlowEdgeKind::Result, None) => assert_eq!(source.kind, FlowNodeKind::Analysis),
            other => panic!("{}: {other:?}", e.id),
        }
    }
    let mut edge_ids: Vec<&str> = flow.edges.iter().map(|e| e.id.as_str()).collect();
    edge_ids.sort();
    let n = edge_ids.len();
    edge_ids.dedup();
    assert_eq!(edge_ids.len(), n, "つながりの ID が重複");
}

#[test]
fn solar_panel_and_battery_rows_go_to_the_analyses_that_read_them() {
    let flow = demo_flow();
    let gen_bal_size = [
        "analysis:power.energy_balance",
        "analysis:power.orbit_generation",
        "analysis:power.sizing",
    ];
    for attr in [
        "area",
        "cell_efficiency",
        "packing_factor",
        "eol_factor",
        "normal",
    ] {
        assert_eq!(
            targets_of(&flow, "part:SP_PX", attr),
            gen_bal_size,
            "{attr}"
        );
    }
    assert_eq!(
        targets_of(&flow, "part:BAT", "capacity"),
        ["analysis:power.energy_balance", "analysis:power.sizing"]
    );
    let sp = flow.nodes.iter().find(|n| n.id == "part:SP_PX").unwrap();
    assert_eq!(sp.kind, FlowNodeKind::Part);
    assert!(
        sp.rows
            .iter()
            .any(|r| r.id == "area" && r.display.is_some())
    );
    let power = flow
        .nodes
        .iter()
        .find(|n| n.id == "setting:analysis.power")
        .unwrap();
    assert_eq!(power.label, "analysis/power.toml");
    assert_eq!(
        targets_of(&flow, "setting:analysis.power", "dod_max"),
        ["analysis:power.energy_balance", "analysis:power.sizing"]
    );
    // モードの並びは区間ごとの行
    assert!(power.rows.iter().any(|r| r.id == "sequence.0"));
    let orbit = flow
        .nodes
        .iter()
        .find(|n| n.id == "setting:conditions.orbit")
        .unwrap();
    assert!(orbit.rows.iter().any(|r| r.id == "period" && r.derived));
}

#[test]
fn analyses_connect_by_the_results_they_use() {
    let flow = demo_flow();
    let mut results: Vec<(&str, &str)> = flow
        .edges
        .iter()
        .filter(|e| e.kind == FlowEdgeKind::Result)
        .map(|e| (e.source.as_str(), e.target.as_str()))
        .collect();
    results.sort();
    assert_eq!(
        results,
        [
            ("analysis:power.energy_balance", "analysis:power.sizing"),
            ("analysis:power.mode_load", "analysis:power.energy_balance"),
            ("analysis:power.mode_load", "analysis:power.sizing"),
            ("analysis:power.orbit_generation", "analysis:power.sizing"),
        ]
    );
    let analyses: Vec<&str> = flow
        .nodes
        .iter()
        .filter(|n| n.kind == FlowNodeKind::Analysis)
        .filter_map(|n| n.analysis.as_deref())
        .collect();
    assert_eq!(
        analyses,
        [
            "power.mode_load",
            "power.orbit_generation",
            "power.energy_balance",
            "power.sizing"
        ]
    );
}

#[test]
fn many_loads_fold_in_order_of_power() {
    let flow = demo_flow();
    let ws =
        Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
            .unwrap();
    let s = ws.snapshot().unwrap();
    let fold = flow.folds.first().expect("負荷のまとまり");
    assert!(fold.nodes.len() >= 5);
    let peak = |node: &str| {
        let id = node.trim_start_matches("part:");
        let p = s.resolved.parts.get(id).unwrap();
        let max = match p.value("power") {
            Some(fune_model::resolve::SiValue::ByState(m)) => {
                m.values().map(|v| v.value).fold(0.0, f64::max)
            }
            Some(fune_model::resolve::SiValue::Scalar(v)) => v.value,
            _ => 0.0,
        };
        max * f64::from(p.count.max(1))
    };
    for w in fold.nodes.windows(2) {
        assert!(peak(&w[0]) >= peak(&w[1]), "{} < {}", w[0], w[1]);
    }
    // 太陽電池と電池はまとまりに入らない
    assert!(
        !fold
            .nodes
            .iter()
            .any(|n| n == "part:SP_PX" || n == "part:BAT")
    );
}
