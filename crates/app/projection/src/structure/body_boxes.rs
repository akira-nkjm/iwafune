//! 部品の箱（`size`）を機体座標に置いたもの。**部品の機体座標での位置・姿勢と箱の読み出しはここだけ**:
//! 質量特性（[`super::items`]）・干渉の確認（[`super::interference`]）が共有し、打ち上げの外形（#374）や粗い有限要素のモデル（#370）も
//! ここから箱を取る。姿勢は `ctx.resolved`（`pose::body_pose`）からだけ読むので、形態を選べるようになれば、ここが読む設計を差し替えるだけで追従する。

use crate::context::Context;
use crate::pose;
use fune_model::resolve::ResolvedPart;
use sat_analysis::structure::interference::InterferenceBox;
use sat_astro::math::{Quat, Vec3};

/// 部品 1 つの機体座標での位置・姿勢と、箱の辺の長さ（`size` があれば）。
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub id: String,
    /// 部品の原点（箱の中心）の位置 [m]
    pub position: Vec3,
    /// 部品 → 機体座標系の回転
    pub orientation: Quat,
    /// 箱の辺の長さ（部品座標系）[m]
    pub size: Option<Vec3>,
}

/// 部品の置き場所と箱。取付が無ければ原点・恒等姿勢。
pub fn placed(ctx: &Context<'_>, p: &ResolvedPart) -> Placed {
    let (position, orientation) =
        pose::body_pose(ctx.resolved, &p.id).unwrap_or((Vec3::ZERO, Quat::IDENTITY));
    Placed {
        id: p.id.clone(),
        position,
        orientation,
        size: ctx.vec3(p, "size").map(|s| Vec3::new(s[0], s[1], s[2])),
    }
}

/// 機体座標に置いた箱 1 つ（寸法を持つ部品）。
#[derive(Debug, Clone, PartialEq)]
pub struct BodyBox {
    pub id: String,
    /// 員数（同じ位置・姿勢に重ねて置く台数）
    pub count: u32,
    pub position: Vec3,
    pub orientation: Quat,
    pub size: Vec3,
    /// 取付先をたどった部品の ID（近いほうから。寸法を持たない部品も含む）
    pub mount_ancestors: Vec<String>,
}

/// 解析の入力の箱にする（干渉の確認・円錐の遮り・外形の面積が同じ箱の入力を使う。変換はここだけ）。
impl From<BodyBox> for InterferenceBox {
    fn from(b: BodyBox) -> Self {
        Self {
            id: b.id,
            count: b.count,
            position: b.position,
            orientation: b.orientation,
            size: b.size,
            mount_ancestors: b.mount_ancestors,
        }
    }
}

/// 設計の全部品の箱と、寸法を持たない部品。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BodyBoxes {
    /// 寸法を持つ部品の箱（部品 ID の昇順）
    pub boxes: Vec<BodyBox>,
    /// 寸法を持たない部品の ID（昇順）
    pub unsized_parts: Vec<String>,
}

/// 設計の全部品を箱として機体座標に置く。寸法を持たない部品は `unsized_parts` に入れる（不足入力にはしない）。
pub fn boxes(ctx: &Context<'_>) -> BodyBoxes {
    let mut out = BodyBoxes::default();
    for p in ctx.resolved.parts.values() {
        let at = placed(ctx, p);
        match at.size {
            Some(size) => out.boxes.push(BodyBox {
                id: at.id,
                count: p.count_or_one(),
                position: at.position,
                orientation: at.orientation,
                size,
                mount_ancestors: ctx
                    .resolved
                    .mount_ancestors(&p.id)
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            }),
            None => out.unsized_parts.push(at.id),
        }
    }
    out
}
