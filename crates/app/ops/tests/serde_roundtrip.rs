//! すべての Op の種類が JSON（HTTP・`fune apply ops.json`）を往復しても変わらないこと。
//! 中身の `id` が落ちるような型の誤りを捕まえる。Op の種類が増えたら `name` の match がコンパイルエラーになる。

use fune_model::Id;
use fune_model::builtin;
use fune_model::design::{
    Case, CaseField, CatalogField, Distribution, Harness, Maturity, NetField, PartField, Pose,
    Source, SourceKind, UncertaintySide,
};
use fune_model::value::{
    Authored, Provenance, Quantity, Quaternion, Scalar, SettingValue, Value, Vector3,
};
use fune_ops::{ModeCell, Op, ViewNodePos};
use std::collections::BTreeMap;

#[path = "serde_roundtrip/samples.rs"]
mod samples;
use samples::{maneuver, net, phase, requirement};

/// 種類の名前（ワイルドカードを使わない。種類が増えたら見本を足す）。
fn name(op: &Op) -> &'static str {
    match op {
        Op::CreatePart { .. } => "CreatePart",
        Op::RemovePart { .. } => "RemovePart",
        Op::RenamePart { .. } => "RenamePart",
        Op::RenameGroup { .. } => "RenameGroup",
        Op::MovePart { .. } => "MovePart",
        Op::SetValue { .. } => "SetValue",
        Op::UnsetValue { .. } => "UnsetValue",
        Op::SetField { .. } => "SetField",
        Op::Place { .. } => "Place",
        Op::SetPort { .. } => "SetPort",
        Op::SetMountPoint { .. } => "SetMountPoint",
        Op::CreateCatalogItem { .. } => "CreateCatalogItem",
        Op::SetCatalogValue { .. } => "SetCatalogValue",
        Op::UnsetCatalogValue { .. } => "UnsetCatalogValue",
        Op::SetCatalogField { .. } => "SetCatalogField",
        Op::RemoveCatalogItem { .. } => "RemoveCatalogItem",
        Op::RenameCatalogItem { .. } => "RenameCatalogItem",
        Op::MoveCatalogItem { .. } => "MoveCatalogItem",
        Op::AddCatalogSource { .. } => "AddCatalogSource",
        Op::RemoveCatalogSource { .. } => "RemoveCatalogSource",
        Op::CreateAttribute { .. } => "CreateAttribute",
        Op::CreateKind { .. } => "CreateKind",
        Op::UpdateAttribute { .. } => "UpdateAttribute",
        Op::UpdateKind { .. } => "UpdateKind",
        Op::DeleteAttribute { .. } => "DeleteAttribute",
        Op::DeleteKind { .. } => "DeleteKind",
        Op::RenameAttribute { .. } => "RenameAttribute",
        Op::RenameKind { .. } => "RenameKind",
        Op::CreateMode { .. } => "CreateMode",
        Op::RemoveMode { .. } => "RemoveMode",
        Op::RenameMode { .. } => "RenameMode",
        Op::MoveMode { .. } => "MoveMode",
        Op::SetModeDuration { .. } => "SetModeDuration",
        Op::SetModeTiming { .. } => "SetModeTiming",
        Op::SetSequence { .. } => "SetSequence",
        Op::RemoveSequence { .. } => "RemoveSequence",
        Op::Acknowledge { .. } => "Acknowledge",
        Op::Unacknowledge { .. } => "Unacknowledge",
        Op::SetModeStates { .. } => "SetModeStates",
        Op::CreateNet { .. } => "CreateNet",
        Op::UpdateNet { .. } => "UpdateNet",
        Op::SetNetField { .. } => "SetNetField",
        Op::RemoveNet { .. } => "RemoveNet",
        Op::SetCase { .. } => "SetCase",
        Op::RemoveCase { .. } => "RemoveCase",
        Op::SetSource { .. } => "SetSource",
        Op::RemoveSource { .. } => "RemoveSource",
        Op::MoveSource { .. } => "MoveSource",
        Op::SetHarness { .. } => "SetHarness",
        Op::RemoveHarness { .. } => "RemoveHarness",
        Op::CreateManeuver { .. } => "CreateManeuver",
        Op::SetManeuver { .. } => "SetManeuver",
        Op::RemoveManeuver { .. } => "RemoveManeuver",
        Op::CreateGroundStation { .. } => "CreateGroundStation",
        Op::RemoveGroundStation { .. } => "RemoveGroundStation",
        Op::RenameGroundStation { .. } => "RenameGroundStation",
        Op::CreatePhase { .. } => "CreatePhase",
        Op::SetPhase { .. } => "SetPhase",
        Op::RemovePhase { .. } => "RemovePhase",
        Op::RenamePhase { .. } => "RenamePhase",
        Op::SetPhaseField { .. } => "SetPhaseField",
        Op::CopyPhaseOrbit { .. } => "CopyPhaseOrbit",
        Op::UnsetPhaseOrbit { .. } => "UnsetPhaseOrbit",
        Op::UnsetPhaseLaw { .. } => "UnsetPhaseLaw",
        Op::SetCasePhase { .. } => "SetCasePhase",
        Op::SetCaseEpoch { .. } => "SetCaseEpoch",
        Op::SetCaseField { .. } => "SetCaseField",
        Op::CreateConfiguration { .. } => "CreateConfiguration",
        Op::SetConfiguration { .. } => "SetConfiguration",
        Op::RemoveConfiguration { .. } => "RemoveConfiguration",
        Op::RenameConfiguration { .. } => "RenameConfiguration",
        Op::UnsetConfigurationPose { .. } => "UnsetConfigurationPose",
        Op::RenameNet { .. } => "RenameNet",
        Op::RenameHarness { .. } => "RenameHarness",
        Op::RenameCase { .. } => "RenameCase",
        Op::RenameSource { .. } => "RenameSource",
        Op::RenameManeuver { .. } => "RenameManeuver",
        Op::CreateCommLink { .. } => "CreateCommLink",
        Op::RemoveCommLink { .. } => "RemoveCommLink",
        Op::RenameCommLink { .. } => "RenameCommLink",
        Op::SetProjectField { .. } => "SetProjectField",
        Op::AddLocalCatalog { .. } => "AddLocalCatalog",
        Op::CreateRequirement { .. } => "CreateRequirement",
        Op::SetRequirement { .. } => "SetRequirement",
        Op::RemoveRequirement { .. } => "RemoveRequirement",
        Op::SetViewLayout { .. } => "SetViewLayout",
        Op::SetViewEdge { .. } => "SetViewEdge",
        Op::UnsetViewEdge { .. } => "UnsetViewEdge",
        Op::ResetView { .. } => "ResetView",
        Op::SetSetting { .. } => "SetSetting",
    }
}

fn mm(x: f64, y: f64, z: f64) -> Vector3 {
    Vector3 {
        x,
        y,
        z,
        unit: Some("mm".into()),
        frame: None,
    }
}

fn from() -> Option<Provenance> {
    Some(Provenance {
        source: Some("em_test_2026_10".into()),
        at: Some("TR-EM 表 4".into()),
        note: Some("EM 実測".into()),
    })
}

fn by_state() -> Value {
    let cell = |v: f64| Authored {
        value: Scalar::new(v, "W"),
        from: None,
    };
    Value::ByState(BTreeMap::from([
        ("off".to_string(), cell(0.0)),
        ("on".to_string(), cell(1.2)),
    ]))
}

fn part_ops() -> Vec<Op> {
    vec![
        Op::CreatePart {
            id: "RW_S".into(),
            group: Some("aocs".into()),
            group_under: vec![],
            catalog: Some("local/rw_10mNms".into()),
            kind: None,
            count: Some(2),
            label: Some("予備".into()),
            mode_state: Some("off".into()),
        },
        Op::RemovePart { id: "RW_S".into() },
        Op::RenamePart {
            id: "RW_S".into(),
            to: "RW_T".into(),
        },
        Op::RenameGroup {
            id: "aocs".into(),
            to: "ADCS".into(),
        },
        Op::MovePart {
            id: "RW_S".into(),
            to: Some("power".into()),
            under: vec![],
        },
        Op::SetValue {
            part: "RW_X".into(),
            attr: "mass".into(),
            state: None,
            value: Value::Scalar(Scalar {
                value: 0.62,
                min: Some(0.6),
                max: None,
                unit: "kg".into(),
            }),
            from: from(),
        },
        Op::SetValue {
            part: "RW_X".into(),
            attr: "power".into(),
            state: None,
            value: by_state(),
            from: None,
        },
        Op::UnsetValue {
            part: "RW_X".into(),
            attr: "power".into(),
            state: Some("on".into()),
        },
        Op::SetField {
            part: "RW_X".into(),
            field: PartField::Maturity(Some(Maturity::New)),
        },
        Op::Place {
            part: "RW_X".into(),
            on: Some("PANEL_MZ".into()),
            point: Some("RW_SLOT_X".into()),
            position: Some(mm(0.0, 0.0, 12.0)),
            orientation: Some(Quaternion::IDENTITY),
            configuration: None,
        },
        Op::Place {
            part: "SAP_PY".into(),
            on: None,
            point: None,
            position: Some(mm(2.6, -1.5, 0.0)),
            orientation: Some(Quaternion::IDENTITY),
            configuration: Some("STOWED".into()),
        },
        Op::SetPort {
            part: "RW_X".into(),
            port: "PWR".into(),
            net: Some("P12V".into()),
        },
        Op::SetMountPoint {
            part: "PANEL_PZ".into(),
            point: "ANT".into(),
            pose: Some(Pose {
                position: Some(mm(1.0, 2.0, 3.0)),
                orientation: None,
            }),
        },
    ]
}

fn catalog_ops() -> Vec<Op> {
    vec![
        Op::CreateCatalogItem {
            id: "local/rw_30mNms".into(),
            kind: "reaction_wheel".into(),
            label: Some("RW 30".into()),
            label_en: Some("RW 30".into()),
            maturity: Some(Maturity::OffTheShelf),
            copy_from_part: Some("RW_X".into()),
            dir: None,
        },
        Op::SetCatalogValue {
            item: "local/rw_10mNms".into(),
            attr: "power".into(),
            state: Some("on".into()),
            value: Value::Scalar(Scalar::new(0.7, "W")),
            from: from(),
        },
        Op::UnsetCatalogValue {
            item: "local/rw_10mNms".into(),
            attr: "hk_rate".into(),
            state: None,
        },
        Op::SetCatalogField {
            item: "local/rw_10mNms".into(),
            field: CatalogField::PartNumber(Some("RW-10B".into())),
        },
        Op::SetCatalogField {
            item: "local/rw_10mNms".into(),
            field: CatalogField::Tags(vec!["flight_proven".into(), "国産".into()]),
        },
        Op::RemoveCatalogItem {
            id: "local/rw_30mNms".into(),
        },
        Op::RenameCatalogItem {
            from: "local/rw_10mNms".into(),
            to: "local/rw_10mNms_b".into(),
        },
        Op::MoveCatalogItem {
            item: "local/rw_10mNms".into(),
            dir: "vendor_a/rw".into(),
        },
        serde_json::from_str(r#"{"op":"add_catalog_source","name":"a","git":"g"}"#).unwrap(),
        Op::RemoveCatalogSource { name: "lab".into() },
    ]
}

fn other_ops() -> Vec<Op> {
    let mut attr = builtin::attributes()["max_torque"].clone();
    attr.id = Id::new("max_torque_2");
    let mut kind = builtin::kinds()["heater"].clone();
    kind.id = Id::new("film_heater");
    let case = Case {
        id: Id::new("launch_day"),
        label: Some("打ち上げ当日".into()),
        description: None,
        mode: Some(Id::new("SAFE")),
        phase: Some(Id::new("LEOP")),
        side: Some(UncertaintySide::Max),
        thermal_case: Some("cold".into()),
        beta: Some(Quantity::new(20.0, "deg")),
        orbits: Some(3),
        sun_distance: None,
        epoch: None,
    };
    let source = Source {
        id: Id::new("tvac"),
        kind: SourceKind::Measured,
        label: Some("TVAC".into()),
        reference: Some("TR-1".into()),
        revision: Some("Rev. B".into()),
        retrieved: Some("2027-01-21".into()),
        sha256: Some("ab".repeat(32)),
        distribution: Some(Distribution::OnRequest),
        date: Some("2027-01-20".into()),
        by: Some("test-team".into()),
        note: None,
    };
    let harness = Harness {
        id: Id::new("H_NEW"),
        label: None,
        from: "PCDU.OUT2".parse().unwrap(),
        to: "CAM.PWR".parse().unwrap(),
        nets: vec![Id::new("P5V")],
        wire: Some("local/awg24_ptfe".into()),
        cores: Some(2),
        length: Some(Quantity::new(0.3, "m")),
    };
    vec![
        Op::CreateAttribute { def: attr.clone() },
        Op::UpdateAttribute { def: attr },
        Op::DeleteAttribute {
            id: "max_torque_2".into(),
        },
        Op::CreateKind { def: kind.clone() },
        Op::UpdateKind { def: kind },
        Op::DeleteKind {
            id: "film_heater".into(),
        },
        Op::RenameAttribute {
            id: "max_torque_2".into(),
            to: "max_torque_3".into(),
        },
        Op::RenameKind {
            id: "film_heater".into(),
            to: "foil_heater".into(),
        },
        Op::MoveMode {
            id: "SAFE".into(),
            position: fune_ops::ModePosition::Before("NOMINAL".into()),
        },
        Op::SetModeDuration {
            mode: "NOMINAL".into(),
            duration: Some(Quantity::new(90.0, "min")),
        },
        Op::SetModeTiming {
            mode: "MISSION".into(),
            when: Some(fune_model::design::ModeWhen::Sunlit),
            start: Some(fune_model::design::StartRef {
                from: fune_model::design::StartFrom::EclipseExit,
                offset: Some(Quantity::new(5.0, "min")),
            }),
        },
        Op::SetSequence {
            name: "imaging_day".into(),
            anchor: Some(fune_model::design::StartRef {
                from: fune_model::design::StartFrom::EclipseEntry,
                offset: None,
            }),
            segments: vec![fune_model::design::ModeSegment {
                mode: "NOMINAL".into(),
                duration: Quantity::new(76.0, "min"),
            }],
        },
        Op::RemoveSequence {
            name: "imaging_day".into(),
        },
        Op::Acknowledge {
            finding: fune_model::design::FindingKind::Interference,
            first: "SAP_PY".into(),
            second: "SP_PY".into(),
            configuration: Some("STOWED".into()),
            reason: "Hinge edge".into(),
        },
        Op::Unacknowledge {
            finding: fune_model::design::FindingKind::ConeClearance,
            first: "SS_PX".into(),
            second: "SAP_PY".into(),
            configuration: None,
        },
        Op::CreateMode {
            id: "DEPLOY".into(),
            label: Some("展開".into()),
            description: None,
            copy_from: Some("SAFE".into()),
        },
        Op::RemoveMode {
            id: "DEPLOY".into(),
        },
        Op::RenameMode {
            id: "DEPLOY".into(),
            to: "DEPLOYMENT".into(),
        },
        Op::SetModeStates {
            cells: vec![ModeCell {
                mode: "NOMINAL".into(),
                part: "UHF_TRX".into(),
                state: Some("rx".into()),
                duty: BTreeMap::from([("tx".to_string(), 0.05)]),
                active: None,
                spare_state: None,
                eclipse: false,
            }],
        },
        Op::CreateNet { net: net() },
        Op::UpdateNet { net: net() },
        Op::SetNetField {
            net: "P5V".into(),
            field: NetField::Rail(Some(Quantity::new(5.1, "V"))),
        },
        Op::SetNetField {
            net: "P5V".into(),
            field: NetField::Limit(None),
        },
        Op::RemoveNet {
            id: "P5V_PL".into(),
        },
        Op::SetCase { case },
        Op::RemoveCase {
            id: "launch_day".into(),
        },
        Op::SetSource {
            source,
            dir: Some("datasheets".into()),
        },
        Op::RemoveSource { id: "tvac".into() },
        Op::MoveSource {
            id: "tvac".into(),
            dir: ".".into(),
        },
        Op::SetHarness { harness },
        Op::RemoveHarness { id: "H_NEW".into() },
        Op::CreateManeuver {
            maneuver: maneuver(),
        },
        Op::SetManeuver {
            maneuver: maneuver(),
        },
        Op::RemoveManeuver { id: "RAISE".into() },
        Op::CreateGroundStation {
            id: "SVALBARD".into(),
            label: Some("Svalbard".into()),
            latitude: Some(Quantity::new(78.2, "deg")),
            longitude: None,
            altitude: None,
            min_elevation: None,
        },
        Op::RemoveGroundStation {
            id: "SVALBARD".into(),
        },
        Op::RenameGroundStation {
            id: "SVALBARD".into(),
            to: "SVALBARD2".into(),
        },
        Op::CreateConfiguration {
            id: "STOWED".into(),
            label: Some("Stowed".into()),
            description: Some("Paddle folded onto the +X face".into()),
        },
        Op::SetConfiguration {
            id: "STOWED".into(),
            label: None,
            description: None,
        },
        Op::RemoveConfiguration {
            id: "STOWED".into(),
        },
        Op::RenameConfiguration {
            id: "STOWED".into(),
            to: "FOLDED".into(),
        },
        Op::UnsetConfigurationPose {
            configuration: "STOWED".into(),
            part: "SAP_PY".into(),
        },
        Op::CreatePhase { phase: phase() },
        Op::SetPhase { phase: phase() },
        Op::RemovePhase { id: "LEOP".into() },
        Op::RenamePhase {
            id: "LEOP".into(),
            to: "COMMISSIONING".into(),
        },
        Op::SetPhaseField {
            phase: "LEOP".into(),
            path: "orbit.altitude".into(),
            value: Some(SettingValue::Quantity(Quantity::new(450.0, "km"))),
        },
        Op::CopyPhaseOrbit {
            phase: "LEOP".into(),
        },
        Op::UnsetPhaseOrbit {
            phase: "LEOP".into(),
        },
        Op::UnsetPhaseLaw {
            phase: "LEOP".into(),
            mode: "NOMINAL".into(),
        },
        Op::SetCasePhase {
            case: "eclipse_worst".into(),
            phase: None,
        },
        Op::SetCaseField {
            case: "eclipse_worst".into(),
            field: CaseField::Beta(Some(Quantity::new(60.0, "deg"))),
        },
        Op::RenameNet {
            id: "P5V".into(),
            to: "P5V_MAIN".into(),
        },
        Op::RenameHarness {
            id: "H_OBC_CAM".into(),
            to: "H_OBC_CAMERA".into(),
        },
        Op::RenameCase {
            id: "hot_imaging".into(),
            to: "hot_img".into(),
        },
        Op::RenameSource {
            id: "sys_judgement".into(),
            to: "sys_judge".into(),
        },
        Op::RenameManeuver {
            id: "RAISE".into(),
            to: "M_RAISE".into(),
        },
        Op::CreateCommLink {
            id: "x_down".into(),
            direction: fune_model::design::LinkDirection::Down,
            part: "STX".into(),
            band: "s_band".into(),
        },
        Op::RemoveCommLink {
            id: "x_down".into(),
        },
        Op::RenameCommLink {
            id: "x_down".into(),
            to: "x_dl".into(),
        },
        Op::SetProjectField {
            field: fune_model::design::ProjectField::Name("demo".into()),
        },
        Op::SetProjectField {
            field: fune_model::design::ProjectField::Label(None),
        },
        Op::AddLocalCatalog {
            name: "vendor".into(),
            path: "vendor_parts".into(),
        },
        Op::CreateRequirement {
            requirement: requirement(),
        },
        Op::SetRequirement {
            requirement: requirement(),
        },
        Op::RemoveRequirement {
            id: "R_MASS".into(),
        },
        Op::SetViewEdge {
            view: "electrical".into(),
            key: "P12V:PCDU.OUT1->RW_X.PWR".into(),
            edge: fune_model::design::ViewEdge {
                points: vec![[-20, 0], [20, 0], [20, 80]],
                source_side: Some(fune_model::design::ViewPortSide::Right),
                target_side: None,
                source_offset: None,
                target_offset: None,
            },
        },
        Op::UnsetViewEdge {
            view: "electrical".into(),
            key: "P12V:PCDU.OUT1->RW_X.PWR".into(),
        },
        Op::ResetView {
            view: "electrical".into(),
        },
        Op::SetViewLayout {
            view: "electrical".into(),
            nodes: vec![ViewNodePos {
                id: "RW_X".into(),
                x: 420,
                y: 120,
                w: Some(160),
                h: None,
            }],
            remove: vec!["OLD".into()],
        },
        Op::SetSetting {
            path: "conditions.orbit.altitude".into(),
            value: Some(SettingValue::Quantity(Quantity::new(550.0, "km"))),
        },
        Op::SetSetting {
            path: "analysis.power.initial_soc".into(),
            value: None,
        },
    ]
}

#[test]
fn every_op_round_trips_through_json() {
    let ops: Vec<Op> = [part_ops(), catalog_ops(), other_ops()].concat();
    let mut seen: Vec<&str> = ops.iter().map(name).collect();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 89, "some op kinds have no sample: {seen:?}");
    for op in &ops {
        let json = serde_json::to_string(op).unwrap();
        let back: Op =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{}: {e}\n{json}", name(op)));
        assert_eq!(&back, op, "{json}");
    }
    let batch = serde_json::to_string(&ops).unwrap();
    let back: Vec<Op> = serde_json::from_str(&batch).unwrap();
    assert_eq!(back, ops);
}

/// `configuration` を持たない Place の JSON は今まで通り読め、`None` は書き出さない（既存の Op の JSON は変わらない）。
#[test]
fn place_without_configuration_keeps_its_json() {
    let old = r#"{"op":"place","part":"RW_X","on":"PANEL_MZ","point":null,"position":null,"orientation":null}"#;
    let op: Op = serde_json::from_str(old).unwrap();
    assert_eq!(
        op,
        Op::Place {
            part: "RW_X".into(),
            on: Some("PANEL_MZ".into()),
            point: None,
            position: None,
            orientation: None,
            configuration: None,
        }
    );
    let json = serde_json::to_value(&op).unwrap();
    assert!(json.get("configuration").is_none(), "{json}");
}
