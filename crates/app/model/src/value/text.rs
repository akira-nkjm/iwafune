//! 人が打つ値の文字列（CLI の `fune set RW_X.mass "620 g"`・GUI の入力欄）→ [`Value`]。
//!
//! 単位の部分はそのまま文字列で残す（解析と次元の検査は `check` と [`super::units`]）。

use super::{Quaternion, RangeValue, Scalar, Value, ValueError, ValueType, Vector3};

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
/// - bool: `"true"` / `"false"`、enum / text / table: そのまま
pub fn value_from_text(text: &str, value_type: ValueType) -> Result<Value, ValueError> {
    let text = text.trim();
    match value_type {
        ValueType::Scalar => {
            let (value, unit) = parse_number_unit(text)?;
            Ok(Value::Scalar(Scalar::new(value, unit)))
        }
        ValueType::Range => parse_range(text).map(Value::Range),
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
            "テンソルは文字列から入力できません（JSON で渡してください）".to_string(),
        )),
    }
}

fn parse_range(text: &str) -> Result<RangeValue, ValueError> {
    let (left, right) = text
        .split_once("..")
        .ok_or_else(|| ValueError::Syntax(text.to_string()))?;
    let (min, left_unit) = parse_number_unit(left)?;
    let (max, unit) = parse_number_unit(right)?;
    if !left_unit.is_empty() && left_unit != unit {
        return Err(ValueError::Syntax(format!(
            "{text}（範囲の両端の単位をそろえてください）"
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
}
