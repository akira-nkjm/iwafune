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
    assert_eq!(
        design.defs.labels.role_system[&fune_model::builtin::Role::ReactionWheel].as_deref(),
        Some("aocs")
    );
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
    let view = query::part_view(&ws, "RW_X", &Default::default()).unwrap();
    assert_eq!(view.part.paths, wheel.paths);
    // 部品のフィールドのパス（名前は fune_model::design::PartFieldName::NAMES、記法は ParamPath::Field）
    let mut fields = fune_model::design::PartFieldName::NAMES.to_vec();
    fields.sort_unstable();
    let names: Vec<&str> = wheel.field_paths.keys().map(String::as_str).collect();
    assert_eq!(names, fields);
    assert_eq!(wheel.field_paths["count"], "RW_X.count");
    assert_eq!(wheel.field_paths["candidates"], "RW_X.candidates");
}

/// 下書き（書き込まない `ops`）を当てた部品の表示: 同じ編集で定義を作る属性のパスも、定義のある属性と同じ記法で入る。
#[test]
fn draft_part_view_has_paths_for_new_attributes() {
    let ws = demo();
    let def = fune_model::builtin::attributes()["heat_fraction"].clone();
    let create = fune_service::dto::Op::CreateAttribute {
        def: fune_model::design::AttributeDef {
            id: fune_model::Id::new("tank_pressure"),
            by_state: true,
            ..def
        },
    };
    let req = fune_service::dto::PartViewRequest {
        ops: vec![create],
        configuration: None,
    };
    let draft = query::part_view(&ws, "UHF_TRX", &req).unwrap();
    assert_eq!(
        draft.part.paths["tank_pressure"]["tx"],
        "UHF_TRX.tank_pressure.tx"
    );
    assert_eq!(
        draft.part.paths["tank_pressure"][""],
        "UHF_TRX.tank_pressure"
    );
    // 書き込んでいない
    let saved = query::part_view(&ws, "UHF_TRX", &Default::default()).unwrap();
    assert!(!saved.part.paths.contains_key("tank_pressure"));
}

/// ネットとハーネスの端・戻りの区分・部品の置き場所・設定の区分は構造で返す（Web は文字列を分解しない）。
#[test]
fn ends_places_and_setting_groups_are_structured() {
    use fune_model::path::SettingScope;
    use fune_service::dto::System;
    let ws = demo();
    let design = query::design_view(&ws).unwrap();
    let h = design
        .harnesses
        .iter()
        .find(|h| h.id == "H_PCDU_RWX")
        .unwrap();
    assert_eq!((h.from.part.as_str(), h.to.part.as_str()), ("PCDU", "RW_X"));
    // 部品のハーネスは端の部品で選ぶ
    let view = query::part_view(&ws, "RW_X", &Default::default()).unwrap();
    assert_eq!(view.harnesses, ["H_PCDU_RWX"]);
    assert!(
        query::part_view(&ws, "RW_Y", &Default::default())
            .unwrap()
            .harnesses
            .is_empty()
    );
    // デモに戻り（GND）のネットは無い
    assert!(design.nets.iter().all(|n| !n.is_return));
    let cells = design.parts.iter().find(|p| p.id == "CELL_MOD_1").unwrap();
    assert_eq!(cells.dir, ["power", "SAP_PY"]);
    for dir in cells.dir.iter() {
        assert!(design.tree.iter().any(|n| &n.id == dir), "{dir}");
    }
    let setting = |key: &str| design.settings.iter().find(|s| s.key == key).unwrap();
    let altitude = setting("conditions.orbit.altitude");
    assert_eq!(
        (
            altitude.scope,
            altitude.group.as_str(),
            altitude.field.as_str()
        ),
        (SettingScope::Conditions, "conditions.orbit", "altitude")
    );
    assert_eq!(altitude.system, None);
    assert_eq!(
        setting("analysis.power.initial_soc").system,
        Some(System::Power)
    );
    assert_eq!(
        setting("budget.mass.system_margin").system,
        Some(System::Structure)
    );
    assert_eq!(
        setting("budget.data.system_margin").system,
        Some(System::Cdh)
    );
    let policy = design
        .settings
        .iter()
        .find(|s| s.key.starts_with("budget.policy."))
        .unwrap();
    assert_eq!((policy.scope, policy.system), (SettingScope::Budget, None));
}

/// 系とバジェットの種類の一覧と並びは、model の列挙から（Web は一覧を手で書かない）。
#[test]
fn defs_list_systems_and_budget_kinds_from_the_model() {
    use fune_model::design::{AnalysisSystem, BudgetKind};
    use fune_service::dto::System;
    let design = query::design_view(&demo()).unwrap();
    let systems: Vec<AnalysisSystem> = design
        .defs
        .systems
        .iter()
        .map(|s| fune_projection::system::from_analysis(*s))
        .collect();
    assert_eq!(systems, AnalysisSystem::ALL);
    assert_eq!(design.defs.budget_kinds, BudgetKind::ALL);
    assert!(design.defs.systems.contains(&System::Payload));
    assert_eq!(design.defs.labels.section["aocs"].label, "AOCS");
    assert_eq!(
        design.defs.labels.section["payload"].label,
        "ミッション機器"
    );
    assert!(
        !design
            .settings
            .iter()
            .any(|s| s.system == Some(System::Payload))
    );
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
        "an item from a [[catalog]] path in fune.toml can be rewritten"
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
        assert_eq!(
            a.evidence, b.evidence,
            "{attr}: the item source is the default source"
        );
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
    let main = view.modes.iter().find(|r| r.node == view.mode).unwrap();
    let total = view.total.as_ref().unwrap();
    assert!((main.with_growth.si - total.si).abs() < 1e-12);
    assert_eq!(Some(main.verdict), view.verdict);
    assert!(view.missing.is_empty());
    for mode in &out.modes {
        let row = view
            .modes
            .iter()
            .find(|r| r.node.as_deref() == Some(mode.id.as_str()))
            .unwrap();
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
            view.system_margin.si,
        )
        .unwrap();
        assert!((row.with_growth.si - mode.total_with_growth.value()).abs() < 1e-10);
        assert_eq!(row.margin.as_ref().map(|q| q.si), expected.margin);
        assert_eq!(row.allocated.as_ref().map(|q| q.si), expected.allowed);
        assert_eq!(Some(row.verdict), expected.verdict);
        assert!(!row.margin.as_ref().unwrap().display.is_empty());
    }
}

/// 無いものは null で返す（空文字や日本語の文言で埋めない）。未設定の値・種別・成熟度・モード表のセルの割合。
#[test]
fn absent_values_are_structured_not_text() {
    use fune_model::design::Maturity;
    use fune_service::dto::ValueData;
    let design = query::design_view(&demo()).unwrap();
    let rw = design.parts.iter().find(|p| p.id == "RW_X").unwrap();
    assert_eq!(rw.kind.as_deref(), Some("reaction_wheel"));
    let bat = design.parts.iter().find(|p| p.id == "BAT").unwrap();
    assert_eq!(bat.maturity, Some(Maturity::OffTheShelf));
    // 未設定の値は `unset` と `display: null`（「未設定」の文言は CLI と Web が作る）
    let mut unset = 0;
    for part in &design.parts {
        for v in part.values.values() {
            let is_unset = matches!(v.value, ValueData::Unset);
            assert_eq!(v.display.is_none(), is_unset, "{}.{}", part.id, v.attr);
            unset += usize::from(is_unset);
        }
    }
    assert!(unset > 0);
    for s in &design.settings {
        assert_eq!(s.display.is_none(), matches!(s.value, ValueData::Unset));
    }
    // モード表のセルは状態（`cells`）と割合の表示（`shares`）の構造。割合の無いセルは `shares` に無い
    let trx = design
        .modes
        .rows
        .iter()
        .find(|r| r.part == "UHF_TRX")
        .unwrap();
    assert_eq!(trx.cells["DOWNLINK"].state, "rx");
    assert_eq!(trx.shares["DOWNLINK"]["tx"].display, "10.0 %");
    assert!((trx.shares["DOWNLINK"]["tx"].si - 0.1).abs() < 1e-12);
    assert!(!trx.shares.contains_key("MISSION"));
    assert_eq!(trx.kind.as_deref(), Some("transceiver"));
}

/// 系に属さず部品の配分も無い部品は「その他」の行（`node`・`label` が null）にまとめる。ID の欄に文言を入れない。
/// 合計と、配分の行の値は変わらない（「その他」の行に足されるだけ）。
#[test]
fn budget_puts_parts_without_a_group_in_one_other_row() {
    use fune_service::{
        budget,
        dto::{BudgetKind, BudgetRequest},
    };
    let ws = demo();
    let saved = budget::budget(&ws, BudgetKind::Mass, &BudgetRequest::default()).unwrap();
    assert!(
        saved
            .rows
            .iter()
            .all(|r| r.node.is_some() && r.label.is_some())
    );
    let ops = serde_json::from_value(serde_json::json!([
        { "op": "create_part", "id": "LOOSE_A", "kind": "component" },
        { "op": "create_part", "id": "LOOSE_B", "kind": "component" },
        { "op": "set_value", "part": "LOOSE_A", "attr": "mass",
          "value": { "type": "scalar", "data": { "value": 100.0, "unit": "g" } } },
        { "op": "set_value", "part": "LOOSE_B", "attr": "mass",
          "value": { "type": "scalar", "data": { "value": 50.0, "unit": "g" } } },
    ]))
    .unwrap();
    let req = BudgetRequest { case: None, ops };
    let draft = budget::budget(&ws, BudgetKind::Mass, &req).unwrap();
    let others: Vec<_> = draft.rows.iter().filter(|r| r.node.is_none()).collect();
    assert_eq!(others.len(), 1);
    assert_eq!(others[0].label, None);
    assert!((others[0].current.si - 0.15).abs() < 1e-12);
    // 「その他」は最後の行
    assert!(draft.rows.last().unwrap().node.is_none());
    assert_eq!(draft.rows.len(), saved.rows.len() + 1);
    for (a, b) in saved.rows.iter().zip(&draft.rows) {
        assert_eq!(a.node, b.node);
        assert_eq!(a.with_growth.si, b.with_growth.si);
    }
    let total = |v: &fune_service::dto::BudgetView| v.total.as_ref().unwrap().si;
    assert!(total(&draft) > total(&saved));
}

/// 役割で決まる系、種類ごとの配分、上限（要求）が無いときを同じ集計で扱う。
#[test]
fn budget_system_rows_and_optional_limits() {
    use fune_service::{
        budget,
        dto::{BudgetKind, BudgetRequest, Op, System},
    };
    let ws = demo();
    let saved = budget::budget(&ws, BudgetKind::Mass, &BudgetRequest::default()).unwrap();
    let ops = serde_json::from_value(serde_json::json!([
        { "op": "move_part", "id": "RW_X", "to": null },
        { "op": "remove_requirement", "id": "R_MASS" },
        { "op": "set_setting", "path": "budget.power.alloc.RW_X",
          "value": { "type": "quantity", "value": { "value": 1.0, "unit": "W" } } }
    ]))
    .unwrap();
    let request = BudgetRequest { case: None, ops };
    let view = budget::budget(&ws, BudgetKind::Mass, &request).unwrap();
    assert!(view.limit.is_none() && view.verdict.is_none() && view.margin.is_none());
    assert_eq!(
        view.total.as_ref().unwrap().si,
        saved.total.as_ref().unwrap().si
    );
    assert!(view.rows.iter().all(|r| r.node.as_deref() != Some("RW_X")));
    let aocs = view
        .rows
        .iter()
        .find(|r| r.node.as_deref() == Some("aocs"))
        .unwrap();
    let before = saved.rows.iter().find(|r| r.node == aocs.node).unwrap();
    assert_eq!(aocs.current.si, before.current.si);
    assert_eq!(aocs.system, Some(System::Aocs));
    assert_eq!(aocs.label.as_deref(), Some("AOCS"));
    assert_eq!(aocs.label_en.as_deref(), Some("AOCS"));
    let mut explicit = request;
    explicit.ops.push(
        serde_json::from_value::<Op>(serde_json::json!({
            "op": "set_setting", "path": "budget.mass.alloc.RW_X",
            "value": { "type": "quantity", "value": { "value": 0.3, "unit": "kg" } }
        }))
        .unwrap(),
    );
    let view = budget::budget(&ws, BudgetKind::Mass, &explicit).unwrap();
    let wheel = view
        .rows
        .iter()
        .find(|r| r.node.as_deref() == Some("RW_X"))
        .unwrap();
    assert!((wheel.current.si - 0.152).abs() < 1e-12);
    assert_eq!(wheel.system, Some(System::Aocs));
}
