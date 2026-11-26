//! 部品の配置 → `Place`。打った取付・位置・姿勢（`fune part place`・GUI の配置の欄。[`place_op`]）と、
//! 機体座標での配置（GUI の 3D 配置と `fune part place --body-pos`。[`place_body_op`]）。
//! 姿勢の角度（`--rotation "0 90 0 deg"`）は [`rotation_from_text`] が読み、クォータニオンにして書く（角度 ⇄ クォータニオンは
//! `sat_astro::math::Quat::from_fixed_xyz`）。逆変換は `fune_projection::pose::body_to_mount_local`、単位の換算は
//! `fune_model::value::units`（sat-units）。

use super::text::{given, invalid, quaternion_from_text, vector_from_text};
use crate::dto::{ApplyRequest, ApplyResult, PlaceBodyRequest, PlaceText};
use crate::error::{Result, ServiceError};
use crate::query::pose;
use crate::workspace::Workspace;
use fune_model::value::units::{expect_dimension, to_si, unit_info};
use fune_model::value::{Quaternion, Vector3};
use fune_ops::Op;
use sat_astro::math::{Quat, Vec3};

/// `fune part place` → `Place`（取付と、取付点に対する位置・姿勢を丸ごと置き換える）。取付先が無ければ機体の根。
/// 姿勢はクォータニオン（`orientation`）か角度（`rotation`）のどちらか 1 つ。
pub fn place_op(t: &PlaceText) -> Result<Op> {
    let on = given(&t.on);
    let orientation = match (given(&t.orientation), given(&t.rotation)) {
        (Some(_), Some(_)) => {
            return Err(invalid(
                "Give the attitude with exactly one of --orientation (quaternion) or --rotation (angles)",
            ));
        }
        (Some(q), None) => Some(quaternion_from_text(q)?),
        (None, Some(r)) => Some(rotation_from_text(r)?),
        (None, None) => None,
    };
    let configuration = given(&t.configuration);
    // 形態の中の配置は取付を変えないので、取付先を書かなくても取付点（書くなら部品の取付と同じ）を渡せる
    let point = match configuration {
        Some(_) => given(&t.point),
        None => on.and(given(&t.point)),
    };
    Ok(Op::Place {
        part: t.part.trim().to_owned(),
        on: on.map(str::to_owned),
        point: point.map(str::to_owned),
        position: given(&t.position).map(vector_from_text).transpose()?,
        orientation,
        configuration: configuration.map(str::to_owned),
    })
}

/// 姿勢の角度の文字列（`"0 90 0 deg"`・`"0 1.5708 0 rad"`。親の座標軸 X・Y・Z まわりに X → Y → Z の順に回す）→ クォータニオン
/// （6 桁に丸めて書く。スカラー部は非負）。単位は角度で必ず書く。
pub fn rotation_from_text(text: &str) -> Result<Quaternion> {
    let v = vector_from_text(text)?;
    let unit = v
        .unit
        .as_deref()
        .ok_or_else(|| invalid(format!("Give the angle unit (deg or rad): {text}")))?;
    let unit = expect_dimension(unit, "Angle").map_err(invalid)?;
    let q = Quat::from_fixed_xyz([v.x, v.y, v.z].map(|a| unit.to_si(a)));
    Ok(Quaternion {
        w: round6(q.w),
        x: round6(q.x),
        y: round6(q.y),
        z: round6(q.z),
    })
}

/// 機体座標の位置・姿勢から `Place` の Op を作る（取付先・取付点はそのまま。位置はファイルの今の単位で書く）。
/// `configuration` があれば、機体座標はその形態で解決した設計で読み（親の姿勢が基準と違えば、取付点に対する姿勢も違う）、
/// 結果はその形態の `[pose.<部品>]` に書く `Place`（`configuration` つき）になる。逆変換は 1 つ
/// （`fune_projection::pose::body_to_mount_local`）で、形態ごとの合成は持たない。取付の根（取付先が無い）の取付点の座標系は
/// 機体座標なので、同じ逆変換（恒等の座標系）で、機体座標での姿勢がそのまま根の `[pose]` になる。
pub fn place_body_op(
    ws: &Workspace,
    id: &str,
    position: &Vector3,
    orientation: Option<&Quaternion>,
    configuration: Option<&str>,
) -> Result<Op> {
    let s = ws.snapshot()?;
    pose::require_configuration(&s.design, configuration)?;
    let resolved = pose::resolved_in(&s.resolved, configuration);
    let rp = resolved
        .parts
        .get(id)
        .ok_or_else(|| ServiceError::not_found("part", id))?;
    let unit = fune_model::design::position_unit(position);
    let si = |v: f64| to_si(v, unit).map_err(|e| ServiceError::InvalidInput(e.to_string()));
    let p_body = Vec3::new(si(position.x)?, si(position.y)?, si(position.z)?);
    let (_, q_now) =
        fune_projection::pose::body_pose(resolved, id).unwrap_or((Vec3::ZERO, Quat::IDENTITY));
    let q_body = orientation.map_or(q_now, |q| Quat {
        w: q.w,
        x: q.x,
        y: q.y,
        z: q.z,
    });
    let (p_local, q_local) =
        fune_projection::pose::body_to_mount_local(resolved, id, p_body, q_body)
            .ok_or_else(|| ServiceError::not_found("part", id))?;
    let authored = s
        .design
        .parts
        .get(id)
        .and_then(|p| pose::authored_pose(&s.design, p, configuration));
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
        configuration: configuration.map(str::to_owned),
    })
}

/// 機体座標での配置を適用する（`POST /parts/{id}/place_body`）。
pub fn place_body(ws: &Workspace, id: &str, req: &PlaceBodyRequest) -> Result<ApplyResult> {
    let op = place_body_op(
        ws,
        id,
        &req.position,
        req.orientation.as_ref(),
        req.configuration.as_deref(),
    )?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn text(orientation: Option<&str>, rotation: Option<&str>) -> PlaceText {
        PlaceText {
            part: "RW_X".into(),
            on: Some("PANEL_MZ".into()),
            point: Some("RW_SLOT_X".into()),
            position: Some("0 0 12 mm".into()),
            orientation: orientation.map(str::to_owned),
            rotation: rotation.map(str::to_owned),
            configuration: None,
        }
    }

    fn orientation_of(op: Op) -> Option<Quaternion> {
        match op {
            Op::Place { orientation, .. } => orientation,
            _ => panic!("{op:?}"),
        }
    }

    /// 配置: 位置・姿勢の文字列は `fune part place --position` と同じ読み方。取付先が空なら機体の根で、取付点も書かない。
    #[test]
    fn place_reads_vector_and_quaternion_text() {
        let t = text(Some("1 0 0 0"), None);
        let Op::Place {
            on,
            point,
            position,
            orientation,
            ..
        } = place_op(&t).unwrap()
        else {
            panic!()
        };
        assert_eq!(
            (on.as_deref(), point.as_deref()),
            (Some("PANEL_MZ"), Some("RW_SLOT_X"))
        );
        let p = position.unwrap();
        assert_eq!((p.z, p.unit.as_deref()), (12.0, Some("mm")));
        assert_eq!(orientation.unwrap().w, 1.0);
        let root = PlaceText {
            on: Some(" ".into()),
            position: Some(String::new()),
            ..t.clone()
        };
        let Op::Place {
            on,
            point,
            position,
            ..
        } = place_op(&root).unwrap()
        else {
            panic!()
        };
        assert_eq!((on, point, position), (None, None, None));
        let bad = PlaceText {
            position: Some("0 0 mm".into()),
            ..t
        };
        assert!(place_op(&bad).is_err());
    }

    /// 手計算: Y まわり 90° は (cos 45°, 0, sin 45°, 0) = (0.707107, 0, 0.707107, 0)（6 桁）。rad でも同じ。
    /// Z まわり 270° は −90° と同じ回転で、スカラー部が非負の (0.707107, 0, 0, −0.707107)。
    #[test]
    #[allow(clippy::approx_constant)] // ファイルに 6 桁で書く値（0.707107）を確かめる
    fn rotation_text_becomes_a_rounded_quaternion() {
        let half = Quaternion {
            w: 0.707107,
            x: 0.0,
            y: 0.707107,
            z: 0.0,
        };
        assert_eq!(rotation_from_text("0 90 0 deg").unwrap(), half);
        assert_eq!(
            rotation_from_text(" 0 1.5707963267948966 0 rad ").unwrap(),
            half
        );
        let z = rotation_from_text("0 0 270 deg").unwrap();
        assert_eq!((z.w, z.z), (0.707107, -0.707107));
        let none = rotation_from_text("0 0 0 deg").unwrap();
        assert_eq!((none.w, none.x, none.y, none.z), (1.0, 0.0, 0.0, 0.0));
        for bad in ["0 90 0", "0 90 deg", "0 90 0 mm", "0 90 0 parsec"] {
            assert!(rotation_from_text(bad).is_err(), "{bad}");
        }
    }

    /// 角度の欄は `Place` の姿勢になり、クォータニオンと両方は書けない（空の欄は書かないのと同じ）。
    #[test]
    #[allow(clippy::approx_constant)] // ファイルに 6 桁で書く値（0.707107）を確かめる
    fn place_takes_either_orientation_or_rotation() {
        let by_angle = orientation_of(place_op(&text(Some(" "), Some("0 90 0 deg"))).unwrap());
        assert_eq!(by_angle.map(|q| q.y), Some(0.707107));
        assert!(place_op(&text(Some("1 0 0 0"), Some("0 90 0 deg"))).is_err());
        assert_eq!(
            orientation_of(place_op(&text(None, Some(""))).unwrap()),
            None
        );
    }
}
