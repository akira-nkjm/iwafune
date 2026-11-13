//! 値 → 人が打つ文字列（`fune set` の値・`fune setting set` の値・旗の値。GUI の「CLI としてコピー」も同じ）。
//! [`super`] の読み方の逆で、同じ型として読み直すと元の値に戻る（往復のテストは [`super`] の tests）。

use crate::value::{Quantity, Quaternion, RangeValue, SettingValue, Value, Vector3};

/// 数（`0.25`・`-5`・`1e-7`）。Rust の最短の往復表記（読み直すと同じ f64）。
pub fn number_to_text(value: f64) -> String {
    format!("{value}")
}

/// 数と単位（`"620 g"`。無次元なら数だけ）。[`super::parse_number_unit`] の逆。
pub fn number_unit_to_text(value: f64, unit: &str) -> String {
    if unit.is_empty() {
        number_to_text(value)
    } else {
        format!("{} {unit}", number_to_text(value))
    }
}

/// 量（`"5 V"`）。出典（`from`）は書かない（旗 `--source`・`--note` で別に書く）。
pub fn quantity_to_text(q: &Quantity) -> String {
    number_unit_to_text(q.value, &q.unit)
}

/// ベクトル（`"100 100 20 mm"`。単位が無ければ 3 つの数）。座標系（`frame`）は書かない。
pub fn vector_to_text(v: &Vector3) -> String {
    let mut s = format!(
        "{} {} {}",
        number_to_text(v.x),
        number_to_text(v.y),
        number_to_text(v.z)
    );
    if let Some(u) = &v.unit {
        s.push(' ');
        s.push_str(u);
    }
    s
}

/// クォータニオン（`"1 0 0 0"`。w x y z の順）。
pub fn quaternion_to_text(q: &Quaternion) -> String {
    [q.w, q.x, q.y, q.z].map(number_to_text).join(" ")
}

/// 範囲（`"-20 .. 60 degC"`。単位は右の端にだけ書く）。
pub fn range_to_text(r: &RangeValue) -> String {
    format!(
        "{} .. {}",
        number_to_text(r.min),
        number_unit_to_text(r.max, &r.unit)
    )
}

/// 一覧（`"off,on"`）。[`super::list_from_text`] の逆（語にカンマや前後の空白が無いとき）。
pub fn list_to_text<S: AsRef<str>>(items: impl IntoIterator<Item = S>) -> String {
    items
        .into_iter()
        .map(|s| s.as_ref().to_owned())
        .collect::<Vec<_>>()
        .join(",")
}

/// 属性の値（[`super::value_from_text`] で同じ型として読み直せる文字列）。スカラーは nominal だけ（不確かさは
/// `--min`・`--max` に [`number_unit_to_text`] で書く）。状態ごとの値（1 状態ずつ書く）とテンソル（JSON で渡す）は
/// 1 つの文字列にならないので None。
pub fn value_to_text(value: &Value) -> Option<String> {
    Some(match value {
        Value::Scalar(s) => number_unit_to_text(s.value, &s.unit),
        Value::Vec3(v) => vector_to_text(v),
        Value::Quat(q) => quaternion_to_text(q),
        Value::Range(r) => range_to_text(r),
        Value::Enum(s) | Value::Text(s) | Value::Table(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::ByState(_) | Value::Tensor3(_) => return None,
    })
}

/// 設定の値（[`super::setting_from_text`] に今の値を渡すと同じ値に読み直せる文字列）。
pub fn setting_to_text(value: &SettingValue) -> String {
    match value {
        SettingValue::Number(n) => number_to_text(*n),
        SettingValue::Integer(i) => i.to_string(),
        SettingValue::Bool(b) => b.to_string(),
        SettingValue::Text(s) => s.clone(),
        SettingValue::Quantity(q) => quantity_to_text(q),
        SettingValue::Vector(v) => vector_to_text(v),
        SettingValue::Quaternion(q) => quaternion_to_text(q),
        SettingValue::List(l) => list_to_text(l),
    }
}
