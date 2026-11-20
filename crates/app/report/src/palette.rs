//! 図の色・線の型。**図の色はここだけで決める**（Web の `lib/tone.ts` とは別。印刷の図は白地・細い線で、白黒でも見分けられるよう線の型も変える）。

/// 系列の色（色覚の違いでも見分けやすい 8 色）。
pub const SERIES: [&str; 8] = [
    "#0072B2", "#D55E00", "#009E73", "#CC79A7", "#E69F00", "#56B4E9", "#000000", "#999900",
];

/// 系列の線の型（`stroke-dasharray`。空は実線）。色を使い切ったら次の型へ進む。
pub const DASHES: [&str; 4] = ["", "6 3", "2 2", "8 3 2 3"];

/// 軸・枠の色。
pub const AXIS: &str = "#333333";
/// 目盛りの横線の色。
pub const GRID: &str = "#DDDDDD";
/// 影（蝕）の塗り。
pub const SHADE: &str = "#E6E6E6";
/// 許容範囲の線の色。
pub const LIMIT: &str = "#444444";
/// 文字の色。
pub const TEXT: &str = "#111111";

/// 系列 `i` の（色, 線の型）。
pub fn series_style(i: usize) -> (&'static str, &'static str) {
    (
        SERIES[i % SERIES.len()],
        DASHES[(i / SERIES.len()) % DASHES.len()],
    )
}
