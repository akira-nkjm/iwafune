//! 部品の取付と、機体座標での位置・姿勢の表示。

use fune_model::value::Vector3;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 取付先と、取付点に対する位置・姿勢（取付のある部品だけ）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct MountView {
    /// 取付先の部品
    pub on: String,
    /// 取付点（書かれていなければ取付先の原点）
    pub point: Option<String>,
    /// 取付点に対する相対位置 [m]（書かれていなければ原点）
    pub position: [f64; 3],
    /// 取付点に対する相対位置の書かれたまま（値と単位）。GUI はこの単位で見せて送る（ファイルの単位を保つ。CLI で同じ単位を打つのと同じ）
    pub authored_position: Option<Vector3>,
    /// 取付点に対する相対姿勢 `[w, x, y, z]`（書かれていなければ恒等）
    pub orientation: [f64; 4],
}

/// 機体座標での位置・姿勢（取付の連鎖をたどったもの）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct BodyPoseView {
    /// 機体座標での位置 [m]
    pub position: [f64; 3],
    /// 機体座標での姿勢 `[w, x, y, z]`
    pub orientation: [f64; 4],
    /// 取付の根（取付先を持たず、ほかの部品がこれに取り付く。機体座標の原点・恒等で、動かせない）
    pub root: bool,
}
