//! 量の表示文字列（表示単位と SI 接頭辞の選択）。**量の整形はここだけ。**
//!
//! SI の値と次元名から、読みやすい表示単位を選んで `12.3 mW` のような文字列を作る。
//! 換算はすべて [`crate::parse_unit`] が返す [`Unit`] で行う（単位の表を別に持たない）。
//! ここにあるのは「次元ごとに、どの単位の並びから選ぶか」という表示の好みだけで、
//! 並びの各要素が解析でき次元が合うことはテストで固定する。
//!
//! - 選び方: 並び（小さい順）のうち、3 桁に丸めた値の絶対値がしきい値（ふつう 1）以上になる最大の単位。
//!   どれも満たさなければ最小の単位。0・非有限は基準の単位。
//! - Energy と Torque は次元が同じなので、次元名で区別する（[`format_quantity`]）。
//!   [`Dimension`] だけから整形する [`format_dimension`] は Energy として扱う。
//! - 温度は絶対温度として `degC` で表示する。温度差は [`format_in_unit`] で `K` を指定する。
//! - 比（[`Fraction`]）は次元名の代わりに種類名 `Fraction` を渡すと % で表示する。

use crate::parse::dimension_name_eq;
use crate::{Dimension, Unit, UnitError, named_dimensions, parse_dimension, parse_unit};
use alloc::format;
use alloc::string::String;
use preferences::{PREFERENCES, Preference};

mod label;
mod preferences;

pub use label::unit_label;

/// 表示の有効数字。
pub const SIGNIFICANT_DIGITS: u32 = 3;

/// 整形した量（整形の関数の戻り値）。これ自体は API に出さず、アプリ層が SI 値・次元と合わせて表示用の型にする。
#[derive(Debug, Clone, PartialEq)]
pub struct FormattedQuantity {
    /// 表示単位での値（丸める前）
    pub value: f64,
    /// 表示単位（通常は parse_unit で解析できる式。dB 出力の種類は表示専用の記号、無次元は空文字列）
    pub unit: String,
    /// 表示文字列（例: `"12.3 mW"`）
    pub text: String,
}

/// 次元の名前（[`named_dimensions`] の綴り）。最初に一致したものを返す（Energy と Torque は Energy）。
pub fn dimension_name(dim: Dimension) -> Option<&'static str> {
    named_dimensions()
        .iter()
        .find(|(_, d)| d.is_compatible_with(dim))
        .map(|(name, _)| *name)
}

/// 次元の表示名: 名前のある次元はその名前（[`dimension_name`]）、無ければ SI の組立単位の式。
pub fn dimension_label(dim: Dimension) -> String {
    dimension_name(dim).map_or_else(|| si_unit_expression(dim), String::from)
}

/// 次元の表示名。`preferred`（呼び出し側が知っている次元名）がその次元の名前として正しければ、その正規の綴り
/// （[`canonical_dimension_name`]）を使い、違えば [`dimension_label`]。**同じ次元の別名（Energy と Torque）を
/// 選ぶ規則はここだけ**（次元だけからは Energy になるため、名前を知っている側が渡す）。
pub fn dimension_label_preferring(dim: Dimension, preferred: Option<&str>) -> String {
    if let Some(p) = preferred
        && parse_dimension(p).is_ok_and(|d| d == dim)
    {
        return canonical_dimension_name(p).unwrap_or(p).into();
    }
    dimension_label(dim)
}

/// 次元名の代わりに使える表示の種類（次元は無次元。比を % で出す [`Fraction`]）。
const KINDS: &[&str] = &[
    crate::display_kind::FRACTION,
    crate::display_kind::DECIBEL,
    crate::display_kind::DECIBEL_WATT,
    crate::display_kind::DECIBEL_PER_KELVIN,
    crate::display_kind::DECIBEL_HERTZ,
];

/// 次元名を正規の綴りにする（`angular_momentum` → `AngularMomentum`、別名 `current` → `ElectricCurrent`）。
/// 表示の種類（`Fraction`）はその名前のまま返す。
pub fn canonical_dimension_name(name: &str) -> Option<&'static str> {
    let name = name.trim();
    if let Some(kind) = KINDS.iter().find(|k| dimension_name_eq(name, k)) {
        return Some(kind);
    }
    named_dimensions()
        .iter()
        .find(|(candidate, _)| dimension_name_eq(name, candidate))
        .map(|(candidate, _)| *candidate)
        .or_else(|| parse_dimension(name).ok().and_then(dimension_name))
}

/// 次元名に対する表示単位の候補（小さい順）。GUI の単位の選択肢などに使う。
pub fn display_units(dimension: &str) -> Option<impl Iterator<Item = &'static str>> {
    let canonical = canonical_dimension_name(dimension)?;
    let p = PREFERENCES.iter().find(|p| p.name == canonical)?;
    Some(p.units.iter().map(|(u, _)| *u))
}

/// 次元名 `dimension` の入力の例（`1 kg`）。人が値を打つときの書式の見本で、単位は SI の 1 を表示するときに選ばれる単位
/// （[`format_quantity`] の表示単位。表を別に持たない）。無次元など単位を書かない次元、読めない次元名は None。
pub fn input_example(dimension: &str) -> Option<String> {
    let unit = format_quantity(1.0, dimension).ok()?.unit;
    (!unit.is_empty()).then(|| format!("1 {unit}"))
}

/// SI の値 `si_value` を、次元名 `dimension`（例: `"Power"`）に合う表示単位で整形する。
pub fn format_quantity(si_value: f64, dimension: &str) -> Result<FormattedQuantity, UnitError> {
    let canonical = canonical_dimension_name(dimension).ok_or(UnitError::UnknownDimension)?;
    let symbol = match canonical {
        crate::display_kind::DECIBEL => Some("dB"),
        crate::display_kind::DECIBEL_WATT => Some("dBW"),
        crate::display_kind::DECIBEL_PER_KELVIN => Some("dB/K"),
        crate::display_kind::DECIBEL_HERTZ => Some("dB·Hz"),
        _ => None,
    };
    if let Some(symbol) = symbol {
        return Ok(build(si_value, symbol));
    }
    match PREFERENCES.iter().find(|p| p.name == canonical) {
        Some(p) => format_with_preference(si_value, p),
        None => {
            let dim = parse_dimension(canonical)?;
            format_in_unit(si_value, &si_unit_expression(dim))
        }
    }
}

/// [`Dimension`] だけから整形する（次元名は [`dimension_name`] で決める）。名前の無い次元は SI の組立単位で出す。
pub fn format_dimension(si_value: f64, dim: Dimension) -> FormattedQuantity {
    let by_name = dimension_name(dim).and_then(|n| format_quantity(si_value, n).ok());
    by_name
        .or_else(|| format_in_unit(si_value, &si_unit_expression(dim)).ok())
        .unwrap_or_else(|| FormattedQuantity {
            value: si_value,
            unit: String::new(),
            text: format_number(si_value, SIGNIFICANT_DIGITS),
        })
}

/// 指定した単位式 `unit` で整形する（接頭辞は選ばない）。対数単位では SI の値が正でないとエラー。
pub fn format_in_unit(si_value: f64, unit: &str) -> Result<FormattedQuantity, UnitError> {
    let u = parse_unit(unit)?;
    let value = to_unit(&u, si_value)?;
    Ok(build(value, unit.trim()))
}

fn to_unit(u: &Unit, si_value: f64) -> Result<f64, UnitError> {
    if u.is_logarithmic() && si_value.partial_cmp(&0.0) != Some(core::cmp::Ordering::Greater) {
        return Err(UnitError::NonPositiveForLogarithmic);
    }
    Ok(u.from_si(si_value))
}

fn format_with_preference(si_value: f64, p: &Preference) -> Result<FormattedQuantity, UnitError> {
    if si_value == 0.0 || !si_value.is_finite() {
        return format_in_unit(si_value, p.base);
    }
    let mut chosen: Option<(f64, &str)> = None;
    for (symbol, threshold) in p.units {
        let value = to_unit(&parse_unit(symbol)?, si_value)?;
        let fits = libm::fabs(round_significant(value, SIGNIFICANT_DIGITS)) >= *threshold;
        if fits || chosen.is_none() {
            chosen = Some((value, symbol));
        }
        if !fits {
            break;
        }
    }
    let (value, symbol) = chosen.ok_or(UnitError::UnknownUnit)?;
    Ok(build(value, symbol))
}

fn build(value: f64, unit: &str) -> FormattedQuantity {
    let number = format_number(value, SIGNIFICANT_DIGITS);
    let text = if unit.is_empty() {
        number
    } else {
        match unit_label(unit).as_str() {
            // 角度の度は数に続けて書く（`97.6°`）。`°C` などは空ける
            "°" => format!("{number}°"),
            label => format!("{number} {label}"),
        }
    };
    FormattedQuantity {
        value,
        unit: String::from(unit),
        text,
    }
}

/// 有効数字 `significant` 桁に丸める。
fn round_significant(value: f64, significant: u32) -> f64 {
    if value == 0.0 || !value.is_finite() {
        return value;
    }
    let exponent = libm::floor(libm::log10(libm::fabs(value)));
    let scale = libm::pow(10.0, f64::from(significant) - 1.0 - exponent);
    libm::round(value * scale) / scale
}

/// 数値を有効数字 `significant` 桁の文字列にする（整数部の桁は落とさない。極端な大きさは指数表記）。
pub fn format_number(value: f64, significant: u32) -> String {
    if value.is_nan() {
        return String::from("NaN");
    }
    if value.is_infinite() {
        return String::from(if value > 0.0 { "inf" } else { "-inf" });
    }
    let significant = significant.max(1);
    let rounded = round_significant(value, significant);
    if rounded == 0.0 {
        return String::from("0");
    }
    let magnitude = libm::fabs(rounded);
    if !(1e-3..1e9).contains(&magnitude) {
        return format!("{:.*e}", (significant - 1) as usize, rounded);
    }
    let exponent = libm::floor(libm::log10(magnitude)) as i32;
    if exponent >= significant as i32 {
        // 整数部が有効数字より長いときは整数に丸めるだけにする（123456.7 → 123457）。
        return format!("{:.0}", libm::round(value));
    }
    let decimals = (significant as i32 - 1 - exponent).max(0) as usize;
    format!("{rounded:.decimals$}")
}

/// 次元を SI の基本単位の式にする（例: Power → `kg*m^2/s^3`）。[`crate::parse_unit`] で解析できる形。
pub fn si_unit_expression(dim: Dimension) -> String {
    const SYMBOLS: [&str; 9] = ["m", "kg", "s", "A", "K", "mol", "cd", "rad", "bit"];
    let (mut num, mut den) = (String::new(), String::new());
    for (symbol, exp) in SYMBOLS.iter().zip(dim.exponents()) {
        let (n, d) = (exp.numerator(), exp.denominator());
        if n == 0 {
            continue;
        }
        let target = if n > 0 { &mut num } else { &mut den };
        if !target.is_empty() {
            target.push('*');
        }
        let n_abs = n.unsigned_abs();
        let part = match (n_abs, d) {
            (1, 1) => String::from(*symbol),
            (_, 1) => format!("{symbol}^{n_abs}"),
            _ => format!("{symbol}^({n_abs}/{d})"),
        };
        target.push_str(&part);
    }
    match (num.is_empty(), den.is_empty()) {
        (true, true) => String::new(),
        (false, true) => num,
        (true, false) if den.contains('*') => format!("1/({den})"),
        (true, false) => format!("1/{den}"),
        (false, false) if den.contains('*') => format!("{num}/({den})"),
        (false, false) => format!("{num}/{den}"),
    }
}

#[cfg(test)]
mod tests;
