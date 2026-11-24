//! JSON（API・serde の中間形）の中で、量やベクトルの形を見分ける（**ここだけ**。service と GUI 向けの変換もこれを使う）。

use super::{Curve, Quantity, Quaternion, Vector3};
use serde_json::Value as Json;

/// `{ value, unit, from? }`（`value` が数値）なら量として読む。
pub fn quantity_from_json(v: &Json) -> Option<Quantity> {
    let map = v.as_object()?;
    let only = map
        .keys()
        .all(|k| matches!(k.as_str(), "value" | "unit" | "from"));
    if !(only && map.get("value").is_some_and(Json::is_number)) {
        return None;
    }
    serde_json::from_value(v.clone()).ok()
}

/// `{ x, y, z, unit?, frame? }` ならベクトルとして読む。
pub fn vector_from_json(v: &Json) -> Option<Vector3> {
    let map = v.as_object()?;
    let only = map
        .keys()
        .all(|k| matches!(k.as_str(), "x" | "y" | "z" | "unit" | "frame"));
    let has_xyz = ["x", "y", "z"].iter().all(|k| map.contains_key(*k));
    if !(only && has_xyz) {
        return None;
    }
    serde_json::from_value(v.clone()).ok()
}

/// `{ w, x, y, z }` ならクォータニオンとして読む。
pub fn quaternion_from_json(v: &Json) -> Option<Quaternion> {
    let map = v.as_object()?;
    if map.len() != 4 || !["w", "x", "y", "z"].iter().all(|k| map.contains_key(*k)) {
        return None;
    }
    serde_json::from_value(v.clone()).ok()
}

/// `{ at, values, at_unit?, unit? }` なら曲線（表）として読む。
pub fn curve_from_json(v: &Json) -> Option<Curve> {
    let map = v.as_object()?;
    let only = map
        .keys()
        .all(|k| matches!(k.as_str(), "at" | "at_unit" | "values" | "unit"));
    if !(only && map.contains_key("at") && map.contains_key("values")) {
        return None;
    }
    serde_json::from_value(v.clone()).ok()
}

/// 量の形か（`quantity_from_json` が読めるか）。
pub fn is_quantity_json(v: &Json) -> bool {
    quantity_from_json(v).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn shapes_are_recognised() {
        let q = quantity_from_json(&json!({ "value": 550.0, "unit": "km" }));
        assert_eq!(q.map(|q| q.unit), Some("km".to_string()));
        assert!(quantity_from_json(&json!({ "value": "x" })).is_none());
        assert!(quantity_from_json(&json!({ "value": 1.0, "min": 0.5 })).is_none());
        assert!(vector_from_json(&json!({ "x": 1.0, "y": 0.0, "z": 0.0 })).is_some());
        assert!(quaternion_from_json(&json!({ "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 })).is_some());
        assert!(vector_from_json(&json!({ "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 })).is_none());
    }
}
