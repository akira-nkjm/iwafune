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
            targets_of(&flow, "parts:CELLS+SP_MX+SP_MY+SP_PX+SP_PY", attr),
            gen_bal_size,
            "{attr}"
        );
    }
    assert_eq!(
        targets_of(&flow, "part:BAT", "capacity"),
        ["analysis:power.energy_balance", "analysis:power.sizing"]
    );
    let sp = flow
        .nodes
        .iter()
        .find(|n| n.id == "parts:CELLS+SP_MX+SP_MY+SP_PX+SP_PY")
        .unwrap();
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
    assert_eq!(
        (power.label.as_str(), power.sub.as_deref()),
        ("電源の設定", Some("analysis/power.toml"))
    );
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
    // まとめたカードは部品の合計（1 部品のカードは補足が部品 ID）
    let peak = |node: &str| -> f64 {
        let n = flow.nodes.iter().find(|n| n.id == node).unwrap();
        let ids: Vec<String> = if n.members.is_empty() {
            n.sub.clone().into_iter().collect()
        } else {
            n.members.iter().map(|m| m.id.clone()).collect()
        };
        ids.iter()
            .map(|id| {
                let p = s.resolved.parts.get(id.as_str()).unwrap();
                let max = match p.value("power") {
                    Some(fune_model::resolve::SiValue::ByState(m)) => {
                        m.values().map(|v| v.value).fold(0.0, f64::max)
                    }
                    Some(fune_model::resolve::SiValue::Scalar(v)) => v.value,
                    _ => 0.0,
                };
                max * f64::from(p.count.max(1))
            })
            .sum()
    };
    for w in fold.nodes.windows(2) {
        assert!(peak(&w[0]) >= peak(&w[1]), "{} < {}", w[0], w[1]);
    }
    // 太陽電池と電池はまとまりに入らない
    assert!(
        !fold
            .nodes
            .iter()
            .any(|n| n.contains("SP_PX") || n == "part:BAT")
    );
}

/// 同じ役割で同じ読まれ方の部品は 1 枚にまとめ、部品ごとに違う値は範囲か「違う部品の数」で出す。
#[test]
fn parts_of_one_role_read_the_same_way_share_a_card() {
    let flow = demo_flow();
    let solar = flow
        .nodes
        .iter()
        .find(|n| n.id == "parts:CELLS+SP_MX+SP_MY+SP_PX+SP_PY")
        .unwrap();
    assert_eq!(
        (solar.label.as_str(), solar.label_en.as_str()),
        ("太陽電池パネル", "Solar panel")
    );
    assert_eq!(solar.members.len(), 5);
    let packing = solar
        .rows
        .iter()
        .find(|r| r.id == "packing_factor")
        .unwrap();
    assert!(
        packing.display.as_deref().unwrap().contains(" – "),
        "{packing:?}"
    );
    let efficiency = solar
        .rows
        .iter()
        .find(|r| r.id == "cell_efficiency")
        .unwrap();
    assert_eq!(efficiency.display.as_deref(), Some("0.295"));
    let trx = flow
        .nodes
        .iter()
        .find(|n| n.id == "parts:STX+UHF_TRX")
        .unwrap();
    let power = &trx.rows[0];
    assert_eq!((power.display.as_deref(), power.varies), (None, Some(2)));
    // 1 部品のカードは部品の名前と ID
    let bat = flow.nodes.iter().find(|n| n.id == "part:BAT").unwrap();
    assert_eq!(bat.sub.as_deref(), Some("BAT"));
    assert!(bat.members.is_empty());
}

/// 設定・ケースの行の名前は語彙の表示名（キーの綴りのままにしない）。
#[test]
fn setting_and_case_rows_use_display_names() {
    let flow = demo_flow();
    for n in flow.nodes.iter().filter(|n| n.kind != FlowNodeKind::Part) {
        assert!(!n.label.contains('/'), "{}", n.label);
        for r in &n.rows {
            assert!(
                !r.label.contains('_') && !r.label.contains('.'),
                "{}: {}",
                n.id,
                r.label
            );
        }
    }
    let orbit = flow
        .nodes
        .iter()
        .find(|n| n.id == "setting:conditions.orbit")
        .unwrap();
    assert_eq!(orbit.label, "軌道");
    assert!(
        orbit
            .rows
            .iter()
            .any(|r| r.label == "近地点引数" && r.label_en == "Argument of perigee")
    );
}

/// 解析のカードは名前付きの主な出力（2〜3 行）と判定。
#[test]
fn analysis_cards_show_named_main_outputs() {
    let flow = demo_flow();
    for n in flow
        .nodes
        .iter()
        .filter(|n| n.kind == FlowNodeKind::Analysis)
    {
        assert!((2..=3).contains(&n.rows.len()), "{}: {:?}", n.id, n.rows);
        assert!(n.rows.iter().all(|r| r.display.is_some()));
    }
    let balance = flow
        .nodes
        .iter()
        .find(|n| n.id == "analysis:power.energy_balance")
        .unwrap();
    let dod = balance.rows.iter().find(|r| r.id == "max_dod").unwrap();
    assert!(dod.display.as_deref().unwrap().contains(" / "));
    assert!(dod.verdict.is_some());
}

/// 言語で変わる値は英語の表示も持つ（モードの名前は英語では ID）。
#[test]
fn mode_names_in_outputs_follow_the_language() {
    let flow = demo_flow();
    let load = flow
        .nodes
        .iter()
        .find(|n| n.id == "analysis:power.mode_load")
        .unwrap();
    let max = load.rows.iter().find(|r| r.id == "max_mode").unwrap();
    assert!(max.display.as_deref().unwrap().contains("ダウンリンク"));
    assert!(max.display_en.as_deref().unwrap().ends_with("· DOWNLINK"));
}
