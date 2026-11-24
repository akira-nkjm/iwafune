//! 人が打つ値の文字列（CLI の `fune set RW_X.mass "620 g"`・GUI の入力欄）の書式。**読む向きと書く向きの両方がここ**。
//!
//! - 読む: 文字列 → [`Value`]・[`SettingValue`]・量・一覧（[`value_from_text`]・[`setting_from_text`] など）
//! - 書く: 値 → 文字列（[`write`]。`to_cli` の値の引数）。書いたものを読むと元の値に戻る（往復のテストはこのファイル）
//!
//! 単位の部分はそのまま文字列で残す（次元の検査は `check`）。量として読むときだけ単位が読めるかを確かめる（[`super::units`]）。
//! どの型で読むか（属性定義・設定の今の値）は呼ぶ側（fune-service）が決める。

mod write;

pub use write::{
    list_to_text, number_to_text, number_unit_to_text, quantity_to_text, quaternion_to_text,
    range_to_text, setting_to_text, value_to_text, vector_to_text,
};

use super::units::unit_info;
use super::{
    Curve, Quantity, Quaternion, RangeValue, Scalar, SettingValue, Value, ValueError, ValueType,
    Vector3,
};

/// `"620 g"`・`"0.3"`・`"-20degC"` → (数値, 単位)。単位が無ければ無次元（空文字列）。
pub fn parse_number_unit(text: &str) -> Result<(f64, String), ValueError> {
    let text = text.trim();
    let split = number_prefix_len(text);
    if split == 0 {
        return Err(ValueError::Syntax(text.to_string()));
    }
    let number: f64 = text[..split]
        .parse()
        .map_err(|_| ValueError::Syntax(text.to_string()))?;
    if !number.is_finite() {
        return Err(ValueError::Syntax(text.to_string()));
    }
    Ok((number, text[split..].trim().to_string()))
}

/// 先頭の数値（符号・小数点・指数を含む）の長さ。
fn number_prefix_len(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut i = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        i += 1;
    }
    let digits_start = i;
    while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
        i += 1;
    }
    if i == digits_start {
        return 0;
    }
    // 指数部（`1e-3`）。`e` の後に数字が続くときだけ数値の一部とみなす。
    if i < bytes.len() && matches!(bytes[i], b'e' | b'E') {
        let mut j = i + 1;
        if j < bytes.len() && matches!(bytes[j], b'+' | b'-') {
            j += 1;
        }
        if j < bytes.len() && bytes[j].is_ascii_digit() {
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            i = j;
        }
    }
    i
}

/// 値の型に合わせて文字列を解釈する。
///
/// - scalar: `"620 g"`
/// - range: `"-20 .. 60 degC"`
/// - vec3: `"0 0 1"`・`"100 100 20 mm"`
/// - quat: `"1 0 0 0"`（w x y z）
/// - curve: `"0:480 10:360 20:150 (km^2/s^2 -> kg)"`（`引数:値` の並びと、括弧の中に引数の単位 `->` 値の単位）
/// - bool: `"true"` / `"false"`、enum / text / table: そのまま
pub fn value_from_text(text: &str, value_type: ValueType) -> Result<Value, ValueError> {
    let text = text.trim();
    match value_type {
        ValueType::Scalar => {
            let (value, unit) = parse_number_unit(text)?;
            Ok(Value::Scalar(Scalar::new(value, unit)))
        }
        ValueType::Range => parse_range(text).map(Value::Range),
        ValueType::Curve => parse_curve(text).map(Value::Curve),
        ValueType::Vec3 => parse_vector(text).map(Value::Vec3),
        ValueType::Quat => parse_quaternion(text).map(Value::Quat),
        ValueType::Bool => match text {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Err(ValueError::Syntax(text.to_string())),
        },
        ValueType::Enum => Ok(Value::Enum(text.to_string())),
        ValueType::Text => Ok(Value::Text(text.to_string())),
        ValueType::Table => Ok(Value::Table(text.to_string())),
        ValueType::Tensor3 => Err(ValueError::Type(
            "Cannot enter a tensor as text (pass it as JSON)".to_string(),
        )),
    }
}

/// 曲線（`"0:480 10:360 (km^2/s^2 -> kg)"`）。単位は括弧の中（引数の単位 `->` 値の単位。どちらも省略できる）。
fn parse_curve(text: &str) -> Result<Curve, ValueError> {
    let syntax = || ValueError::Syntax(text.to_string());
    let (body, units) = match text.split_once('(') {
        Some((body, rest)) => (body, rest.strip_suffix(')').ok_or_else(syntax)?),
        None => (text, ""),
    };
    let (at_unit, unit) = match units.split_once("->") {
        Some((a, b)) => (a.trim(), b.trim()),
        None if units.trim().is_empty() => ("", ""),
        None => return Err(syntax()),
    };
    let (mut at, mut values) = (Vec::new(), Vec::new());
    for pair in body.split(|c: char| c.is_whitespace() || c == ',') {
        if pair.is_empty() {
            continue;
        }
        let (a, v) = pair.split_once(':').ok_or_else(syntax)?;
        at.push(a.parse::<f64>().map_err(|_| syntax())?);
        values.push(v.parse::<f64>().map_err(|_| syntax())?);
    }
    if at.is_empty() {
        return Err(syntax());
    }
    Ok(Curve {
        at,
        at_unit: at_unit.to_string(),
        values,
        unit: unit.to_string(),
    })
}

fn parse_range(text: &str) -> Result<RangeValue, ValueError> {
    let (left, right) = text
        .split_once("..")
        .ok_or_else(|| ValueError::Syntax(text.to_string()))?;
    let (min, left_unit) = parse_number_unit(left)?;
    let (max, unit) = parse_number_unit(right)?;
    if !left_unit.is_empty() && left_unit != unit {
        return Err(ValueError::Syntax(format!(
            "{text} (both ends of the range must use the same unit)"
        )));
    }
    Ok(RangeValue { min, max, unit })
}

/// 空白またはカンマ区切りの数値列と、末尾の単位。
fn split_numbers(text: &str) -> Result<(Vec<f64>, String), ValueError> {
    let mut numbers = Vec::new();
    let mut unit = String::new();
    for token in text.split(|c: char| c.is_whitespace() || c == ',') {
        if token.is_empty() {
            continue;
        }
        match token.parse::<f64>() {
            Ok(n) if unit.is_empty() => numbers.push(n),
            _ if unit.is_empty() => unit = token.to_string(),
            _ => return Err(ValueError::Syntax(text.to_string())),
        }
    }
    Ok((numbers, unit))
}

fn parse_vector(text: &str) -> Result<Vector3, ValueError> {
    match split_numbers(text)? {
        (n, unit) if n.len() == 3 => Ok(Vector3 {
            x: n[0],
            y: n[1],
            z: n[2],
            unit: (!unit.is_empty()).then_some(unit),
            frame: None,
        }),
        _ => Err(ValueError::Syntax(text.to_string())),
    }
}

fn parse_quaternion(text: &str) -> Result<Quaternion, ValueError> {
    match split_numbers(text)? {
        (n, unit) if n.len() == 4 && unit.is_empty() => Ok(Quaternion {
            w: n[0],
            x: n[1],
            y: n[2],
            z: n[3],
        }),
        _ => Err(ValueError::Syntax(text.to_string())),
    }
}

/// 量（`"12 V"`・`"-5 deg"`）。単位が読めなければ [`ValueError::Unit`]（次元は見ない）。
pub fn quantity_from_text(text: &str) -> Result<Quantity, ValueError> {
    let (value, unit) = parse_number_unit(text)?;
    unit_info(&unit)?;
    Ok(Quantity::new(value, unit))
}

/// カンマ区切りの一覧（`"off,on"`・`"mass, power"`）→ 語の列（空白を除き、空の語は捨てる）。
/// **一覧の読み方はここだけ**（CLI の旗・GUI の欄・設定の一覧で共通。fune-service が `edit::list_from_text` として再公開する）。
pub fn list_from_text(text: &str) -> Vec<String> {
    text.split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 設定の値（`fune setting set <パス> <値>`）。`like`（設定の今の値）と同じ型で読む。今の値が無ければ文字列から推し量る
/// （整数 → 小数 → 真偽 → 量 → 文字列の順。量は単位が読めるときだけ）。
pub fn setting_from_text(
    text: &str,
    like: Option<&SettingValue>,
) -> Result<SettingValue, ValueError> {
    let t = text.trim();
    let syntax = || ValueError::Syntax(t.to_owned());
    Ok(match like {
        Some(SettingValue::Number(_)) => SettingValue::Number(t.parse().map_err(|_| syntax())?),
        Some(SettingValue::Integer(_)) => SettingValue::Integer(t.parse().map_err(|_| syntax())?),
        Some(SettingValue::Bool(_)) => SettingValue::Bool(t.parse().map_err(|_| syntax())?),
        Some(SettingValue::Quantity(_)) => SettingValue::Quantity(quantity_from_text(t)?),
        Some(SettingValue::Vector(_)) => SettingValue::Vector(parse_vector(t)?),
        Some(SettingValue::Quaternion(_)) => SettingValue::Quaternion(parse_quaternion(t)?),
        Some(SettingValue::List(_)) => SettingValue::List(setting_list_from_text(t)?),
        Some(SettingValue::Curve(_)) => SettingValue::Curve(parse_curve(t)?),
        Some(SettingValue::Text(_)) => SettingValue::Text(t.to_owned()),
        None => guess_setting(t),
    })
}

/// 設定の一覧の値。主な書き方はカンマ区切り（`A, B, C`。`list_from_text` と同じ）。`[` で始まれば TOML の文字列の配列
/// （`["A", "B"]`）として読む（要素は文字列だけ。空の配列は空の一覧）。
fn setting_list_from_text(t: &str) -> Result<Vec<String>, ValueError> {
    if !t.starts_with('[') {
        return Ok(list_from_text(t));
    }
    #[derive(serde::Deserialize)]
    struct Wrapper {
        v: Vec<String>,
    }
    toml::from_str::<Wrapper>(&format!("v = {t}"))
        .map(|w| w.v)
        .map_err(|_| ValueError::Syntax(t.to_owned()))
}

fn guess_setting(t: &str) -> SettingValue {
    if let Ok(i) = t.parse::<i64>() {
        return SettingValue::Integer(i);
    }
    if let Ok(x) = t.parse::<f64>() {
        return SettingValue::Number(x);
    }
    if let Ok(b) = t.parse::<bool>() {
        return SettingValue::Bool(b);
    }
    match quantity_from_text(t) {
        Ok(q) => SettingValue::Quantity(q),
        Err(_) => SettingValue::Text(t.to_owned()),
    }
}

/// 地方時 `"HH:MM"`（`"10:30"`・`"9:05"`。時は 0〜23、分は 2 桁の 0〜59）→ 時（10.5）。読めなければ None。
/// 降交点・昇交点の地方時を検証（`check`）と軌道の組み立て（projection）の両方がこれで読む。
pub fn parse_clock_hours(text: &str) -> Option<f64> {
    let (h, m) = text.trim().split_once(':')?;
    if m.len() != 2 {
        return None;
    }
    match (h.parse::<u32>(), m.parse::<u32>()) {
        (Ok(h), Ok(m)) if h < 24 && m < 60 => Some(f64::from(h) + f64::from(m) / 60.0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_hours_read_hh_mm_only() {
        assert_eq!(parse_clock_hours("10:30"), Some(10.5));
        assert_eq!(parse_clock_hours(" 9:06 "), Some(9.1));
        assert_eq!(parse_clock_hours("24:00"), None);
        assert_eq!(parse_clock_hours("10:3"), None);
        assert_eq!(parse_clock_hours("10.5"), None);
    }

    /// 曲線の文字列は `引数:値` の並びと、括弧の中の単位（引数の単位 `->` 値の単位）。点の無い・形の違う文字列は誤り。
    #[test]
    fn curve_text_reads_points_and_units() {
        let c = value_from_text("0:480, 10:360 20:150 (km^2/s^2 -> kg)", ValueType::Curve);
        assert_eq!(
            c.ok(),
            Some(Value::Curve(Curve {
                at: vec![0.0, 10.0, 20.0],
                at_unit: "km^2/s^2".into(),
                values: vec![480.0, 360.0, 150.0],
                unit: "kg".into(),
            }))
        );
        for bad in [
            "",
            "0 480",
            "0:480 (kg)",
            "0:480 (a -> b",
            "a:1",
            "0:480 10:",
        ] {
            assert!(value_from_text(bad, ValueType::Curve).is_err(), "{bad}");
        }
    }

    #[test]
    fn number_and_unit_split() {
        assert_eq!(parse_number_unit("620 g").ok(), Some((620.0, "g".into())));
        assert_eq!(parse_number_unit("0.3").ok(), Some((0.3, String::new())));
        assert_eq!(
            parse_number_unit("-20degC").ok(),
            Some((-20.0, "degC".into()))
        );
        assert_eq!(
            parse_number_unit("1e-3 N*m").ok(),
            Some((1e-3, "N*m".into()))
        );
        assert!(parse_number_unit("g 620").is_err());
    }

    #[test]
    fn typed_text_values() {
        let r = value_from_text("-20 .. 60 degC", ValueType::Range);
        assert_eq!(
            r.ok(),
            Some(Value::Range(RangeValue {
                min: -20.0,
                max: 60.0,
                unit: "degC".into()
            }))
        );
        let v = value_from_text("100 100 20 mm", ValueType::Vec3);
        assert!(matches!(v, Ok(Value::Vec3(Vector3 { z, .. })) if z == 20.0));
        assert!(value_from_text("1 0 0 0", ValueType::Quat).is_ok());
    }

    /// 書いた文字列を同じ型で読み直すと元の値に戻る（`to_cli` の値の引数が `fune set` でそのまま使える）。
    #[test]
    fn values_survive_text_and_back() {
        let scalar = |v: f64, u: &str| Value::Scalar(Scalar::new(v, u));
        let values = [
            scalar(620.0, "g"),
            scalar(-20.5, "degC"),
            scalar(1e-7, "N*m"),
            scalar(0.3, ""),
            scalar(18.0, "mN*m*s"),
            Value::Range(RangeValue {
                min: -20.0,
                max: 60.0,
                unit: "degC".into(),
            }),
            Value::Range(RangeValue {
                min: 0.1,
                max: 0.9,
                unit: String::new(),
            }),
            Value::Vec3(Vector3 {
                x: 100.0,
                y: -0.5,
                z: 20.0,
                unit: Some("mm".into()),
                frame: None,
            }),
            Value::Vec3(Vector3::unitless(0.0, 0.0, 1.0)),
            Value::Quat(Quaternion {
                w: std::f64::consts::FRAC_1_SQRT_2,
                x: 0.0,
                y: 0.0,
                z: -std::f64::consts::FRAC_1_SQRT_2,
            }),
            Value::Bool(true),
            Value::Enum("al6061_t6".into()),
            Value::Text("EM 実測".into()),
            Value::Table("rw_torque".into()),
            Value::Curve(Curve {
                at: vec![0.0, 10.5, 30.0],
                at_unit: "km^2/s^2".into(),
                values: vec![480.0, 360.25, 1e-3],
                unit: "kg".into(),
            }),
            Value::Curve(Curve {
                at: vec![1.0],
                at_unit: String::new(),
                values: vec![2.0],
                unit: String::new(),
            }),
        ];
        for v in &values {
            let text = value_to_text(v).unwrap();
            assert_eq!(
                &value_from_text(&text, v.value_type()).unwrap(),
                v,
                "{text}"
            );
        }
        assert_eq!(value_to_text(&scalar(620.0, "g")).as_deref(), Some("620 g"));
        assert_eq!(value_to_text(&values[5]).as_deref(), Some("-20 .. 60 degC"));
        // 状態ごとの値は 1 セルずつスカラーとして書いて読む
        let mut cells = std::collections::BTreeMap::new();
        for (state, w) in [("on", 1.2), ("peak", 3.5)] {
            cells.insert(
                state.to_owned(),
                super::super::Authored::bare(Scalar::new(w, "W")),
            );
        }
        assert_eq!(value_to_text(&Value::ByState(cells.clone())), None);
        for cell in cells.values() {
            let text = value_to_text(&Value::Scalar(cell.value.clone())).unwrap();
            assert_eq!(
                value_from_text(&text, ValueType::Scalar).unwrap(),
                Value::Scalar(cell.value.clone())
            );
        }
    }

    /// 設定の値は、今の値の型で読み直すと元に戻る。今の値が無ければ書いた形から推し量る。
    #[test]
    fn settings_survive_text_and_back() {
        let settings = [
            SettingValue::Number(0.25),
            SettingValue::Number(4.0),
            SettingValue::Integer(-3),
            SettingValue::Bool(false),
            SettingValue::Text("10:30".into()),
            SettingValue::Quantity(Quantity::new(500.0, "km")),
            SettingValue::Quantity(Quantity::new(-5.0, "deg")),
            SettingValue::Vector(Vector3 {
                x: 0.0,
                y: 0.1,
                z: -0.2,
                unit: Some("m".into()),
                frame: None,
            }),
            SettingValue::Quaternion(Quaternion::IDENTITY),
            SettingValue::List(vec!["TOKYO_GS".into(), "OKINAWA".into()]),
        ];
        for s in &settings {
            let text = setting_to_text(s);
            assert_eq!(&setting_from_text(&text, Some(s)).unwrap(), s, "{text}");
        }
        assert_eq!(setting_to_text(&settings[5]), "500 km");
        assert_eq!(
            setting_from_text("4", None).unwrap(),
            SettingValue::Integer(4)
        );
        assert_eq!(
            setting_from_text("0.25", None).unwrap(),
            SettingValue::Number(0.25)
        );
        assert_eq!(setting_from_text("-5 deg", None).unwrap(), settings[6]);
        // 単位として読めなければ文字列
        assert_eq!(
            setting_from_text("5 parsec", None).unwrap(),
            SettingValue::Text("5 parsec".into())
        );
        assert!(matches!(
            setting_from_text("5 parsec", Some(&settings[5])),
            Err(ValueError::Unit { .. })
        ));
        assert!(setting_from_text("abc", Some(&settings[0])).is_err());
    }

    /// 設定の一覧は、カンマ区切りでも TOML の配列でも同じ値になる。配列の要素は文字列だけ。
    #[test]
    fn setting_lists_read_commas_or_a_toml_array() {
        let like = SettingValue::List(vec![]);
        let want = SettingValue::List(vec!["A".into(), "B 2".into()]);
        for text in [
            "A, B 2",
            "[\"A\", \"B 2\"]",
            "['A','B 2']",
            " [ \"A\" , \"B 2\" ] ",
        ] {
            assert_eq!(
                setting_from_text(text, Some(&like)).unwrap(),
                want,
                "{text}"
            );
        }
        assert_eq!(
            setting_from_text("[]", Some(&like)).unwrap(),
            SettingValue::List(vec![])
        );
        for bad in ["[1, 2]", "[\"A\"", "[A, B]"] {
            assert!(setting_from_text(bad, Some(&like)).is_err(), "{bad}");
        }
    }

    #[test]
    fn lists_split_on_commas_only() {
        assert_eq!(list_from_text(" a, b ,,c "), ["a", "b", "c"]);
        assert_eq!(list_to_text(["off", "on"]), "off,on");
        assert_eq!(list_from_text(&list_to_text(["off", "on"])), ["off", "on"]);
        assert!(list_from_text(" ").is_empty());
    }
}
