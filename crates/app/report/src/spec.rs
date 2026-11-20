//! グラフの入力（サービスが組み立てる。描画は [`crate::chart`]）。

/// 1 本の系列（`x` と `y` は同じ長さ。有限でない点は飛ばす）。
#[derive(Debug, Clone)]
pub struct Series {
    pub label: String,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

/// 許容範囲（片方だけでもよい。破線で引く）。
#[derive(Debug, Clone)]
pub struct Limit {
    pub label: String,
    pub lo: Option<f64>,
    pub hi: Option<f64>,
}

/// 縦に並べる 1 段。
#[derive(Debug, Clone)]
pub struct Panel {
    /// 縦軸の題（単位つき）
    pub y_label: String,
    pub series: Vec<Series>,
    pub limits: Vec<Limit>,
}

/// グラフ全体。
#[derive(Debug, Clone)]
pub struct Chart {
    /// 横軸の題（単位つき）
    pub x_label: String,
    pub panels: Vec<Panel>,
    /// 影をつける横軸の区間（蝕）
    pub shade: Vec<(f64, f64)>,
    /// 影の凡例の文字（影が無ければ使わない）
    pub shade_label: String,
}
