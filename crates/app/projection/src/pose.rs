//! 取付の連鎖 ⇄ 機体座標での位置と姿勢。合成と逆変換は sat-astro の数学（`Quat`）で行う（姿勢の合成と逆変換はここだけ）。

use fune_model::resolve::{PoseSi, Resolved};
use sat_astro::math::{Quat, Vec3};

/// 部品の機体座標での原点の位置 [m] と姿勢（部品 → 機体）。部品が無ければ None。取付が無ければ原点・恒等。
pub fn body_pose(resolved: &Resolved, part: &str) -> Option<(Vec3, Quat)> {
    resolved.parts.get(part)?;
    Some(compose(&resolved.pose_chain(part)))
}

/// 部品の取付点の機体座標での位置 [m] と姿勢（取付点 → 機体）。取付が無ければ原点・恒等。
pub fn mount_frame(resolved: &Resolved, part: &str) -> Option<(Vec3, Quat)> {
    let rp = resolved.parts.get(part)?;
    let mut chain = resolved.pose_chain(part);
    if rp.mount.is_some() {
        chain.pop();
    }
    Some(compose(&chain))
}

/// 機体座標の位置 [m]・姿勢（部品 → 機体）→ 取付点に対する相対位置 [m]・姿勢（`body_pose` の逆）。
/// GUI の 3D 配置で動かした結果をファイルの `pose` に直すのに使う。
pub fn body_to_mount_local(
    resolved: &Resolved,
    part: &str,
    position: Vec3,
    orientation: Quat,
) -> Option<(Vec3, Quat)> {
    let (origin, frame) = mount_frame(resolved, part)?;
    let inverse = frame.conjugate();
    Some((
        inverse.rotate(position - origin),
        inverse.mul(orientation).normalized(),
    ))
}

fn compose(chain: &[PoseSi]) -> (Vec3, Quat) {
    let mut position = Vec3::ZERO;
    let mut orientation = Quat::IDENTITY;
    for step in chain {
        let (p, q) = pose_of(step);
        position += orientation.rotate(p);
        orientation = orientation.mul(q).normalized();
    }
    (position, orientation)
}

fn pose_of(p: &PoseSi) -> (Vec3, Quat) {
    let [x, y, z] = p.position;
    let [w, qx, qy, qz] = p.orientation;
    (
        Vec3::new(x, y, z),
        Quat {
            w,
            x: qx,
            y: qy,
            z: qz,
        },
    )
}

/// 部品座標のベクトル → 機体座標（方向。正規化はしない）。
pub fn to_body(resolved: &Resolved, part: &str, v: Vec3) -> Vec3 {
    body_pose(resolved, part).map_or(v, |(_, q)| q.rotate(v))
}
