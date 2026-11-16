use crate::dto::{FlowEdgeKind, FlowNodeKind, FlowView};
use crate::workspace::Workspace;
use sat_analysis::spec::System;
use std::path::Path;

mod enum_rows;

pub(super) fn demo_flow() -> FlowView {
    let ws =
        Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
            .unwrap();
    super::system_flow(&ws, System::Power, None).unwrap()
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
    // 範囲の記号は日本語は「〜」、英語は「–」（値の整形は同じ）
    let ja = packing.display.as_deref().unwrap();
    let en = packing.display_en.as_deref().unwrap();
    assert!(ja.contains('〜') && !ja.contains('–'), "{packing:?}");
    assert_eq!(en, ja.replace('〜', " – "));
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

/// 解析のカードの行は KPI（定義の順に 3 つまで。名前は KPI の表示名、値は KPI の表示、判定は KPI の判定）。
#[test]
fn analysis_cards_show_kpis_in_their_defined_order() {
    let ws =
        Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
            .unwrap();
    for case in [None, Some("hot_imaging")] {
        let flow = crate::flow::system_flow(&ws, sat_analysis::spec::System::Power, case).unwrap();
        for n in flow
            .nodes
            .iter()
            .filter(|n| n.kind == FlowNodeKind::Analysis)
        {
            let id = n.analysis.as_deref().unwrap();
            let r = crate::analysis::run_analysis(
                &ws,
                id,
                &crate::dto::RunAnalysisRequest {
                    case: case.map(str::to_owned),
                    ..Default::default()
                },
            )
            .unwrap();
            let want: Vec<(&str, &str, Option<&str>)> = r
                .kpis
                .iter()
                .take(3)
                .map(|k| {
                    (
                        k.label.as_str(),
                        k.label_en.as_str(),
                        k.value.as_ref().map(|v| v.display.as_str()),
                    )
                })
                .collect();
            let got: Vec<(&str, &str, Option<&str>)> = n
                .rows
                .iter()
                .map(|r| (r.label.as_str(), r.label_en.as_str(), r.display.as_deref()))
                .collect();
            assert_eq!(got, want, "{id}");
            let verdicts: Vec<_> = r.kpis.iter().take(3).map(|k| k.verdict).collect();
            assert_eq!(
                n.rows.iter().map(|r| r.verdict).collect::<Vec<_>>(),
                verdicts
            );
            assert!(n.description.as_deref().is_some_and(|d| !d.is_empty()));
            assert!(n.analysis_kind.is_some());
        }
    }
    // デモ（ケース無し）の電源の解析のカードの名前と値
    let flow = demo_flow();
    let rows = |id: &str| -> Vec<(String, String)> {
        let n = flow
            .nodes
            .iter()
            .find(|n| n.id == format!("analysis:{id}"))
            .unwrap();
        n.rows
            .iter()
            .map(|r| (r.label.clone(), r.display.clone().unwrap_or_default()))
            .collect()
    };
    let names = |id: &str| rows(id).into_iter().map(|(l, _)| l).collect::<Vec<_>>();
    assert_eq!(names("power.mode_load"), ["モードの消費電力（バジェット）"]);
    assert_eq!(
        names("power.orbit_generation"),
        ["軌道平均の発電", "日照率"]
    );
    assert_eq!(
        names("power.energy_balance"),
        ["軌道平均の収支（最悪）", "最大 DoD"]
    );
    assert_eq!(
        names("power.sizing"),
        ["太陽電池の余裕", "バッテリーの余裕"]
    );
    let generation = rows("power.orbit_generation");
    assert!(generation[0].1.ends_with(" W"), "{generation:?}");
    assert!(generation[1].1.ends_with(" %"), "{generation:?}");
    let balance = flow
        .nodes
        .iter()
        .find(|n| n.id == "analysis:power.energy_balance")
        .unwrap();
    let dod = balance.rows.iter().find(|r| r.id == "max_dod").unwrap();
    assert!(dod.display.as_deref().unwrap().ends_with(" %"));
    assert!(dod.verdict.is_some());
    let load = flow
        .nodes
        .iter()
        .find(|n| n.id == "analysis:power.mode_load")
        .unwrap();
    assert_eq!(
        load.analysis_kind,
        Some(sat_analysis::spec::AnalysisKind::Steady)
    );
}
