//! 値 1 つの検査（値の型・状態の軸・単位・次元・値域・方向ベクトル）。位置は呼び出し側が付ける。

use crate::design::AttributeDef;
use crate::diagnostic::Severity;
use crate::resolve::{SiValue, si_value};
use crate::value::units::{dimension_label, parse_dimension_name, same_dimension, unit_info};
use crate::value::{Quantity, Value, ValueError, ValueType};

/// 位置なしの診断（重さ・コード・本文）。
pub type Finding = (Severity, &'static str, String);

/// 値を属性定義と照らして検査する。`states` は部品の種別の状態の語彙。
pub fn check_value_against(def: &AttributeDef, value: &Value, states: &[String]) -> Vec<Finding> {
    let mut out = Vec::new();
    if let Some(f) = type_finding(def, value) {
        out.push(f);
        return out;
    }
    if let Value::ByState(cells) = value {
        for state in cells.keys() {
            if !states.iter().any(|s| s == state) {
                let message = if states.is_empty() {
                    format!(
                        "A value is given for state `{state}`, but this part's kind has no states"
                    )
                } else {
                    format!(
                        "State `{state}` is not in this part's kind ({})",
                        states.join(" / ")
                    )
                };
                out.push((Severity::Error, "state.unknown", message));
            }
        }
    }
    for unit in units_of(value) {
        out.extend(unit_findings(def, &unit));
    }
    if out.iter().any(|f| f.0 == Severity::Error) {
        return out;
    }
    out.extend(shape_findings(def, value));
    out
}

/// 値の型と状態の軸の一致。
fn type_finding(def: &AttributeDef, value: &Value) -> Option<Finding> {
    let found = value.value_type();
    let text_like = |t: ValueType| matches!(t, ValueType::Enum | ValueType::Text);
    if matches!(value, Value::ByState(_)) && !def.by_state {
        return Some((
            Severity::Error,
            "value.axis_mismatch",
            format!(
                "Per-state values are given, but attribute `{}` is not looked up by state",
                def.id
            ),
        ));
    }
    if found == def.value_type || (text_like(found) && text_like(def.value_type)) {
        return None;
    }
    Some((
        Severity::Error,
        "value.type_mismatch",
        format!(
            "The value type of attribute `{}` is {:?}, but {:?} is given",
            def.id, def.value_type, found
        ),
    ))
}

/// 値に書かれた単位（無次元のベクトルは空文字列）。
fn units_of(value: &Value) -> Vec<String> {
    match value {
        Value::Scalar(s) => vec![s.unit.clone()],
        Value::ByState(m) => {
            let mut units: Vec<String> = m.values().map(|c| c.value.unit.clone()).collect();
            units.dedup();
            units
        }
        Value::Vec3(v) => vec![v.unit.clone().unwrap_or_default()],
        Value::Tensor3(t) => vec![t.unit.clone()],
        Value::Range(r) => vec![r.unit.clone()],
        _ => Vec::new(),
    }
}

/// 属性定義の次元（`dimension` か、`units` の先頭の単位の次元）を単位と比べる。
fn unit_findings(def: &AttributeDef, unit: &str) -> Vec<Finding> {
    let info = match unit_info(unit) {
        Ok(info) => info,
        Err(e) => return vec![(Severity::Error, "unit.parse", e.to_string())],
    };
    let expected = match (&def.dimension, def.units.first()) {
        (Some(name), _) => parse_dimension_name(name).ok(),
        (None, Some(first)) => unit_info(first).ok().map(|i| i.dimension),
        (None, None) => None,
    };
    let mut out = Vec::new();
    if let Some(want) = expected
        && !info.dimension.is_compatible_with(want)
    {
        out.push((
            Severity::Error,
            "unit.dimension",
            format!(
                "Unit `{unit}` is {}, but attribute `{}` is {}",
                dimension_label(info.dimension),
                def.id,
                dimension_label(want)
            ),
        ));
        return out;
    }
    if !def.units.is_empty() && !def.units.iter().any(|u| u == unit) {
        out.push((
            Severity::Warning,
            "unit.not_allowed",
            format!(
                "Unit `{unit}` is not among the allowed units of attribute `{}` ({})",
                def.id,
                def.units.join(", ")
            ),
        ));
    }
    out
}

/// 範囲の向き・不確かさの順序・値域・方向ベクトル・列挙の語彙。
fn shape_findings(def: &AttributeDef, value: &Value) -> Vec<Finding> {
    let mut out = Vec::new();
    let si = match si_value(value) {
        Ok(si) => si,
        Err(e) => return vec![(Severity::Error, "unit.parse", e.to_string())],
    };
    let mut nominals: Vec<f64> = Vec::new();
    match &si {
        SiValue::Scalar(s) => {
            nominals.push(s.value);
            uncertainty(&mut out, s);
        }
        SiValue::ByState(m) => {
            for s in m.values() {
                nominals.push(s.value);
                uncertainty(&mut out, s);
            }
        }
        SiValue::Range { min, max } => {
            if min > max {
                out.push((
                    Severity::Error,
                    "value.range_inverted",
                    format!(
                        "The range lower bound {min} is greater than the upper bound {max} (SI)"
                    ),
                ));
            }
        }
        SiValue::Vec3(v) if def.unit_vector => out.extend(direction(v)),
        SiValue::Enum(s) | SiValue::Text(s)
            if def.value_type == ValueType::Enum
                && !def.symbols.is_empty()
                && !def.symbols.contains(s) =>
        {
            out.push((
                Severity::Error,
                "enum.unknown",
                format!(
                    "`{s}` is not in the vocabulary of attribute `{}` ({})",
                    def.id,
                    def.symbols.join(", ")
                ),
            ));
        }
        _ => {}
    }
    out.extend(nominals.into_iter().filter_map(|v| domain(def, v)));
    out
}

/// 値域（SI）の外なら warning。
fn domain(def: &AttributeDef, v: f64) -> Option<Finding> {
    let outside = def.min.is_some_and(|m| v < m) || def.max.is_some_and(|m| v > m);
    outside.then(|| {
        (
            Severity::Warning,
            "value.out_of_domain",
            format!(
                "Value {v} (SI) is outside the range of attribute `{}`: [{}, {}]",
                def.id,
                def.min.map_or("-∞".into(), |m| m.to_string()),
                def.max.map_or("∞".into(), |m| m.to_string())
            ),
        )
    })
}

fn uncertainty(out: &mut Vec<Finding>, s: &crate::resolve::SiScalar) {
    let below = s.min.is_some_and(|m| m > s.value);
    let above = s.max.is_some_and(|m| m < s.value);
    if below || above {
        out.push((
            Severity::Warning,
            "value.uncertainty_order",
            "The uncertainty does not satisfy min ≤ value ≤ max".to_string(),
        ));
    }
}

fn direction(v: &[f64; 3]) -> Option<Finding> {
    let norm = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if norm == 0.0 {
        Some((
            Severity::Error,
            "vector.zero",
            "The direction vector is 0".to_string(),
        ))
    } else if (norm - 1.0).abs() > super::UNIT_NORM_TOLERANCE {
        Some((
            Severity::Warning,
            "vector.not_unit",
            format!("The direction vector does not have norm 1 ({norm:.4})"),
        ))
    } else {
        None
    }
}

/// 量の次元を検査する。`spec` は次元名（`"Length"`）か、名前の無い次元なら基準の単位（`"W/(m*K)"`）。
pub fn check_quantity_dimension(q: &Quantity, spec: &str) -> Option<Finding> {
    let result: Result<bool, ValueError> = match parse_dimension_name(spec) {
        Ok(want) => unit_info(&q.unit).map(|i| i.dimension.is_compatible_with(want)),
        Err(_) => same_dimension(&q.unit, spec),
    };
    match result {
        Ok(true) => None,
        Ok(false) => Some((
            Severity::Error,
            "unit.dimension",
            format!(
                "Unit `{}` is {}, but {spec} is required",
                q.unit,
                unit_info(&q.unit).map_or_else(|_| "?".into(), |i| dimension_label(i.dimension))
            ),
        )),
        Err(e) => Some((Severity::Error, "unit.parse", e.to_string())),
    }
}
