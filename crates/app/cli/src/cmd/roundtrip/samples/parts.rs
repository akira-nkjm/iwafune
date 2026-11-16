//! 往復の検査の見本（部品の木・値・解除と欄・配置の Op）。

use super::{mm, quat, s, scalar};
use fune_service::dto::{Op, PartField, Pose, Provenance, Value};
use serde_json::json;

pub(in super::super) fn part_ops() -> Vec<Op> {
    [tree_ops(), value_ops(), unset_and_field_ops(), place_ops()].concat()
}

fn tree_ops() -> Vec<Op> {
    vec![
        Op::CreatePart {
            id: s("RW_W"),
            group: Some(s("aocs")),
            catalog: Some(s("local/rw_10mNms")),
            kind: None,
            count: Some(2),
            label: Some(s("Spare RW")),
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
            id: s("aocs"),
            to: s("aocs"),
        },
        Op::MovePart {
            id: s("RW_Z"),
            to: Some(s("power")),
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
        at: Some(s("TR-EM table 4")),
        note: Some(s("EM measurement \"v2\"")),
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
            field: PartField::Label(Some(s("X-axis RW"))),
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
            field: serde_json::from_value(json!({"field":"redundancy","value":"warm"})).unwrap(),
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
