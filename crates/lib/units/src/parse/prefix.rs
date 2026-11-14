//! SI 接頭辞の表と、接頭辞を付けてよい単位の規則。**接頭辞の読み方はここだけ。**
//!
//! 規則（[`split_prefix`]）:
//! - 完全一致する記号（`m`・`min`・`h`・`T`・`G` など）は [`crate::named_unit`] が先に引くので、接頭辞としては読まない。
//!   したがって `h` は時間、`T` はテスラ、`m` はメートルのまま。接頭辞は完全一致が無かった綴りにだけ使う。
//! - 接頭辞は 1 文字で、先頭の文字はどれも異なる（最長一致の曖昧さは無い）。`da`・`P`・`E` などは持たない
//!   （`Pa` はパスカルの完全一致で、`P`（ペタ）と読む余地を作らない）。
//! - 接頭辞を付けてよいのは [`accepts_prefix`] の単位だけ。すでに接頭辞を含む記号（`kg`・`mAh`・`mL`・`nT`・`uT`・`kbps`）、
//!   倍率の無い記号（`one`・`1`・`%`・`ppm`）、時間の慣用単位（`min`・`h`・`d`・`day`）、`rpm`・`atm`・`arcmin` には付けない
//!   （`kkg`・`mmAh`・`k1`・`kh` は拒否）。
//! - 倍率 100 の `h`（ヘクト）は `hPa`・`hL` に限る。`hm`・`hs` のような綴りは時間の `h` との取り違えになりやすいため。

/// 接頭辞の記号と倍率。大きい順（先頭の文字はすべて異なる）。
const PREFIXES: [(&str, f64); 12] = [
    ("T", 1e12),
    ("G", 1e9),
    ("M", 1e6),
    ("k", 1e3),
    ("h", 1e2),
    ("c", 1e-2),
    ("m", 1e-3),
    ("µ", 1e-6),
    ("u", 1e-6),
    ("n", 1e-9),
    ("p", 1e-12),
    ("f", 1e-15),
];

/// 接頭辞を付けてはいけない記号。表の記号（[`crate::NAMED_UNIT_SYMBOLS`]）のうち、これ以外には付けてよい。
///
/// affine 単位（`degC`・`degF`）と対数単位（`dB` など）はここに含めない。これらは接頭辞を付けると
/// 専用のエラー（[`crate::UnitError::AffineInArithmetic`]・[`crate::UnitError::LogarithmicInArithmetic`]）にする。
pub(super) const NO_PREFIX: &[&str] = &[
    // 倍率の無い記号・比
    "one", "1", "%", "ppm", //
    // すでに接頭辞を含む記号
    "kg", "mAh", "mL", "nT", "uT", "kbps", //
    // 時間の慣用単位・回転数・気圧・分角
    "min", "h", "d", "day", "rpm", "atm", "arcmin",
];

/// ヘクトを付けてよい単位。
const HECTO_UNITS: &[&str] = &["Pa", "L"];

/// 記号 `rest` に接頭辞 `prefix` を付けてよいか（`rest` は表にある記号）。
pub(super) fn accepts_prefix(prefix: &str, rest: &str) -> bool {
    if NO_PREFIX.contains(&rest) {
        return false;
    }
    prefix != "h" || HECTO_UNITS.contains(&rest)
}

/// 記号の先頭の接頭辞を取り出す。`(接頭辞, 倍率, 残り)`。接頭辞が無ければ `None`。
pub(super) fn split_prefix(symbol: &str) -> Option<(&'static str, f64, &str)> {
    PREFIXES.iter().find_map(|&(prefix, factor)| {
        symbol
            .strip_prefix(prefix)
            .map(|rest| (prefix, factor, rest))
    })
}

#[cfg(test)]
mod tests;
