//! 往復の検査の見本（定義・系・表示・記録の Op）。

use super::s;
use fune_service::dto::{
    AttributeDef, Case, Distribution, Harness, KindDef, LinkDirection, ModeCell, ModeSegment,
    ModeWhen, Net, NetField, NetKind, Op, PortRef, ProjectField, Quantity, SettingValue, Source,
    SourceKind, StartFrom, StartRef, ViewNodePos,
};
use serde_json::json;
use std::collections::BTreeMap;

pub(in super::super) fn def_ops() -> Vec<Op> {
    let mut attr: AttributeDef = serde_json::from_value(json!({ "label": "Max dipole label", "label_en": "Max dipole",
        "type": "scalar", "by_state": true, "dimension": "MagneticDipoleMoment", "units": ["A*m^2", "mA*m^2"],
        "display_unit": "A*m^2" })).unwrap();
    attr.id = "max_dipole2".into();
    let mut kind: KindDef = serde_json::from_value(
        json!({ "label": "Wire label", "label_en": "Wire", "extends": "component",
        "states": ["off", "on"], "required": ["mass"], "optional": ["power"] }),
    )
    .unwrap();
    kind.id = "wire2".into();
    let mut full: AttributeDef = serde_json::from_value(json!({ "label": "Interface temperature label", "label_en": "Interface temperature",
        "description": "Interface temperature (\"estimated\")", "type": "scalar", "by_state": false, "dimension": "Temperature",
        "units": ["degC", "K"], "display_unit": "degC", "min": -40.5, "max": 358.15, "default": 293.15,
        "budget": "power", "unit_vector": true, "fov_axis": "boresight", "section": "thermal" })).unwrap();
    full.id = "if_temp".into();
    let mut sym: AttributeDef = serde_json::from_value(json!({ "label": "Finish", "type": "enum",
        "symbols": ["black", "white", "mli"], "min": -1.0 }))
    .unwrap();
    sym.id = "finish".into();
    let mut ported: KindDef = serde_json::from_value(json!({ "label": "PDU label", "label_en": "PDU",
        "description": "also outputs -12 V", "extends": "component", "role": "pcdu", "states": ["off", "on"],
        "required": ["mass", "power"], "optional": ["voltage"],
        "ports": { "IN": "power", "OUT1": "power", "CAN": "data", "ANT": "rf" } })).unwrap();
    ported.id = "pdu2".into();
    vec![
        Op::CreateAttribute { def: attr },
        Op::CreateAttribute { def: full.clone() },
        Op::UpdateAttribute { def: full },
        Op::CreateAttribute { def: sym },
        Op::DeleteAttribute { id: s("finish") },
        Op::CreateKind { def: kind },
        Op::CreateKind {
            def: ported.clone(),
        },
        Op::UpdateKind { def: ported },
        Op::DeleteKind { id: s("wire2") },
        Op::RenameAttribute {
            id: s("finish"),
            to: s("coating"),
        },
        Op::RenameKind {
            id: s("wire2"),
            to: s("cable"),
        },
    ]
}

pub(in super::super) fn system_ops() -> Vec<Op> {
    let mut net = Net::new("P5V_X", NetKind::Power);
    net.label = Some(s("5 V rail"));
    net.source = Some(PortRef::new("PCDU", "OUT1"));
    net.rail = Some(Quantity::new(5.0, "V"));
    net.limit = Some(Quantity::new(2.0, "A"));
    let mut bus = Net::new("CAN_X", NetKind::Data);
    bus.protocol = Some(s("CAN"));
    bus.bitrate = Some(Quantity::new(1.0, "Mbit/s"));
    bus.master = Some(PortRef::new("OBC", "CAN1"));
    let cell = |part: &str, state: Option<&str>, duty: &[(&str, f64)]| ModeCell {
        mode: s("NOMINAL"),
        part: s(part),
        state: state.map(s),
        duty: duty
            .iter()
            .map(|(k, v)| (s(k), *v))
            .collect::<BTreeMap<_, _>>(),
        active: None,
        spare_state: None,
        eclipse: false,
    };
    vec![
        Op::CreateMode {
            id: s("DETUMBLE"),
            label: Some(s("Detumble")),
            description: Some(s("Right after separation")),
            copy_from: Some(s("SAFE")),
        },
        Op::RemoveMode { id: s("DOWNLINK") },
        Op::RenameMode {
            id: s("DOWNLINK"),
            to: s("DL"),
        },
        Op::MoveMode {
            id: s("SAFE"),
            position: fune_service::dto::ModePosition::First,
        },
        Op::MoveMode {
            id: s("SAFE"),
            position: fune_service::dto::ModePosition::Last,
        },
        Op::MoveMode {
            id: s("SAFE"),
            position: fune_service::dto::ModePosition::Before(s("NOMINAL")),
        },
        Op::MoveMode {
            id: s("SAFE"),
            position: fune_service::dto::ModePosition::After(s("NOMINAL")),
        },
        Op::SetModeDuration {
            mode: "NOMINAL".into(),
            duration: Some(Quantity::new(90.0, "min")),
        },
        Op::SetModeDuration {
            mode: "NOMINAL".into(),
            duration: None,
        },
        Op::SetModeTiming {
            mode: "MISSION".into(),
            when: Some(ModeWhen::Sunlit),
            start: Some(StartRef {
                from: StartFrom::EclipseExit,
                offset: Some(Quantity::new(5.0, "min")),
            }),
        },
        Op::SetModeTiming {
            mode: "MISSION".into(),
            when: None,
            start: None,
        },
        Op::SetSequence {
            name: "imaging_day".into(),
            anchor: Some(StartRef {
                from: StartFrom::EclipseEntry,
                offset: None,
            }),
            segments: vec![
                ModeSegment {
                    mode: "NOMINAL".into(),
                    duration: Quantity::new(76.0, "min"),
                },
                ModeSegment {
                    mode: "MISSION".into(),
                    duration: Quantity::new(5.0, "min"),
                },
            ],
        },
        Op::RemoveSequence {
            name: "imaging_day".into(),
        },
        Op::SetModeStates {
            cells: vec![
                cell("UHF_TRX", Some("rx"), &[("tx", 0.1)]),
                cell("CAM", None, &[]),
                ModeCell {
                    active: Some(3),
                    spare_state: Some(s("standby")),
                    eclipse: true,
                    ..cell("RW_X", Some("on"), &[])
                },
                ModeCell {
                    eclipse: true,
                    ..cell("CAM", None, &[])
                },
            ],
        },
        Op::CreateNet { net },
        Op::UpdateNet { net: bus },
        Op::SetNetField {
            net: s("P12V"),
            field: NetField::Rail(Some(Quantity::new(-12.0, "V"))),
        },
        Op::SetNetField {
            net: s("P12V"),
            field: NetField::Limit(None),
        },
        Op::SetNetField {
            net: s("P12V"),
            field: NetField::Kind(NetKind::Data),
        },
        Op::SetNetField {
            net: s("CAN_BUS"),
            field: NetField::Master(Some(PortRef::new("OBC", "CAN2"))),
        },
        Op::RemoveNet { id: s("P5V") },
    ]
}

pub(in super::super) fn view_ops() -> Vec<Op> {
    vec![
        Op::SetViewEdge {
            view: "electrical".into(),
            key: "P12V:PCDU.OUT1>RW_X.PWR".into(),
            edge: fune_service::dto::ViewEdge {
                points: vec![[-20, 0], [20, 0], [20, 80]],
                source_side: Some(fune_service::dto::ViewPortSide::Right),
                target_side: None,
                source_offset: Some(18),
                target_offset: None,
            },
        },
        Op::UnsetViewEdge {
            view: "electrical".into(),
            key: "P12V:PCDU.OUT1>RW_X.PWR".into(),
        },
        Op::ResetView {
            view: "electrical".into(),
        },
        Op::SetViewLayout {
            view: s("electrical"),
            nodes: vec![
                ViewNodePos {
                    id: s("RW_X"),
                    x: 420,
                    y: -120,
                    w: Some(160),
                    h: Some(60),
                },
                ViewNodePos {
                    id: s("P12V"),
                    x: 10,
                    y: 20,
                    w: None,
                    h: None,
                },
            ],
            remove: vec![s("OLD")],
        },
    ]
}

pub(in super::super) fn record_ops() -> Vec<Op> {
    let mut case: Case = serde_json::from_value(json!({ "label": "Worst-case eclipse", "description": "β 0",
        "mode": "NOMINAL", "phase": "NOMINAL_OPS", "side": "max", "thermal_case": "cold", "beta": { "value": 0.0, "unit": "deg" }, "orbits": 5, "sun_distance": { "value": 0.7, "unit": "AU" }, "epoch": "2031-05-02T00:00:00Z" })).unwrap();
    case.id = "eclipse_worst".into();
    let source = Source {
        id: "ds_x".into(),
        kind: SourceKind::Datasheet,
        label: Some(s("Datasheet")),
        reference: Some(s("https://example.com/ds.pdf")),
        revision: Some(s("Rev. B")),
        retrieved: Some(s("2026-10-02")),
        sha256: Some("0123456789abcdef".repeat(4)),
        distribution: Some(Distribution::OnRequest),
        date: Some(s("2026-10-01")),
        by: Some(s("test-team")),
        note: Some(s("rev B")),
    };
    let harness = Harness {
        id: "H_X".into(),
        label: Some(s("PCDU → RW_X")),
        from: PortRef::new("PCDU", "OUT3"),
        to: PortRef::new("RW_X", "PWR"),
        nets: vec!["P12V".into(), "CAN_BUS".into()],
        wire: Some(s("local/awg22")),
        cores: Some(2),
        length: Some(Quantity::new(0.35, "m")),
    };
    vec![
        Op::SetCase { case },
        Op::RemoveCase {
            id: s("eclipse_worst"),
        },
        Op::SetSource {
            source: source.clone(),
            dir: Some(s("datasheets/rw")),
        },
        Op::SetSource { source, dir: None },
        Op::RemoveSource { id: s("ds_x") },
        Op::MoveSource {
            id: s("ds_x"),
            dir: s("datasheets"),
        },
        Op::MoveSource {
            id: s("ds_x"),
            dir: s("."),
        },
        Op::SetHarness { harness },
        Op::RemoveHarness { id: s("H_X") },
        Op::CreateManeuver {
            maneuver: maneuver(
                "MV_X",
                json!({ "category": "insertion", "method": "low_thrust",
                "thruster": "RW_X", "target": { "altitude": { "value": 600.0, "unit": "km" } } }),
            ),
        },
        Op::SetManeuver {
            maneuver: maneuver(
                "MV_Y",
                json!({ "label": "Deorbit \"test\"", "category": "deorbit",
                "method": "finite_burn", "thruster": "RW_X", "after": "MV_X",
                "via_altitude": { "value": 20000.0, "unit": "km" },
                "target": { "body": "mars", "perigee_altitude": { "value": -50.0, "unit": "km" },
                    "apogee_altitude": { "value": 600.0, "unit": "km" },
                    "inclination": { "value": 97.5, "unit": "deg" } },
                "constraints": { "sunlit_only": false, "mode": "NOMINAL" },
                "transfer": { "departure": "2028-12-10T00:00:00Z", "arrival": "2029-09-25T00:00:00Z",
                    "departure_by": "launcher" } }),
            ),
        },
        Op::CreateManeuver {
            maneuver: maneuver(
                "MV_Z",
                json!({ "label": "Correction allowance", "category": "station_keeping",
                "method": "impulsive", "thruster": "RW_X", "after": "MV_X",
                "delta_v": { "value": 15.0, "unit": "m/s" } }),
            ),
        },
        Op::RemoveManeuver { id: s("MV_X") },
        Op::CreatePhase {
            phase: phase(
                "PH_X",
                json!({ "label": "Commissioning \"test\"", "order": -1, "start": "Separation",
                "end": "All subsystems checked out", "duration": { "value": 30.0, "unit": "d" },
                "modes": ["SAFE", "NOMINAL"], "configuration": "STOWED" }),
            ),
        },
        Op::SetPhase {
            phase: phase("PH_Y", json!({ "order": 2 })),
        },
        Op::RemovePhase { id: s("PH_X") },
        Op::RenamePhase {
            id: s("PH_X"),
            to: s("PH_Z"),
        },
        Op::CreateConfiguration {
            id: s("STOWED"),
            label: Some(s("Stowed \"test\"")),
            description: Some(s("Paddle folded onto the +X face")),
        },
        Op::CreateConfiguration {
            id: s("DEPLOYED"),
            label: None,
            description: None,
        },
        Op::SetConfiguration {
            id: s("STOWED"),
            label: Some(s("Stowed")),
            description: None,
        },
        Op::RemoveConfiguration { id: s("STOWED") },
        Op::RenameConfiguration {
            id: s("STOWED"),
            to: s("FOLDED"),
        },
        Op::UnsetConfigurationPose {
            configuration: s("STOWED"),
            part: s("RW_X"),
        },
        Op::SetPhaseField {
            phase: s("PH_X"),
            path: s("orbit.altitude"),
            value: Some(SettingValue::Quantity(Quantity::new(450.0, "km"))),
        },
        Op::SetPhaseField {
            phase: s("PH_X"),
            path: s("orbit.label"),
            value: Some(SettingValue::Text(s("1"))),
        },
        Op::SetPhaseField {
            phase: s("PH_X"),
            path: s("law.NOMINAL.law"),
            value: Some(SettingValue::Text(s("nadir_pointing"))),
        },
        Op::SetPhaseField {
            phase: s("PH_X"),
            path: s("orbit.altitude"),
            value: None,
        },
        Op::CopyPhaseOrbit { phase: s("PH_X") },
        Op::UnsetPhaseOrbit { phase: s("PH_X") },
        Op::UnsetPhaseLaw {
            phase: s("PH_X"),
            mode: s("NOMINAL"),
        },
        Op::SetCasePhase {
            case: s("eclipse_worst"),
            phase: Some(s("PH_X")),
        },
        Op::SetCasePhase {
            case: s("eclipse_worst"),
            phase: None,
        },
        Op::SetCaseEpoch {
            case: s("eclipse_worst"),
            epoch: Some(s("2031-05-02T00:00:00Z")),
        },
        Op::SetCaseEpoch {
            case: s("eclipse_worst"),
            epoch: None,
        },
        Op::CreateGroundStation {
            id: s("SVALBARD"),
            label: Some(s("Svalbard \"test\"")),
            latitude: Some(Quantity::new(78.2, "deg")),
            longitude: Some(Quantity::new(15.4, "deg")),
            altitude: Some(Quantity::new(450.0, "m")),
            min_elevation: Some(Quantity::new(-5.0, "deg")),
        },
        Op::CreateGroundStation {
            id: s("BARE_GS"),
            label: None,
            latitude: None,
            longitude: None,
            altitude: None,
            min_elevation: None,
        },
        Op::RemoveGroundStation { id: s("SVALBARD") },
        Op::RenameGroundStation {
            id: s("BARE_GS"),
            to: s("BARE_GS2"),
        },
        Op::RenameNet {
            id: s("P5V"),
            to: s("P5V_MAIN"),
        },
        Op::RenameHarness {
            id: s("H_OBC_CAM"),
            to: s("H_OBC_CAMERA"),
        },
        Op::RenameCase {
            id: s("hot_imaging"),
            to: s("hot_img"),
        },
        Op::RenameSource {
            id: s("sys_judgement"),
            to: s("sys_judge"),
        },
        Op::RenameManeuver {
            id: s("RAISE"),
            to: s("M_RAISE"),
        },
        Op::CreateCommLink {
            id: s("x_down"),
            direction: LinkDirection::Down,
            part: s("STX"),
            band: s("s_band"),
        },
        Op::RemoveCommLink { id: s("x_down") },
        Op::RenameCommLink {
            id: s("x_down"),
            to: s("x_dl"),
        },
        Op::SetProjectField {
            field: ProjectField::Name(s("demo project")),
        },
        Op::SetProjectField {
            field: ProjectField::Label(Some(s("Demo \"label\""))),
        },
        Op::SetProjectField {
            field: ProjectField::Label(None),
        },
        Op::AddLocalCatalog {
            name: s("vendor"),
            path: s("vendor/parts"),
        },
        Op::CreateRequirement {
            requirement: requirement(
                "R_X",
                json!({ "statement": "The temperature shall stay \"in range\".", "statement_en": "In range",
                "label": "Range", "quantity": "Temperature", "source": "sys_judgement",
                "constraint": { "kind": "range", "min": { "value": -5.0, "unit": "degC" },
                    "max": { "value": 40.0, "unit": "degC" } },
                "quantify": { "cases": ["hot_imaging", "cold_safe"] },
                "verify": { "analysis": "thermal.steady", "check": "range_low:BAT:operating" } }),
            ),
        },
        Op::SetRequirement {
            requirement: requirement(
                "R_Y",
                json!({ "statement": "Total mass", "quantity": "Mass",
                "constraint": { "kind": "at_most", "value": { "value": 12.0, "unit": "kg" } },
                "quantify": { "all_cases": true }, "verify": { "budget": "mass" } }),
            ),
        },
        Op::SetRequirement {
            requirement: requirement(
                "R_Z",
                json!({ "statement": "Temperature", "quantity": "Temperature",
                "constraint": { "kind": "equal", "value": { "value": 20.0, "unit": "degC" },
                    "tolerance": { "value": 2.0, "unit": "K" } } }),
            ),
        },
        Op::RemoveRequirement { id: s("R_X") },
        Op::SetSetting {
            path: s("conditions.launch.configuration"),
            value: Some(SettingValue::Text(s("42"))),
        },
        Op::SetSetting {
            path: s("conditions.launch.configuration"),
            value: None,
        },
        Op::SetSetting {
            path: s("analysis.common.default_case"),
            value: Some(SettingValue::Text(s("hot_imaging"))),
        },
        Op::SetSetting {
            path: s("analysis.power.dod_max"),
            value: Some(SettingValue::Number(0.25)),
        },
        Op::SetSetting {
            path: s("analysis.power.orbits"),
            value: Some(SettingValue::Integer(4)),
        },
        Op::SetSetting {
            path: s("conditions.orbit.arg_perigee"),
            value: Some(SettingValue::Quantity(Quantity::new(-5.0, "deg"))),
        },
        Op::SetSetting {
            path: s("conditions.orbit.altitude"),
            value: Some(SettingValue::Quantity(Quantity::new(500.0, "km"))),
        },
        Op::SetSetting {
            path: s("analysis.power.dod_max"),
            value: None,
        },
    ]
}

/// JSON の中身と ID からマヌーバを作る。
fn maneuver(id: &str, body: serde_json::Value) -> fune_service::dto::Maneuver {
    let mut m: fune_service::dto::Maneuver = serde_json::from_value(body).unwrap();
    m.id = id.into();
    m
}

/// JSON の中身と ID からフェーズを作る。
fn phase(id: &str, body: serde_json::Value) -> fune_service::dto::Phase {
    let mut p: fune_service::dto::Phase = serde_json::from_value(body).unwrap();
    p.id = id.into();
    p
}

/// JSON の中身と ID から要求を作る。
fn requirement(id: &str, body: serde_json::Value) -> fune_service::dto::Requirement {
    let mut r: fune_service::dto::Requirement = serde_json::from_value(body).unwrap();
    r.id = id.into();
    r
}
