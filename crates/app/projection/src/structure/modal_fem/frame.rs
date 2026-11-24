//! `structure.modal_fem` のフレーム（role = structure）の読み出し。部材の断面（属性 `member_width`、あれば `member_wall`）を持つ
//! フレームは、箱（`size`、機体の軸に平行）の辺の部材をモデルにする（材料は [`super::super::panel`] と同じ読み方）。
//! 断面を持たないフレームは質量だけで、全体に配る（解析の `distributed_mass`）。断面の寸法から面積・断面二次モーメントを出すのは解析（`ModalFemMemberSection::properties`）。

use super::super::body_boxes::placed;
use super::super::panel;
use crate::context::Context;
use crate::missing::MissingInputs;
use crate::outline::axis_aligned_extents;
use crate::parts::{extensive, mass_or_missing};
use fune_model::Target;
use fune_model::builtin::Role;
use sat_analysis::structure::modal_fem::{ModalFemFrame, ModalFemMemberSection};
use sat_units::typed::{Length, Mass, Qty};

/// 部材の断面の属性（外側の辺と、あれば肉厚）。
pub(super) mod attr {
    pub const WIDTH: &str = "member_width";
    pub const WALL: &str = "member_wall";
}

/// 読んだフレームと、部材にしないフレームの質量。
pub(super) struct Frames {
    /// 部材をモデルにするフレーム
    pub frames: Vec<ModalFemFrame>,
    /// 断面を持たないフレームの質量の合計 [kg]（質量だけを全体に配る）
    pub loose_mass: f64,
}

impl Frames {
    /// 質量だけを全体に配るフレームの質量（無ければ `None`）。
    pub fn distributed_mass(&self) -> Option<Qty<Mass>> {
        (self.loose_mass > 0.0).then(|| Qty::from_raw(self.loose_mass))
    }
}

/// フレーム（role = structure）をすべて読む。質量は員数を掛けた素の質量（無ければ不足入力）。
pub(super) fn read(ctx: &Context<'_>, m: &mut MissingInputs) -> Frames {
    let mut out = Frames {
        frames: Vec::new(),
        loose_mass: 0.0,
    };
    for p in crate::parts::by_role(ctx, Role::Structure) {
        let Some(kg) = mass_or_missing(ctx, p, m) else {
            continue;
        };
        let mass = extensive(p, kg);
        let (width, wall) = (ctx.scalar(p, attr::WIDTH), ctx.scalar(p, attr::WALL));
        let Some(width) = width else {
            if wall.is_some() {
                m.push(
                    Target::attr(&p.id, attr::WIDTH),
                    "Member width (needed with the member wall)",
                );
            }
            out.loose_mass += mass;
            continue;
        };
        if let Some(frame) = read_one(ctx, p, (width, wall), mass, m) {
            out.frames.push(frame);
        }
    }
    out
}

/// 部材をモデルにするフレーム 1 つ（箱・材料・断面）。読めないものは不足入力に記録する。
fn read_one(
    ctx: &Context<'_>,
    p: &fune_model::resolve::ResolvedPart,
    (width, wall): (f64, Option<f64>),
    mass: f64,
    m: &mut MissingInputs,
) -> Option<ModalFemFrame> {
    let material = panel::material(ctx, p, m);
    let size = m.need(
        ctx.vec3(p, "size"),
        Target::attr(&p.id, "size"),
        "Frame outline (the box of the members)",
    );
    let (e, nu, _) = match &material {
        Some((name, mat)) => panel::properties(name, mat, m),
        None => (None, None, None),
    };
    let extents = size.and_then(|s| axis_aligned_extents(ctx.resolved, &p.id, s));
    if size.is_some() && extents.is_none() {
        m.push(
            Target::attr(&p.id, attr::WIDTH),
            "The frame box must be aligned with the body axes (a multiple of 90°) to model its members",
        );
    }
    let at = placed(ctx, p);
    Some(ModalFemFrame {
        id: p.id.clone(),
        center: [at.position.x, at.position.y, at.position.z].map(Qty::from_raw),
        size: extents?.map(Qty::<Length>::from_raw),
        youngs_modulus: e?,
        poisson_ratio: nu?,
        section: ModalFemMemberSection {
            width: Qty::from_raw(width),
            wall: wall.map(Qty::from_raw),
        },
        mass: Qty::from_raw(mass),
    })
}
