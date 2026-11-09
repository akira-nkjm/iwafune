//! 往復の検査の見本（すべての Op の種類を少なくとも 1 つ）。

use fune_service::dto::{
    AttributeDef, Case, Harness, KindDef, ModeCell, Net, NetField, NetKind, Op, PartField, PortRef,
    Pose, Provenance, Quantity, Quaternion, SettingValue, Source, SourceKind, Value, Vector3,
    ViewNodePos,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::f64::consts::FRAC_1_SQRT_2;

pub(super) fn s(v: &str) -> String {
    v.to_owned()
}

pub(super) fn scalar(value: f64, unit: &str) -> Value {
    serde_json::from_value(json!({ "type": "scalar", "data": { "value": value, "unit": unit } }))
        .unwrap()
}

fn mm(x: f64, y: f64, z: f64) -> Vector3 {
    Vector3 {
        x,
        y,
        z,
        unit: Some(s("mm")),
        frame: None,
    }
}
pub(super) fn part_ops() -> Vec<Op> {
    [tree_ops(), value_ops(), unset_and_field_ops(), place_ops()].concat()
}

fn quat() -> Quaternion {
    Quaternion {
        w: FRAC_1_SQRT_2,
        x: 0.0,
        y: 0.0,
        z: FRAC_1_SQRT_2,
    }
}

fn tree_ops() -> Vec<Op> {
    vec![
        Op::CreatePart {
            id: s("RW_W"),
            group: Some(s("AOCS")),
            catalog: Some(s("local/rw_10mNms")),
            kind: None,
            count: Some(2),
            label: Some(s("予備 RW")),
            mode_state: Some(s("off")),
        },
        Op::CreatePart {
            id: s("HTR_X"),
            group: None,
            catalog: None,
            kind: Some(s("heater")),
            count: None,
            label: None,
            mode_state: None,
        },
        Op::RemovePart { id: s("RW_Z") },
        Op::RenamePart {
            id: s("RW_Z"),
            to: s("RW_S"),
        },
        Op::RenameGroup {
            id: s("AOCS"),
            to: s("AOCS"),
        },
        Op::MovePart {
            id: s("RW_Z"),
            to: Some(s("POWER")),
        },
        Op::MovePart {
            id: s("RW_Z"),
            to: None,
        },
    ]
}

fn value_ops() -> Vec<Op> {
    let ranged = serde_json::from_value::<Value>(json!({ "type": "scalar",
        "data": { "value": 620.0, "min": 600.0, "max": 640.0, "unit": "g" } }))
    .unwrap();
    let from = Some(Provenance {
        source: Some(s("em_test_2026_10")),
        note: Some(s("EM 実測 \"v2\"")),
    });
    vec![
        Op::SetValue {
            part: s("RW_X"),
            attr: s("mass"),
            state: None,
            value: ranged,
            from,
        },
        Op::SetValue {
            part: s("RW_X"),
            attr: s("power"),
            state: Some(s("on")),
            value: scalar(1.2, "W"),
            from: None,
        },
        Op::SetValue {
            part: s("RW_X"),
            attr: s("temp_op"),
            state: None,
            value: serde_json::from_value(
                json!({ "type": "range", "data": { "min": -20.0, "max": 60.0, "unit": "degC" } }),
            )
            .unwrap(),
            from: None,
        },
        Op::SetValue {
            part: s("RW_X"),
            attr: s("size"),
            state: None,
            value: Value::Vec3(mm(100.0, 100.0, 20.0)),
            from: None,
        },
        Op::SetValue {
            part: s("PANEL_PX"),
            attr: s("material"),
            state: None,
            value: Value::Enum(s("al6061_t6")),
            from: None,
        },
        Op::SetValue {
            part: s("RW_X"),
            attr: s("heat_fraction"),
            state: None,
            value: scalar(0.8, ""),
            from: None,
        },
    ]
}

fn unset_and_field_ops() -> Vec<Op> {
    vec![
        Op::UnsetValue {
            part: s("RW_X"),
            attr: s("mass"),
            state: None,
        },
        Op::UnsetValue {
            part: s("RW_X"),
            attr: s("power"),
            state: Some(s("peak")),
        },
        Op::SetField {
            part: s("RW_X"),
            field: PartField::Label(Some(s("X 軸 RW"))),
        },
        Op::SetField {
            part: s("RW_X"),
            field: PartField::Label(None),
        },
        Op::SetField {
            part: s("RW_X"),
            field: PartField::Count(3),
        },
        Op::SetField {
            part: s("RW_X"),
            field: PartField::Candidates(vec![s("local/a"), s("local/b")]),
        },
        Op::SetField {
            part: s("RW_X"),
            field: serde_json::from_value(json!({ "field": "maturity", "value": "new" })).unwrap(),
        },
    ]
}

fn place_ops() -> Vec<Op> {
    let q = quat();
    vec![
        Op::Place {
            part: s("RW_X"),
            on: Some(s("PANEL_MZ")),
            point: Some(s("RW_SLOT_X")),
            position: Some(mm(0.0, 0.0, 12.0)),
            orientation: Some(q.clone()),
        },
        Op::Place {
            part: s("RW_X"),
            on: None,
            point: None,
            position: None,
            orientation: None,
        },
        Op::SetPort {
            part: s("RW_X"),
            port: s("PWR"),
            net: Some(s("P12V")),
        },
        Op::SetPort {
            part: s("RW_X"),
            port: s("PWR"),
            net: None,
        },
        Op::SetMountPoint {
            part: s("PANEL_MZ"),
            point: s("SLOT_9"),
            pose: Some(Pose {
                position: Some(mm(30.0, -5.0, 5.0)),
                orientation: Some(q),
            }),
        },
        Op::SetMountPoint {
            part: s("PANEL_MZ"),
            point: s("SLOT_9"),
            pose: None,
        },
    ]
}

pub(super) fn def_ops() -> Vec<Op> {
    let mut attr: AttributeDef = serde_json::from_value(json!({ "label": "最大双極子", "label_en": "Max dipole",
        "type": "scalar", "by_state": true, "dimension": "MagneticDipoleMoment", "units": ["A*m^2", "mA*m^2"],
        "display_unit": "A*m^2" })).unwrap();
    attr.id = "max_dipole2".into();
    let mut kind: KindDef = serde_json::from_value(
        json!({ "label": "線材", "label_en": "Wire", "extends": "component",
        "role": "wire", "states": ["off", "on"], "required": ["mass"], "optional": ["power"] }),
    )
    .unwrap();
    kind.id = "wire2".into();
    let mut full: AttributeDef = serde_json::from_value(json!({ "label": "取付面温度", "label_en": "Interface temperature",
        "description": "取付面の温度（\"推定\"）", "type": "scalar", "by_state": false, "dimension": "Temperature",
        "units": ["degC", "K"], "display_unit": "degC", "min": -40.5, "max": 358.15, "default": 293.15,
        "budget": "power", "unit_vector": true, "section": "thermal" })).unwrap();
    full.id = "if_temp".into();
    let mut sym: AttributeDef = serde_json::from_value(json!({ "label": "仕上げ", "type": "enum",
        "symbols": ["black", "white", "mli"], "min": -1.0 }))
    .unwrap();
    sym.id = "finish".into();
    let mut ported: KindDef = serde_json::from_value(json!({ "label": "電源分配器", "label_en": "PDU",
        "description": "-12 V も出す", "extends": "component", "role": "pcdu", "states": ["off", "on"],
        "required": ["mass", "power"], "optional": ["voltage"],
        "ports": { "IN": "power", "OUT1": "power", "CAN": "data", "ANT": "rf" } })).unwrap();
    ported.id = "pdu2".into();
    vec![
        Op::CreateAttribute { def: attr },
        Op::CreateAttribute { def: full },
        Op::CreateAttribute { def: sym },
        Op::CreateKind { def: kind },
        Op::CreateKind { def: ported },
    ]
}

pub(super) fn system_ops() -> Vec<Op> {
    let mut net = Net::new("P5V_X", NetKind::Power);
    net.label = Some(s("5 V 系"));
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
    };
    vec![
        Op::CreateMode {
            id: s("DETUMBLE"),
            label: Some(s("デタンブル")),
            description: Some(s("分離直後")),
            copy_from: Some(s("SAFE")),
        },
        Op::RemoveMode { id: s("DOWNLINK") },
        Op::RenameMode {
            id: s("DOWNLINK"),
            to: s("DL"),
        },
        Op::SetModeStates {
            cells: vec![
                cell("UHF_TRX", Some("rx"), &[("tx", 0.1)]),
                cell("CAM", None, &[]),
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
pub(super) fn view_ops() -> Vec<Op> {
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
pub(super) fn record_ops() -> Vec<Op> {
    let mut case: Case = serde_json::from_value(json!({ "label": "最悪の日陰", "description": "β 0",
        "mode": "NOMINAL", "side": "max", "thermal_case": "cold", "beta": { "value": 0.0, "unit": "deg" }, "orbits": 5 })).unwrap();
    case.id = "eclipse_worst".into();
    let source = Source {
        id: "ds_x".into(),
        kind: SourceKind::Datasheet,
        label: Some(s("データシート")),
        reference: Some(s("https://example.com/ds.pdf")),
        date: Some(s("2026-10-01")),
        by: Some(s("tanaka")),
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
        Op::SetSource { source },
        Op::RemoveSource { id: s("ds_x") },
        Op::SetHarness { harness },
        Op::RemoveHarness { id: s("H_X") },
        Op::SetSetting {
            path: s("analysis.power.dod_max"),
            value: Some(SettingValue::Number(0.25)),
        },
        Op::SetSetting {
            path: s("analysis.power.orbits"),
            value: Some(SettingValue::Integer(4)),
        },
        Op::SetSetting {
            path: s("mission.orbit.arg_perigee"),
            value: Some(SettingValue::Quantity(Quantity::new(-5.0, "deg"))),
        },
        Op::SetSetting {
            path: s("mission.orbit.altitude"),
            value: Some(SettingValue::Quantity(Quantity::new(500.0, "km"))),
        },
        Op::SetSetting {
            path: s("analysis.power.dod_max"),
            value: None,
        },
    ]
}
