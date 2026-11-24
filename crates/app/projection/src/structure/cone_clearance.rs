//! `structure.cone_clearance` の射影。円錐を持つもの（担い手）の表 [`CARRIERS`] と、全部品の箱（[`super::boxes`]）、距離の上限の設定。
//!
//! **どの部品が、どの属性から円錐（軸・半頂角）を持つかの対応はこの表だけ**（担い手を増やすのは表の 1 行。判定・規則・出力は解析が共通に持つ）。
//! 軸の向きは `ctx.vec3` を機体座標に直したもの（[`crate::parts::direction_body`]）、置き場所は `ctx.resolved` から
//! （形態を選べば、その配置で判定する）。軸や半頂角が読めない担い手は、不足入力にせず、そのまま解析へ渡して理由つきで一覧に残す。

use super::body_boxes::{boxes, placed};
use crate::context::Context;
use crate::missing::MissingInputs;
use crate::parts::{SENSOR_ROLES, direction_body};
use crate::quantity::setting_opt;
use fune_model::builtin::Role;
use fune_model::design::setting_dims as dims;
use sat_analysis::structure::cone_clearance::{ConeCarrier, ConeClearanceInput, ConeKind};
use sat_analysis::structure::interference::InterferenceBox;
use sat_units::typed::Qty;

/// 担い手の表の 1 行: 役割の部品が、どの属性から円錐を持つか。
pub struct CarrierRow {
    /// 円錐を持つ部品の役割
    pub roles: &'static [Role],
    pub kind: ConeKind,
    /// 軸の向きの属性（部品座標の単位ベクトル）
    pub axis_attr: &'static str,
    /// 軸の向きに掛ける符号（円錐が開く向きは `sign × 属性の向き`）
    pub axis_sign: f64,
    /// 半頂角の属性（スカラーの角度）
    pub half_angle_attr: &'static str,
}

/// スラスタの役割（推進系のスラスタを含む）。
const THRUSTER_ROLES: &[Role] = &[Role::Thruster, Role::PropulsionThruster];

/// 担い手の表。**ここに 1 行足せば、解析の判定に載る**（アンテナのビームなら、役割 `Antenna`・軸 `boresight`・半頂角の属性を 1 行）。
pub const CARRIERS: &[CarrierRow] = &[
    // 噴流: 排気は推力の向き（`direction`）の反対へ出る。半頂角は任意の属性 `plume_half_angle`
    CarrierRow {
        roles: THRUSTER_ROLES,
        kind: ConeKind::Plume,
        axis_attr: "direction",
        axis_sign: -1.0,
        half_angle_attr: "plume_half_angle",
    },
    // センサの視野: 視線（`boresight`）のまわりに `fov_half_angle`。`aocs.sensor_fov` と同じセンサの役割
    CarrierRow {
        roles: SENSOR_ROLES,
        kind: ConeKind::FieldOfView,
        axis_attr: "boresight",
        axis_sign: 1.0,
        half_angle_attr: "fov_half_angle",
    },
];

/// `structure.cone_clearance`: 全部品の箱と、表の担い手、距離の上限（`analysis/structure.toml` の `cone_clearance_range`。無ければ解析の既定）。
pub fn cone_clearance(ctx: &Context<'_>) -> Result<ConeClearanceInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let body = boxes(ctx);
    let range = ctx
        .design
        .analysis
        .structure
        .as_ref()
        .and_then(|s| s.cone_clearance_range.as_ref());
    let range = setting_opt(&dims::STRUCTURE_CONE_CLEARANCE_RANGE, &[], range, &mut m);
    let carriers = carriers(ctx);
    m.finish(|| {
        Some(ConeClearanceInput {
            boxes: body
                .boxes
                .into_iter()
                .map(|b| InterferenceBox {
                    id: b.id,
                    count: b.count,
                    position: b.position,
                    orientation: b.orientation,
                    size: b.size,
                    mount_ancestors: b.mount_ancestors,
                })
                .collect(),
            unsized_parts: body.unsized_parts,
            carriers,
            range,
        })
    })
}

/// 表の役割を持つ部品（ID の昇順。部品が複数の行に当てはまれば、行ごとに 1 つ）。
fn carriers(ctx: &Context<'_>) -> Vec<ConeCarrier> {
    let mut out = Vec::new();
    for p in ctx.resolved.parts.values() {
        let Some(role) = p.role else { continue };
        for row in CARRIERS.iter().filter(|r| r.roles.contains(&role)) {
            // 読めないものは不足入力にしない（解析が理由つきで一覧に残す）ので、記録は捨てる
            let mut unused = MissingInputs::new();
            let axis = direction_body(ctx, p, row.axis_attr, "Cone axis", &mut unused)
                .map(|v| v.scale(row.axis_sign));
            out.push(ConeCarrier {
                id: p.id.clone(),
                kind: row.kind,
                position: placed(ctx, p).position,
                axis,
                half_angle: ctx.scalar(p, row.half_angle_attr).map(Qty::from_raw),
                mount_ancestors: ctx
                    .resolved
                    .mount_ancestors(&p.id)
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            });
        }
    }
    out
}
