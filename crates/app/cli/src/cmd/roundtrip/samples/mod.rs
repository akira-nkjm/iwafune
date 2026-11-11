//! 往復の検査の見本（すべての Op の種類を少なくとも 1 つ）と共通関数。

mod parts;
mod records;

pub(super) use parts::part_ops;
pub(super) use records::{def_ops, record_ops, system_ops, view_ops};

use fune_service::dto::{Quaternion, Value, Vector3};
use serde_json::json;
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

fn quat() -> Quaternion {
    Quaternion {
        w: FRAC_1_SQRT_2,
        x: 0.0,
        y: 0.0,
        z: FRAC_1_SQRT_2,
    }
}
