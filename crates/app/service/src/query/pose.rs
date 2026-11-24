//! 部品の配置の表示（取付・取付点・機体座標の位置と姿勢）。位置は 1 つの部品の中で同じ単位（[`position_unit`]）、
//! 姿勢は固定軸 X → Y → Z の角度（換算は `sat_astro::math::Quat::to_fixed_xyz`）。合成は `fune_projection::pose`。

use crate::dto::{BodyPoseView, MountPointView, MountView, PoseQuantities};
use crate::error::{Result, ServiceError};
use crate::units::quantity_from_si;
use fune_model::design::{DEFAULT_POSITION_UNIT, Design, Part, Pose, position_unit as unit_of};
use fune_model::resolve::{PoseSi, Resolved, ResolvedPart, pose_si};
use sat_astro::math::Quat;

/// 表示の角度の単位。
const ANGLE_UNIT: &str = "deg";
/// 表示の角度の丸め [deg]（換算の丸め誤差で 89.99999999° などと出ないように）。
const ANGLE_STEP_DEG: f64 = 1e-6;
/// これより小さい位置 [m] は 0 と見せる（合成の丸め誤差で 1e-17 m などと出ないように）。
const LENGTH_EPS_M: f64 = 1e-12;

/// 問い合わせの形態が実在するか（`None` は基準で常に実在。無い ID は NotFound。`Resolved::in_configuration` は無い ID を
/// 基準として扱うので、入口でここを通して取り違えを防ぐ）。
pub(crate) fn require_configuration(design: &Design, configuration: Option<&str>) -> Result<()> {
    match configuration {
        Some(id) if !design.configurations.contains_key(id) => {
            Err(ServiceError::not_found("configuration", id))
        }
        _ => Ok(()),
    }
}

/// 形態 `configuration`（`None` は基準）で解決した設計。**サービスが形態を選ぶのはここだけ**（読み取りの配置の表示と、
/// 機体座標での配置の逆変換が通る）。
pub(crate) fn resolved_in<'a>(resolved: &'a Resolved, configuration: Option<&str>) -> &'a Resolved {
    resolved.in_configuration(configuration)
}

/// 形態 `configuration` で有効な、部品に書かれた姿勢（形態が書いていればその行、無ければ部品の `[pose]`）。
/// **サービスが書かれた姿勢を選ぶのはここだけ**（選び方は `Design::pose_in`）。
pub(crate) fn authored_pose<'a>(
    design: &'a Design,
    part: &'a Part,
    configuration: Option<&str>,
) -> Option<&'a Pose> {
    design.pose_in(part, configuration)
}

/// 形態が部品の姿勢を上書きしているか（基準は常に false）。
pub(crate) fn is_overridden(design: &Design, part: &str, configuration: Option<&str>) -> bool {
    configuration
        .and_then(|c| design.configurations.get(c))
        .is_some_and(|c| c.pose.contains_key(part))
}

/// 部品の配置の位置を見せる単位: 取付点に対する位置の書かれた単位 → 取付点の位置の書かれた単位 → 既定（m）。
/// 取付点の位置・取付点に対する位置・機体座標の位置を、この 1 つの単位でそろえる。
pub(crate) fn position_unit(
    design: &Design,
    part: Option<&Part>,
    configuration: Option<&str>,
) -> String {
    let offset = part
        .and_then(|p| authored_pose(design, p, configuration))
        .and_then(|p| p.position.as_ref());
    let point = part.and_then(|p| p.mount.as_ref()).and_then(|m| {
        let parent = design.parts.get(m.on.as_str())?;
        parent
            .mount_points
            .get(m.point.as_deref()?)?
            .position
            .as_ref()
    });
    offset
        .or(point)
        .map_or(DEFAULT_POSITION_UNIT, unit_of)
        .to_owned()
}

/// 取付先と、取付点に対する位置・姿勢（取付のある部品だけ）。
pub(crate) fn mount_view(
    design: &Design,
    part: Option<&Part>,
    rp: &ResolvedPart,
    (unit, configuration): (&str, Option<&str>),
) -> Option<MountView> {
    let m = rp.mount.as_ref()?;
    let pose = part.and_then(|p| authored_pose(design, p, configuration));
    Some(MountView {
        on: m.on.clone(),
        point: m.point.clone(),
        position: m.pose.position,
        authored_position: pose.and_then(|p| p.position.clone()),
        orientation: m.pose.orientation,
        authored_orientation: pose.and_then(|p| p.orientation.clone()),
        point_pose: quantities(&m.point_pose, unit),
        relative: quantities(&m.pose, unit),
        overridden: is_overridden(design, &rp.id, configuration),
    })
}

/// 機体座標での位置・姿勢。取付のある部品と、取付の根（ほかの部品の取付の連鎖の終わり）だけ。
pub(crate) fn body_pose_view(
    resolved: &Resolved,
    rp: &ResolvedPart,
    unit: &str,
) -> Option<BodyPoseView> {
    let root = rp.mount.is_none() && resolved.is_mount_root(&rp.id);
    if rp.mount.is_none() && !root {
        return None;
    }
    // 解決済みの部品なら必ず求まる（`body_pose` が None になるのは部品が無いときだけ）
    let (p, q) = fune_projection::pose::body_pose(resolved, &rp.id)?;
    let pose = PoseSi {
        position: [p.x, p.y, p.z],
        orientation: q.to_wxyz(),
    };
    Some(BodyPoseView {
        position: pose.position,
        orientation: pose.orientation,
        root,
        quantities: quantities(&pose, unit),
    })
}

/// 部品の取付点（名前の順。位置はそれぞれの書かれた単位）。
pub(crate) fn mount_points(part: Option<&Part>) -> Vec<MountPointView> {
    part.map(|p| {
        p.mount_points
            .iter()
            .map(|(name, pose)| {
                let unit = pose
                    .position
                    .as_ref()
                    .map_or(DEFAULT_POSITION_UNIT, unit_of);
                MountPointView {
                    name: name.clone(),
                    pose: quantities(&pose_si(Some(pose)), unit),
                }
            })
            .collect()
    })
    .unwrap_or_default()
}

/// SI の位置・姿勢 → 表示（位置は `unit`、姿勢は度の角度）。
fn quantities(pose: &PoseSi, unit: &str) -> PoseQuantities {
    let length = |si: f64| {
        let si = if si.abs() < LENGTH_EPS_M { 0.0 } else { si };
        quantity_from_si(si, Some(unit), Some("Length"))
    };
    let angles = Quat::from_wxyz(pose.orientation)
        .normalized()
        .to_fixed_xyz();
    PoseQuantities {
        position: pose.position.map(length),
        rotation: angles.map(angle),
    }
}

/// 角度 [rad] → 度の表示（[`ANGLE_STEP_DEG`] に丸め、丸めて −180° になったら 180°）。値も丸めた度（欄の初期値）。
fn angle(rad: f64) -> crate::dto::QuantityView {
    let deg = (rad.to_degrees() / ANGLE_STEP_DEG).round() * ANGLE_STEP_DEG + 0.0;
    let deg = if deg <= -180.0 { deg + 360.0 } else { deg };
    let mut q = quantity_from_si(deg.to_radians(), Some(ANGLE_UNIT), Some("Angle"));
    q.value = deg;
    q
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 手計算: (0.7071, 0, 0.7071, 0) は X 0°・Y 90°・Z 0°。位置は指定の単位で、丸め誤差は 0 に。
    #[test]
    #[allow(clippy::approx_constant)] // ファイルに 6 桁で書く値（0.707107）を確かめる
    fn quantities_show_angles_in_degrees_and_positions_in_one_unit() {
        let s = core::f64::consts::FRAC_1_SQRT_2;
        let pose = PoseSi {
            position: [0.012, 1e-17, -0.1],
            orientation: [s, 0.0, s, 0.0],
        };
        let q = quantities(&pose, "mm");
        let shown: Vec<_> = q.position.iter().map(|v| v.display.as_str()).collect();
        assert_eq!(shown, ["12.0 mm", "0 mm", "-100 mm"]);
        let angles: Vec<_> = q
            .rotation
            .iter()
            .map(|v| (v.value, v.display.as_str()))
            .collect();
        assert_eq!(angles, [(0.0, "0°"), (90.0, "90.0°"), (0.0, "0°")]);
        // ファイルに 6 桁で書いた値（デモの SS_MX）
        let mx = quantities(
            &PoseSi {
                position: [0.0; 3],
                orientation: [0.707107, 0.0, -0.707107, 0.0],
            },
            "m",
        );
        assert_eq!(mx.rotation.map(|v| v.value), [0.0, -90.0, 0.0]);
    }
}
