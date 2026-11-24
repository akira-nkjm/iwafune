//! 板の升の節点の識別（どの板のどの升か）。節点の型（`network`）と範囲の判定（`range`）の両方が使うので、互いに依存しない葉のモジュールに置く。
//! 板の分け方そのものは `network::subdivide`。

use serde::{Deserialize, Serialize};

/// 升の節点が、板のどの升か。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalCell {
    /// 板の部品の ID
    pub part: String,
    /// u 方向の番号（0 始まり）
    pub i: u32,
    /// v 方向の番号
    pub j: u32,
}

impl ThermalCell {
    /// 升の節点の ID（`<部品>[i,j]`）。
    pub fn node_id(&self) -> String {
        format!("{}[{},{}]", self.part, self.i, self.j)
    }
}
