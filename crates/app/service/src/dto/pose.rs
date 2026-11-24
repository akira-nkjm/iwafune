//! 部品の取付と、機体座標での位置・姿勢の表示。

use super::QuantityView;
use fune_model::value::{Quaternion, Vector3};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 位置と姿勢の表示（作るのは `query::pose`）。位置は同じ単位の X・Y・Z、姿勢は固定軸 X → Y → Z の回転角（度。
/// 換算は `sat_astro::math::Quat::to_fixed_xyz`）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PoseQuantities {
    /// 位置の X・Y・Z（3 つとも同じ単位）
    pub position: [QuantityView; 3],
    /// 親の座標軸 X・Y・Z まわりの回転角（X → Y → Z の順に回す。X・Z は (−180°, 180°]、Y は [−90°, 90°]）
    pub rotation: [QuantityView; 3],
}

/// 部品の取付点（ほかの部品を付ける点。`mount_points.<名前>`）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct MountPointView {
    /// 取付点の名前
    pub name: String,
    /// 部品の座標系での位置（書かれた単位）・姿勢
    pub pose: PoseQuantities,
}

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
    /// 取付点に対する相対姿勢の書かれたまま（書かれていなければ null）。GUI は角度を変えなければこれをそのまま送る
    pub authored_orientation: Option<Quaternion>,
    /// 取付点の位置・姿勢（取付先の座標系。取付点が無ければ原点・恒等）。位置の単位は `relative`・機体座標と同じ
    pub point_pose: PoseQuantities,
    /// 取付点に対する位置（書かれていなければ 0）・姿勢（書かれていなければ 0°）
    pub relative: PoseQuantities,
    /// 問い合わせた形態がこの部品の姿勢を上書きしているか（基準を読んだときは常に false。`authored_*`・`relative` は上書きの値）
    pub overridden: bool,
}

/// 機体座標での位置・姿勢（取付の連鎖をたどったもの。取付の根は自分の `[pose]`）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct BodyPoseView {
    /// 機体座標での位置 [m]
    pub position: [f64; 3],
    /// 機体座標での姿勢 `[w, x, y, z]`
    pub orientation: [f64; 4],
    /// 取付の根（取付先を持たず、ほかの部品がこれに取り付く）。根も `[pose]` で動かせる（根の取付点の座標系は機体座標）
    pub root: bool,
    /// 取付の無い部品（根）の `[pose]` の位置の書かれたまま（値と単位。書かれていなければ欄を出さない）。
    /// 根では機体座標での位置と同じで、GUI の配置の欄の初期値になる（取付のある部品は `MountView.authored_position`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub authored_position: Option<Vector3>,
    /// 取付の無い部品（根）の `[pose]` の姿勢の書かれたまま（書かれていなければ欄を出さない）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub authored_orientation: Option<Quaternion>,
    /// 機体座標での位置・姿勢の表示（位置の単位は取付点に対する位置と同じ）
    pub quantities: PoseQuantities,
}
