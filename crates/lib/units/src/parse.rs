//! 単位式と次元名の文字列解析。
//!
//! SI 接頭辞の表と、接頭辞を付けてよい単位の規則は `prefix`（完全一致の記号が先で、`kkg`・`kh` などは拒否）。
//!
//! [`parse_unit`] は SI 接頭辞付きの記号を `*`（乗算）・`/`（除算）・`^`（冪）で
//! つないだ式（`N*m`、`W/K`、`kg/m^3`、`m^-3`）を左から右へ畳み込む。括弧は入れ子にでき、
//! 括弧の外では従来どおり左結合（`W/K*s` は `(W/K)*s`）。空文字列は無次元。affine 単位（`degC`/`degF`）
//! は単独でのみ有効で、式や接頭辞と組み合わせると [`UnitError::AffineInArithmetic`]。
//! 対数単位（`dB`/`dBi`/`dBm`/`dBW`/`mag`）も同じく単独でのみ有効で、`dBm/Hz` のような式や
//! `kdB` のような接頭辞は [`UnitError::LogarithmicInArithmetic`]。
//!
//! 冪は符号付き整数（`m^-3`）か、括弧で囲んだ整数・分数（`Hz^(1/2)`、`s^(-1/2)`、`m^(2)`）。
//! 雑音密度（`V/Hz^(1/2)`）や角度ランダムウォーク（`deg/h^(1/2)`）を綴るためのもの。
//! 小数（`Hz^0.5`）は 1/3 のような指数を丸めてしまうので受けず、[`UnitError::DecimalExponent`]
//! で `^(1/2)` を案内する。括弧の無い `s^-1/2` は従来どおり `(s^-1)/2` と読む。
//!
//! `rad` に倍率が 1 より大きい接頭辞を付けた記号（`krad` / `Mrad` / `Grad`）は
//! [`UnitError::AmbiguousRadian`]。放射線分野の `krad`（= 10 Gy）が黙って角度になるのを防ぐ。
//! `mrad` / `urad` は角度として受ける。
//!
//! [`parse_dimension`] の語彙は型レベル次元 [`crate::typed`] のエイリアス集合そのもので、
//! 綴りは PascalCase / snake_case / camelCase を同一視する（`AngularVelocity` =
//! `angular_velocity`）。両者が食い違わないことはテストで固定する。
//!
//! `no_std`・アロケーション無し（入力の部分文字列とスタック上の畳み込みだけで動く）。

use crate::{Dimension, Rational, Unit, UnitError, named_unit};

const DIMENSIONLESS_UNIT: Unit = Unit::new(Dimension::DIMENSIONLESS, 1.0, 0.0);

mod names;
mod prefix;

pub(crate) use names::dimension_name_eq;
pub use names::{named_dimensions, parse_dimension};

/// 単位式を解析する。文法は本モジュールの冒頭 doc を参照。
pub fn parse_unit(input: &str) -> Result<Unit, UnitError> {
    let expression = input.trim();
    if expression.is_empty() {
        return Ok(DIMENSIONLESS_UNIT);
    }
    if !expression.contains(['*', '/', '^', '(', ')']) {
        return parse_symbol(expression);
    }
    Parser::new(expression).parse()
}

/// `no_std`・アロケーション無しの再帰下降パーサ。
///
/// `*` と `/` は同じ優先順位で左結合、括弧だけが評価順を変える。外部入力でスタックを
/// 枯渇させないよう、括弧は 32 段までに制限する。
struct Parser<'a> {
    source: &'a str,
    position: usize,
    depth: u8,
}

impl<'a> Parser<'a> {
    const MAX_NESTING: u8 = 32;

    const fn new(source: &'a str) -> Self {
        Self {
            source,
            position: 0,
            depth: 0,
        }
    }

    fn parse(mut self) -> Result<Unit, UnitError> {
        let unit = self.parse_expression(false)?;
        self.skip_whitespace();
        if self.position == self.source.len() {
            Ok(unit)
        } else {
            Err(UnitError::UnknownUnit)
        }
    }

    fn parse_expression(&mut self, nested: bool) -> Result<Unit, UnitError> {
        let mut accumulated = self.parse_factor()?;
        loop {
            self.skip_whitespace();
            match self.peek_byte() {
                Some(b'*') | Some(b'/') => {
                    let divide = self.peek_byte() == Some(b'/');
                    self.position += 1;
                    let right = self.parse_factor()?;
                    accumulated = if divide {
                        accumulated.try_div(&right)?
                    } else {
                        accumulated.try_mul(&right)?
                    };
                }
                Some(b')') if nested => return Ok(accumulated),
                None => return Ok(accumulated),
                _ => return Err(UnitError::UnknownUnit),
            }
        }
    }

    fn parse_factor(&mut self) -> Result<Unit, UnitError> {
        self.skip_whitespace();
        let unit = if self.peek_byte() == Some(b'(') {
            if self.depth == Self::MAX_NESTING {
                return Err(UnitError::UnknownUnit);
            }
            self.depth += 1;
            self.position += 1;
            let nested = self.parse_expression(true)?;
            if self.peek_byte() != Some(b')') {
                return Err(UnitError::UnknownUnit);
            }
            self.position += 1;
            self.depth -= 1;
            nested
        } else {
            self.parse_atomic_symbol()?
        };

        self.skip_whitespace();
        if self.peek_byte() != Some(b'^') {
            return Ok(unit);
        }
        self.position += 1;
        self.skip_whitespace();
        if self.peek_byte() == Some(b'(') {
            // 括弧付きの冪は整数か分数だけ。中で `*` や入れ子の括弧は使えない。
            self.position += 1;
            let start = self.position;
            while let Some(byte) = self.peek_byte() {
                if matches!(byte, b')' | b'(') {
                    break;
                }
                self.position += 1;
            }
            if self.peek_byte() != Some(b')') {
                return Err(UnitError::UnknownUnit);
            }
            let exponent = parse_rational_exponent(&self.source[start..self.position])?;
            self.position += 1;
            return raise_rational(unit, exponent);
        }
        let start = self.position;
        while let Some(byte) = self.peek_byte() {
            if matches!(byte, b'*' | b'/' | b')' | b'(') {
                break;
            }
            self.position += 1;
        }
        raise(unit, parse_exponent(&self.source[start..self.position])?)
    }

    fn parse_atomic_symbol(&mut self) -> Result<Unit, UnitError> {
        let start = self.position;
        while let Some(byte) = self.peek_byte() {
            if matches!(byte, b'*' | b'/' | b'^' | b'(' | b')') {
                break;
            }
            self.position += 1;
        }
        let unit = parse_symbol(self.source[start..self.position].trim())?;
        // affine と対数は完全に単独の入力でのみ有効。括弧や演算子を含む経路では拒否する。
        if unit.is_affine() {
            return Err(UnitError::AffineInArithmetic);
        }
        if unit.is_logarithmic() {
            return Err(UnitError::LogarithmicInArithmetic);
        }
        Ok(unit)
    }

    fn skip_whitespace(&mut self) {
        while self
            .source
            .as_bytes()
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }

    fn peek_byte(&self) -> Option<u8> {
        self.source.as_bytes().get(self.position).copied()
    }
}

/// 単位を整数冪に上げる。`try_mul`/`try_div` を経由するので、次元指数の溢れと
/// scale のオーバー／アンダーフローは既存のエラーとして返る。
fn raise(unit: Unit, exponent: i8) -> Result<Unit, UnitError> {
    let mut accumulated = DIMENSIONLESS_UNIT;
    let mut remaining = exponent.unsigned_abs();
    while remaining > 0 {
        accumulated = accumulated.try_mul(&unit)?;
        remaining -= 1;
    }
    if exponent < 0 {
        DIMENSIONLESS_UNIT.try_div(&accumulated)
    } else {
        Ok(accumulated)
    }
}

/// 単位を有理数の冪に上げる。整数なら [`raise`] と同じ経路（scale は掛け算で正確）、
/// 分数なら次元を [`Dimension::powr`]・scale を `pow` で上げる（`kHz^(1/2)` は √1000）。
fn raise_rational(unit: Unit, exponent: Rational) -> Result<Unit, UnitError> {
    if exponent.denominator() == 1 {
        return raise(unit, exponent.numerator());
    }
    let dimension = unit.dimension().powr(exponent).map_err(UnitError::Dim)?;
    let power = f64::from(exponent.numerator()) / f64::from(exponent.denominator());
    let scale = libm::pow(unit.scale(), power);
    // 負の scale の分数冪（NaN）やオーバーフローは try_mul と同じエラーにする。
    if !scale.is_finite() || scale == 0.0 {
        return Err(UnitError::ScaleOutOfRange);
    }
    Ok(Unit::new(dimension, scale, 0.0))
}

/// 括弧の中の冪（`1/2`、`-1/2`、`3`）を解析する。分母 0 は拒否する。
fn parse_rational_exponent(source: &str) -> Result<Rational, UnitError> {
    let (numerator, denominator) = match source.split_once('/') {
        Some((numerator, denominator)) => {
            let denominator = parse_exponent(denominator)?;
            (parse_exponent(numerator)?, denominator)
        }
        None => (parse_exponent(source)?, 1),
    };
    Rational::try_new(numerator, denominator).ok_or(UnitError::UnknownUnit)
}

/// `^` の右辺（符号付き 10 進整数）を解析する。
fn parse_exponent(source: &str) -> Result<i8, UnitError> {
    let source = source.trim();
    // `Hz^0.5` のような小数は受けない。黙って UnknownUnit にせず、分数の書き方を案内する。
    if source.contains('.')
        && source
            .bytes()
            .all(|b| b.is_ascii_digit() || b"+-.".contains(&b))
    {
        return Err(UnitError::DecimalExponent);
    }
    let (negative, digits) = match source.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, source.trim_prefix('+')),
    };
    // 3 桁までに制限すれば i8 の範囲検査だけで済む。
    if digits.is_empty() || digits.len() > 3 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(UnitError::UnknownUnit);
    }
    let mut magnitude: i16 = 0;
    for byte in digits.bytes() {
        magnitude = magnitude * 10 + i16::from(byte - b'0');
    }
    if magnitude > i16::from(i8::MAX) {
        return Err(UnitError::UnknownUnit);
    }
    let magnitude = magnitude as i8;
    Ok(if negative { -magnitude } else { magnitude })
}

/// 接頭辞付きの記号 1 個を解析する。接頭辞の表と付けてよい単位の規則は `prefix`。
fn parse_symbol(symbol: &str) -> Result<Unit, UnitError> {
    if let Some(unit) = named_unit(symbol) {
        return Ok(unit);
    }

    let (prefix, factor, rest) = prefix::split_prefix(symbol).ok_or(UnitError::UnknownUnit)?;
    let unit = named_unit(rest).ok_or(UnitError::UnknownUnit)?;
    // `krad` は放射線分野では 1000 rad（= 10 Gy、旧吸収線量単位）。角度の
    // キロラジアンとして黙って通すと TID 耐性が角度になるので、倍率 > 1 は拒否する。
    if rest == "rad" && factor > 1.0 {
        return Err(UnitError::AmbiguousRadian);
    }
    // affine 単位(温度の degC/degF 等)への接頭辞付与は物理的に無意味
    // ("mdegC" など)。算術と同じく拒否し、offset を黙って保持しない。
    if unit.is_affine() {
        return Err(UnitError::AffineInArithmetic);
    }
    // 対数単位への接頭辞("kdB" 等)も同じく無意味。with_scale は基準値を
    // 変えてしまい、別の単位を黙って作るので拒否する。
    if unit.is_logarithmic() {
        return Err(UnitError::LogarithmicInArithmetic);
    }
    // `kkg`・`mmAh`・`kh` のような、接頭辞を付けない単位への接頭辞（規則は `prefix`）。
    if !prefix::accepts_prefix(prefix, rest) {
        return Err(UnitError::UnknownUnit);
    }
    Ok(unit.with_scale(factor))
}

#[cfg(test)]
mod tests;
