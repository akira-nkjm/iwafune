//! メッシュの表示用の形。メッシュ上の場（モードの変位、パネルの温度分布など）を持つ解析が、重い任意の出力
//! （`Analysis::run_detailed` の `detail`）に載せる幾何をここで 1 つに定める。Web は 1 つの 3D の部品でこれを描く。
//!
//! 座標は機体の座標系 [m]。表示用なので `f32`（相対 1e-7 は表示に十分で、JSON が小さくなる）。

use serde::{Deserialize, Serialize};

/// 面のメッシュ（節点と、パネルごとの四角形）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SurfaceMesh {
    /// 節点の位置 [m]（機体の座標系）。場の値はこの並びの節点番号に対応する
    pub nodes: Vec<[f32; 3]>,
    /// パネルごとの四角形
    pub groups: Vec<SurfaceGroup>,
    /// 面でない線（フレームの部材など）の両端の節点番号。面のメッシュだけの解析では空で、出力に出さない
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<[u32; 2]>>"))]
    pub lines: Vec<[u32; 2]>,
}

/// 1 枚のパネルの四角形。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SurfaceGroup {
    /// パネルの ID
    pub id: String,
    /// 四角形の節点番号（`SurfaceMesh::nodes` の添字。周回の向きは面の法線に対して左回り）
    pub quads: Vec<[u32; 4]>,
}
