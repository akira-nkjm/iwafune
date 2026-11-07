//! 時系列のチャンネルの表示単位: 次元名から `sat_units::format` で選び JSON に付加する。
//! 表示値 = SI × `display_scale` で済むよう、一次式の単位（affine・対数でないもの）だけを選ぶ。

use sat_units::format::{format_quantity, si_unit_expression, unit_label};
use sat_units::{parse_dimension, parse_unit};
use serde_json::Value;

/// 出力の JSON の中のすべての `Channel`（`key`・`label`・`dimension`・`values` を持つ表）に表示単位と倍率を付加する。
pub fn fix_channels(v: &mut Value) {
    match v {
        Value::Object(map) => {
            if is_channel(map) {
                fix(map);
            }
            map.values_mut().for_each(fix_channels);
        }
        Value::Array(items) => items.iter_mut().for_each(fix_channels),
        _ => {}
    }
}

fn is_channel(map: &serde_json::Map<String, Value>) -> bool {
    ["key", "label", "dimension", "values"]
        .iter()
        .all(|k| map.contains_key(*k))
}

fn fix(map: &mut serde_json::Map<String, Value>) {
    let Some(dim) = map
        .get("dimension")
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return;
    };
    let peak = map
        .get("values")
        .and_then(Value::as_array)
        .map(|vs| {
            vs.iter()
                .filter_map(Value::as_f64)
                .fold(0.0_f64, |m, x| m.max(x.abs()))
        })
        .unwrap_or(0.0);
    let (unit, scale) = linear_unit(peak, &dim).unwrap_or_else(|| (String::new(), 1.0));
    // 表示の綴り（mN·m·s・°C）。倍率は解析できる式から求め済み
    map.insert("display_unit".into(), Value::String(unit_label(&unit)));
    map.insert("display_scale".into(), serde_json::json!(scale));
}

/// 値の大きさに合う一次式の表示単位と倍率（表示値 = SI × 倍率）。affine・対数なら SI の組立単位にする。
fn linear_unit(peak: f64, dim: &str) -> Option<(String, f64)> {
    let chosen = format_quantity(peak, dim).ok()?.unit;
    let unit = parse_unit(&chosen).ok()?;
    if unit.is_linear() && !unit.is_affine() {
        return Some((chosen, unit.from_si(1.0)));
    }
    let si = si_unit_expression(parse_dimension(dim).ok()?);
    Some((si, 1.0))
}

#[cfg(test)]
mod tests {
    use super::fix_channels;
    use serde_json::json;

    #[test]
    fn inserts_fraction_fields_in_nested_and_empty_series() {
        let mut v = json!({"nested": [{"channels": [
            {"key": "ratio", "label": "割合", "dimension": "Fraction", "values": [-0.1, 0.5]},
            {"key": "empty", "label": "空", "dimension": "Power", "values": []}
        ]}]});
        fix_channels(&mut v);
        let c = &v["nested"][0]["channels"];
        assert_eq!(c[0]["display_unit"], "%");
        assert_eq!(c[0]["display_scale"], 100.0);
        assert_eq!(c[1]["display_unit"], "W");
        assert_eq!(c[1]["display_scale"], 1.0);
    }

    #[test]
    fn picks_unit_from_dimension() {
        let mut v = json!({ "series": { "channels": [
            { "key": "p", "label": "p", "dimension": "Power", "values": [0.0, 0.012] },
            { "key": "t", "label": "t", "dimension": "Temperature", "values": [290.0] }
        ] } });
        fix_channels(&mut v);
        let c = &v["series"]["channels"];
        assert_eq!(c[0]["display_unit"], "mW");
        assert!((c[0]["display_scale"].as_f64().unwrap() - 1000.0).abs() < 1e-9);
        assert_eq!(c[1]["display_unit"], "K");
        assert_eq!(c[1]["display_scale"], 1.0);
    }
}
