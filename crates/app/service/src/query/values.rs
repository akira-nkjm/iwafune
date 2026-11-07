//! 解決済みの値・書かれた量 → 表示用の型。表示文字列はすべて `crate::units`（中身は `sat_units::format`）で作る。

use crate::dto::{QuantityView, ValueData, ValueView};
use crate::units::{number_text, quantity_auto, quantity_from_authored, quantity_from_si};
use fune_model::resolve::{ResolvedValue, SiScalar, SiValue};
use fune_model::value::{Quantity, Value};
use std::collections::BTreeMap;

/// 書かれた量 `{ value, unit }` を表示用に（表示は書かれた単位のまま）。単位が読めなければ数値と単位をそのまま出す。
pub(crate) fn authored_quantity(q: &Quantity, dimension: Option<&str>) -> QuantityView {
    quantity_from_authored(q.value, &q.unit, dimension).unwrap_or_else(|_| QuantityView {
        value: q.value,
        unit: q.unit.clone(),
        si: q.value,
        dimension: dimension.unwrap_or_default().to_owned(),
        display: format!("{} {}", number_text(q.value), q.unit),
    })
}

/// SI 値を、次元名があれば自動の単位で、無ければ表示単位で整形する。
fn si_view(si: f64, rv: &ResolvedValue) -> QuantityView {
    match rv.dimension.as_deref() {
        Some(d) => quantity_auto(si, d),
        None => quantity_from_si(si, Some(rv.display_unit.as_str()), None),
    }
}

/// 解決済みの値 1 つを表示用に。
pub(crate) fn value_view(attr: &str, rv: &ResolvedValue) -> ValueView {
    let value = match rv.value.get() {
        None => ValueData::Unset,
        Some(v) => value_data(v, rv),
    };
    let display = display_of(&value);
    ValueView {
        attr: attr.to_owned(),
        value,
        display,
        origin: rv.origin,
        evidence: rv.evidence,
        source: rv.from.as_ref().and_then(|f| f.source.clone()),
        note: rv.from.as_ref().and_then(|f| f.note.clone()),
        file: rv.loc.as_ref().map(|l| l.file.clone()),
        line: rv.loc.as_ref().and_then(|l| l.line),
    }
}

fn scalar_data(s: &SiScalar, rv: &ResolvedValue) -> ValueData {
    ValueData::Scalar {
        nominal: si_view(s.value, rv),
        min: s.min.map(|v| si_view(v, rv)),
        max: s.max.map(|v| si_view(v, rv)),
    }
}

fn value_data(v: &SiValue, rv: &ResolvedValue) -> ValueData {
    match v {
        SiValue::Scalar(s) => scalar_data(s, rv),
        SiValue::ByState(m) => ValueData::ByState {
            states: m
                .iter()
                .map(|(state, s)| (state.clone(), si_view(s.value, rv)))
                .collect::<BTreeMap<_, _>>(),
        },
        SiValue::Vec3(si) => {
            let (x, y, z, unit, frame) = match &rv.authored {
                Some(Value::Vec3(a)) => (
                    a.x,
                    a.y,
                    a.z,
                    a.unit.clone().unwrap_or_default(),
                    a.frame.clone(),
                ),
                _ => (si[0], si[1], si[2], String::new(), None),
            };
            ValueData::Vector {
                x,
                y,
                z,
                unit,
                si: *si,
                frame,
            }
        }
        SiValue::Quat([w, x, y, z]) => ValueData::Quat {
            w: *w,
            x: *x,
            y: *y,
            z: *z,
        },
        SiValue::Tensor3(m) => ValueData::Tensor {
            m: *m,
            unit: rv.display_unit.clone(),
        },
        SiValue::Range { min, max } => ValueData::Range {
            min: si_view(*min, rv),
            max: si_view(*max, rv),
        },
        SiValue::Enum(s) => ValueData::Enum { value: s.clone() },
        SiValue::Bool(b) => ValueData::Bool { value: *b },
        SiValue::Text(s) => ValueData::Text { value: s.clone() },
        SiValue::Table(s) => ValueData::Table { id: s.clone() },
    }
}

/// 値の表示文字列。
pub(crate) fn display_of(v: &ValueData) -> String {
    match v {
        ValueData::Unset => "未設定".to_owned(),
        ValueData::Scalar { nominal, min, max } => match (min, max) {
            (None, None) => nominal.display.clone(),
            (lo, hi) => format!(
                "{} ({} … {})",
                nominal.display,
                lo.as_ref().map_or("—", |q| q.display.as_str()),
                hi.as_ref().map_or("—", |q| q.display.as_str())
            ),
        },
        ValueData::ByState { states } => states
            .iter()
            .map(|(s, q)| format!("{s} {}", q.display))
            .collect::<Vec<_>>()
            .join(" / "),
        ValueData::Vector { x, y, z, unit, .. } => {
            let n = |v: &f64| number_text(*v);
            let body = format!("({}, {}, {})", n(x), n(y), n(z));
            if unit.is_empty() {
                body
            } else {
                format!("{body} {unit}")
            }
        }
        ValueData::Quat { w, x, y, z } => format!(
            "[{}, {}, {}, {}]",
            number_text(*w),
            number_text(*x),
            number_text(*y),
            number_text(*z)
        ),
        ValueData::Tensor { .. } => "3×3".to_owned(),
        ValueData::Range { min, max } => format!("{} … {}", min.display, max.display),
        ValueData::Enum { value } | ValueData::Text { value } => value.clone(),
        ValueData::Bool { value } => value.to_string(),
        ValueData::Table { id } => format!("table:{id}"),
    }
}
