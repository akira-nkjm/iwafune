//! 入力への JSON Merge Patch。

use serde_json::Value;

/// JSON Merge Patch（RFC 7396）。
pub(super) fn merge_patch(mut target: Value, patch: &Value) -> Value {
    let Value::Object(p) = patch else {
        return patch.clone();
    };
    if !target.is_object() {
        target = Value::Object(Default::default());
    }
    if let Value::Object(t) = &mut target {
        for (k, v) in p {
            if v.is_null() {
                t.remove(k);
            } else {
                let cur = t.remove(k).unwrap_or(Value::Null);
                t.insert(k.clone(), merge_patch(cur, v));
            }
        }
    }
    target
}

#[cfg(test)]
mod tests {
    use super::merge_patch;
    use serde_json::json;

    #[test]
    fn merge_patch_rfc7396() {
        let t = json!({"a": 1, "b": {"c": 2, "d": 3}});
        let p = json!({"b": {"c": 5, "d": null}, "e": true});
        assert_eq!(
            merge_patch(t, &p),
            json!({"a": 1, "b": {"c": 5}, "e": true})
        );
    }
}
