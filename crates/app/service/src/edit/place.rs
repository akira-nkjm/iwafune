//! 機体座標での配置 → 取付点に対する相対位置・姿勢の `Place`（GUI の 3D 配置と `fune part place --body-pos`）。
//! 逆変換は `fune_projection::pose::body_to_mount_local`、単位の換算は `fune_model::value::units`（sat-units）。

use crate::dto::{ApplyRequest, ApplyResult, PlaceBodyRequest};
use crate::error::{Result, ServiceError};
use crate::workspace::Workspace;
use fune_model::value::units::{to_si, unit_info};
use fune_model::value::{Quaternion, Vector3};
use fune_ops::Op;
use sat_astro::math::{Quat, Vec3};

/// 機体座標の位置・姿勢から `Place` の Op を作る（取付先・取付点はそのまま。位置はファイルの今の単位で書く）。
pub fn place_body_op(
    ws: &Workspace,
    id: &str,
    position: &Vector3,
    orientation: Option<&Quaternion>,
) -> Result<Op> {
    let s = ws.snapshot()?;
    let rp = s
        .resolved
        .parts
        .get(id)
        .ok_or_else(|| ServiceError::not_found("部品", id))?;
    let unit = position.unit.as_deref().unwrap_or("m");
    let si = |v: f64| to_si(v, unit).map_err(|e| ServiceError::InvalidInput(e.to_string()));
    let p_body = Vec3::new(si(position.x)?, si(position.y)?, si(position.z)?);
    let (_, q_now) =
        fune_projection::pose::body_pose(&s.resolved, id).unwrap_or((Vec3::ZERO, Quat::IDENTITY));
    let q_body = orientation.map_or(q_now, |q| Quat {
        w: q.w,
        x: q.x,
        y: q.y,
        z: q.z,
    });
    let (p_local, q_local) =
        fune_projection::pose::body_to_mount_local(&s.resolved, id, p_body, q_body)
            .ok_or_else(|| ServiceError::not_found("部品", id))?;
    let authored = s.design.parts.get(id).and_then(|p| p.pose.as_ref());
    let file_unit = authored
        .and_then(|p| p.position.as_ref())
        .and_then(|v| v.unit.clone())
        .unwrap_or_else(|| unit.to_owned());
    let out = unit_info(&file_unit)
        .map_err(|e| ServiceError::InvalidInput(e.to_string()))?
        .unit;
    let local = |v: f64| round6(out.from_si(v));
    let orientation = match orientation {
        Some(_) => Some(Quaternion {
            w: round6(q_local.w),
            x: round6(q_local.x),
            y: round6(q_local.y),
            z: round6(q_local.z),
        }),
        None => authored.and_then(|p| p.orientation.clone()),
    };
    Ok(Op::Place {
        part: id.to_owned(),
        on: rp.mount.as_ref().map(|m| m.on.clone()),
        point: rp.mount.as_ref().and_then(|m| m.point.clone()),
        position: Some(Vector3 {
            x: local(p_local.x),
            y: local(p_local.y),
            z: local(p_local.z),
            unit: Some(file_unit),
            frame: None,
        }),
        orientation,
    })
}

/// 機体座標での配置を適用する（`POST /parts/{id}/place_body`）。
pub fn place_body(ws: &Workspace, id: &str, req: &PlaceBodyRequest) -> Result<ApplyResult> {
    let op = place_body_op(ws, id, &req.position, req.orientation.as_ref())?;
    super::apply(
        ws,
        &ApplyRequest {
            ops: vec![op],
            base_revision: req.base_revision.clone(),
            dry_run: req.dry_run,
        },
    )
}

/// ファイルに書く数値を 6 桁の小数に丸める（逆変換の丸め誤差でファイルが揺れないように）。
fn round6(v: f64) -> f64 {
    let r = (v * 1e6).round() / 1e6;
    if r == 0.0 { 0.0 } else { r }
}
