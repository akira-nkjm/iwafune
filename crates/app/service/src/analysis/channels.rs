//! 時系列のチャンネルの表示名と表示単位: 表示名は系列の種類（`kind`）と対象（`subject`）から語彙（`analysis::labels`）で引き、
//! 表示単位は次元名から `sat_units::format` で選び JSON に付加する（形は `dto::DisplayChannel`）。型のままの系列には
//! [`display_series`] が同じ規則で付ける（電源のモードごとのグラフ）。
//! 表示値 = SI × `display_scale` で済むよう、一次式の単位（affine・対数でないもの）だけを選ぶ。

use super::labels::channel_label;
use crate::dto::{DisplayChannel, DisplaySeries};
use sat_analysis::spec::{Channel, ChannelKind, Series};
use sat_units::format::{format_quantity, si_unit_expression, unit_label};
use sat_units::{parse_dimension, parse_unit};
use serde_json::Value;

/// 出力の JSON の中のすべての `Channel`（`key`・`kind`・`dimension`・`values` を持つ表）に表示名・表示単位・倍率を付加する。
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
    ["key", "kind", "dimension", "values"]
        .iter()
        .all(|k| map.contains_key(*k))
}

fn fix(map: &mut serde_json::Map<String, Value>) {
    let kind = map
        .get("kind")
        .and_then(|k| serde_json::from_value::<ChannelKind>(k.clone()).ok());
    if let Some(kind) = kind {
        let subject = map.get("subject").and_then(Value::as_str);
        let l = channel_label(kind, subject);
        map.insert("label".into(), Value::String(l.label));
        map.insert("label_en".into(), Value::String(l.label_en));
    }
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
    let (unit, scale) = unit_and_scale(peak, &dim);
    map.insert("display_unit".into(), Value::String(unit));
    map.insert("display_scale".into(), serde_json::json!(scale));
}

/// 表示の綴り（mN·m·s・°C）と倍率。倍率は解析できる式から求め済み。**表示単位の選び方は JSON の付加と型のままの付加で共通。**
fn unit_and_scale(peak: f64, dim: &str) -> (String, f64) {
    let (unit, scale) = linear_unit(peak, dim).unwrap_or_else(|| (String::new(), 1.0));
    (unit_label(&unit), scale)
}

/// 型のままの時系列に表示名・表示単位・倍率を付ける（出力の JSON に [`fix_channels`] が付けるものと同じ）。
pub(crate) fn display_series(series: &Series) -> DisplaySeries {
    DisplaySeries {
        t: series.t.clone(),
        channels: series.channels.iter().map(display_channel).collect(),
    }
}

fn display_channel(channel: &Channel) -> DisplayChannel {
    let l = channel_label(channel.kind, channel.subject.as_deref());
    let peak = channel.values.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
    let (display_unit, display_scale) = unit_and_scale(peak, &channel.dimension);
    DisplayChannel {
        channel: channel.clone(),
        label: l.label,
        label_en: l.label_en,
        display_unit,
        display_scale,
    }
}

/// 値の大きさに合う一次式の表示単位と倍率（表示値 = SI × 倍率）。affine・対数なら SI の組立単位にする。
pub(crate) fn linear_unit(peak: f64, dim: &str) -> Option<(String, f64)> {
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
            {"key": "soc", "kind": "soc", "subject": null, "dimension": "Fraction", "values": [-0.1, 0.5]},
            {"key": "wheel:RW", "kind": "wheel_momentum", "subject": "RW", "dimension": "Power", "values": []}
        ]}]});
        fix_channels(&mut v);
        let c = &v["nested"][0]["channels"];
        assert_eq!(c[0]["display_unit"], "%");
        assert_eq!(c[0]["display_scale"], 100.0);
        assert_eq!(c[1]["display_unit"], "W");
        assert_eq!(c[1]["display_scale"], 1.0);
        // 表示名は種類と対象から語彙で引く（日本語と英語）
        assert_eq!(c[0]["label"], "SOC");
        assert_eq!(c[1]["label"], "RW の角運動量");
        assert_eq!(c[1]["label_en"], "RW momentum");
    }

    #[test]
    fn picks_unit_from_dimension() {
        let mut v = json!({ "series": { "channels": [
            { "key": "p", "kind": "load", "dimension": "Power", "values": [0.0, 0.012] },
            { "key": "t", "kind": "node_temperature", "subject": "PANEL", "dimension": "Temperature", "values": [290.0] }
        ] } });
        fix_channels(&mut v);
        let c = &v["series"]["channels"];
        assert_eq!(c[0]["display_unit"], "mW");
        assert!((c[0]["display_scale"].as_f64().unwrap() - 1000.0).abs() < 1e-9);
        assert_eq!(c[1]["display_unit"], "K");
        assert_eq!(c[1]["display_scale"], 1.0);
    }
}
