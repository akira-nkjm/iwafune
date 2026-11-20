//! SVG の文字列の部品（座標の整形・XML の退避・図の幅）。

/// 図の幅（viewBox の単位）。
pub(crate) const WIDTH: f64 = 640.0;
/// 左の余白（縦軸の目盛りと題）。
pub(crate) const LEFT: f64 = 66.0;
/// 右の余白。
pub(crate) const RIGHT: f64 = 14.0;

/// 座標の文字列（小数 2 桁に丸めて末尾の 0 を省く）。
pub(crate) fn num(v: f64) -> String {
    let t = format!("{v:.2}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    if t == "-0" || t.is_empty() {
        "0".into()
    } else {
        t.into()
    }
}

/// XML の文字の退避。
pub(crate) fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
