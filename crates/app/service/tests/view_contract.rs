//! CLI と Web が共有する表示の契約（定義・参照・パス・dB の種類・判定の集約）。
use fune_service::{Workspace, analysis, dto::RunAnalysisRequest, query};
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .expect("demo")
}

#[test]
fn definitions_references_and_paths_come_from_model() {
    let ws = demo();
    let design = query::design_view(&ws).unwrap();
    let snapshot = ws.snapshot().unwrap();
    for attr in &design.defs.attributes {
        assert_eq!(attr.section, snapshot.design.attribute_section(&attr.id));
    }
    assert_eq!(design.defs.labels.role_subsystem["reaction_wheel"], "AOCS");
    let wheel = design.parts.iter().find(|p| p.id == "RW_X").unwrap();
    assert_eq!(wheel.paths["power"]["on"], "RW_X.power.on");
    assert_eq!(wheel.paths["mass"][""], "RW_X.mass");
    let bus = design.nets.iter().find(|n| n.id == "P12V").unwrap();
    assert!(
        bus.members
            .iter()
            .any(|m| m.part == "RW_X" && m.port == "PWR")
    );
    assert_eq!(bus.kind.to_string(), "power");
    let view = query::part_view(&ws, "RW_X").unwrap();
    assert_eq!(view.part.paths, wheel.paths);
}

/// カタログ品の値は、その品を選んで上書きしていない部品の値と同じ形・同じ表示（組み立ては service の 1 か所）。
#[test]
fn catalog_items_share_the_part_value_view() {
    let ws = demo();
    let design = query::design_view(&ws).unwrap();
    let item = design
        .catalog
        .iter()
        .find(|c| c.id == "local/rw_10mNms")
        .unwrap();
    assert_eq!(
        (item.scope.as_str(), item.name.as_str()),
        ("local", "rw_10mNms")
    );
    assert!(
        item.editable,
        "fune.toml の [[catalog]] path の品は書き換えられる"
    );
    assert_eq!(item.users, ["RW_X", "RW_Y", "RW_Z"]);
    assert_eq!(item.paths["power"]["on"], "local/rw_10mNms.power.on");
    assert_eq!(item.paths["mass"][""], "local/rw_10mNms.mass");
    assert_eq!(item.states, ["off", "idle", "on", "peak"]);
    let wheel = design.parts.iter().find(|p| p.id == "RW_Y").unwrap();
    for attr in ["power", "max_torque", "temp_op", "size"] {
        let (a, b) = (&item.values[attr], &wheel.values[attr]);
        assert_eq!(a.display, b.display, "{attr}");
        assert_eq!(a.origin, fune_service::dto::Origin::Catalog);
        assert_eq!(a.evidence, b.evidence, "{attr}: 品の source が既定の出典");
        assert_eq!(a.file.as_deref(), Some("catalog/rw_10mNms.toml"));
    }
    // 部品が上書きした値（RW_X.mass）は品の値ではない
    let x = design.parts.iter().find(|p| p.id == "RW_X").unwrap();
    assert_ne!(item.values["mass"].display, x.values["mass"].display);
}

#[test]
fn decibel_metadata_does_not_change_json_values() {
    let result =
        analysis::run_analysis(&demo(), "comm.link_budget", &RunAnalysisRequest::default())
            .unwrap();
    for (field, kind) in [
        ("eirp_dbw", "DecibelWatt"),
        ("fspl_db", "Decibel"),
        ("g_over_t_dbk", "DecibelPerKelvin"),
        ("c_over_n0_dbhz", "DecibelHertz"),
    ] {
        assert_eq!(result.dims[&format!("/links/0/{field}")], kind);
        assert!(result.output.as_ref().unwrap()["links"][0][field].is_number());
    }
    assert!(!result.checks.is_empty());
    assert_eq!(
        result.verdict,
        fune_service::dto::Verdict::worst(result.checks.iter().map(|c| c.verdict))
    );
}

#[test]
fn mode_power_budgets_match_analysis_and_shared_rollup() {
    use fune_service::{
        budget,
        dto::{BudgetKind, BudgetRequest},
    };
    use sat_analysis::budget::{BudgetItem, budget_rollup};
    let ws = demo();
    let view = budget::budget(&ws, BudgetKind::Power, &BudgetRequest::default()).unwrap();
    let result =
        analysis::run_analysis(&ws, "power.mode_load", &RunAnalysisRequest::default()).unwrap();
    let out: sat_analysis::power::mode_load::ModeLoadOutput =
        serde_json::from_value(result.output.unwrap()).unwrap();
    assert_eq!(view.modes.len(), out.modes.len());
    // 全体の行（ケースのモードか、いちばん重いモード）は、そのモードの行と同じ値・同じ判定
    let main = view
        .modes
        .iter()
        .find(|r| Some(&r.node) == view.mode.as_ref())
        .unwrap();
    assert!((main.with_growth.si - view.total.si).abs() < 1e-12);
    assert_eq!(main.verdict, view.verdict);
    for mode in &out.modes {
        let row = view.modes.iter().find(|r| r.node == mode.id).unwrap();
        let items: Vec<_> = mode
            .parts
            .iter()
            .filter_map(|p| {
                Some(BudgetItem {
                    id: p.part.clone(),
                    group: p.group.clone(),
                    value: p.mean?.value(),
                    count: 1,
                    growth: p.growth_margin.value(),
                })
            })
            .collect();
        let expected = budget_rollup(
            &items,
            view.limit.as_ref().map(|q| q.si),
            view.system_margin,
        )
        .unwrap();
        assert!((row.with_growth.si - mode.total_with_growth.value()).abs() < 1e-10);
        assert_eq!(row.margin.as_ref().map(|q| q.si), expected.margin);
        assert_eq!(row.allocated.as_ref().map(|q| q.si), expected.allowed);
        assert_eq!(Some(row.verdict), expected.verdict);
        assert!(!row.margin.as_ref().unwrap().display.is_empty());
    }
}
