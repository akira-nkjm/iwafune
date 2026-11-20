//! 列の単位の選び方と数値だけのセル。整形は `sat_units::format` と `units`（表示単位の選び方・有効数字 3 桁）に任せ、
//! ここは「列で 1 つの単位にそろえる」方針だけを持つ。

use crate::dto::QuantityView;
use crate::units::{number_text, quantity_auto};
use fune_report::model::Cell;
use sat_units::format::{format_in_unit, format_quantity, unit_label};

/// 値の列の単位を、0 でない値の絶対値が最小のものに合わせる（小さい値が 1 以上で読める単位。質量の列など）。
pub(super) fn pick_unit_smallest(dimension: &str, values: impl IntoIterator<Item = f64>) -> String {
    let smallest = values
        .into_iter()
        .filter(|v| v.is_finite() && *v != 0.0)
        .map(f64::abs)
        .fold(f64::INFINITY, f64::min);
    probe(dimension, smallest)
}

/// 値の列の単位を、絶対値が最大のものに合わせる（大きい値を桁数少なく読める単位。電力の列など）。
pub(super) fn pick_unit_largest(dimension: &str, values: impl IntoIterator<Item = f64>) -> String {
    let largest = values
        .into_iter()
        .filter(|v| v.is_finite())
        .map(f64::abs)
        .fold(f64::NEG_INFINITY, f64::max);
    probe(dimension, largest)
}

fn probe(dimension: &str, value: f64) -> String {
    let value = if value.is_finite() { value } else { 0.0 };
    format_quantity(value, dimension)
        .map(|f| f.unit)
        .unwrap_or_default()
}

/// 単位の表示用の綴り（`degC` → `°C`）。
pub(super) fn label_of(unit: &str) -> String {
    unit_label(unit)
}

/// 単位 `unit` で表した数値だけのセル（SI の値も持つ）。
pub(super) fn number_in(si: f64, unit: &str) -> Cell {
    match format_in_unit(si, unit) {
        Ok(f) => Cell::number(number_text(f.value), si),
        Err(_) => Cell::number(number_text(si), si),
    }
}

/// 次元から表示単位を選んだ量の（数値だけのセル, 単位の表示用の綴り）。
pub(super) fn auto_cell(si: f64, dimension: &str) -> (Cell, String) {
    let q = quantity_auto(si, dimension);
    (Cell::number(number_text(q.value), si), unit_label(&q.unit))
}

/// 量の表示（`display` 文字列）。
pub(super) fn display(si: f64, dimension: &str) -> String {
    quantity_auto(si, dimension).display
}

/// 割合（0〜1）を `%` の数値にした文字列。
pub(super) fn percent_number(ratio: f64) -> String {
    number_text(crate::units::percent(ratio).value)
}

/// `QuantityView` を単位つきの数値セルに。
pub(super) fn view_cell(q: &QuantityView, unit: &str) -> Cell {
    number_in(q.si, unit)
}
