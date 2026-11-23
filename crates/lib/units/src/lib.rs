//! # sat-units — 単位カーネル（単位・次元の唯一の正本）
//!
//! no_std の単位カーネル。依存は core のみに頼る
//! `typenum`（型レベル整数）と `libm`（対数単位の `exp10`/`log10` と分数冪の `pow`）だけ。
//!
//! 対数単位（`dB`/`dBi`/`dBm`/`dBW`/`mag`）は実行時の [`Unit`] にだけあり、[`typed`] の
//! コンパイル時の次元型には無い。単独でのみ有効で、境界で SI の線形値に直す。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `dimension` | 実行時の次元（9 つの基本次元の有理数の指数）`Dimension` |
//! | `dimensions` | 型レベル次元と別名の共有定義 |
//! | `display_kind` | 表示種別（`Fraction`・`Decibel*`）の共通名 |
//! | `typed` | コンパイル時の次元解析つきの型付き量 `Qty` |
//! | `quantity` | 単位付きの量のメタ情報（`Quantity`・`Measurement`） |
//! | `unit` | 単位 `Unit` と名前付きの単位の表（SI との換算） |
//! | `parse` | 単位式と次元名の文字列解析 |
//! | `format` | 量の表示文字列（表示単位と SI 接頭辞の選択。量の整形はここだけ） |
//! | `logarithmic` | 対数単位（dB など）の線形値での補助 |
//! | `rational` | 次元の指数に使う有理数 `Rational` |
//! | `last_dimension` | 直前に JSON へ書き出した `Qty` の次元名の記録（JSON の次元記録。`serde` と `alloc` のとき） |
#![cfg_attr(not(feature = "ts"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;
// 直前に書き出した次元の記録（`last_dimension`）はスレッドローカルを使うので std が要る。
// serde（std 版）を有効にしたときだけ作る。
#[cfg(all(feature = "serde", feature = "alloc"))]
extern crate std;

mod dimension;
#[cfg(feature = "alloc")]
pub mod format;
#[cfg(all(feature = "serde", feature = "alloc"))]
mod last_dimension;
mod logarithmic;
mod parse;
mod quantity;
mod rational;
pub mod typed;
mod unit;

pub use dimension::{BaseDimension, DimError, Dimension};
#[cfg(all(feature = "serde", feature = "alloc"))]
pub use last_dimension::take_last_dimension;
pub use logarithmic::{
    apply_db_gain, db_to_ratio, magnitude_to_ratio, ratio_to_db, ratio_to_magnitude,
};
pub use parse::{named_dimensions, parse_dimension, parse_unit};
pub use quantity::{Measurement, Quantity};
pub use rational::Rational;
pub use unit::{NAMED_UNIT_SYMBOLS, Unit, UnitError, named_unit};

pub mod display_kind;

pub mod dimensions;
