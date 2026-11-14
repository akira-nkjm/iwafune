//! マヌーバ（`maneuvers/<ID>.toml`）の検査: スラスタ・前のマヌーバ・制約のモードの参照、実行の順の循環、
//! 目標の軌道の量の次元と書き方（天体間の遷移の捕獲の軌道）、計算の段階に要るもの。

use super::Ctx;
use super::values::check_quantity_dimension;
use crate::builtin::Role;
use crate::design::{Design, EntityKey, Maneuver, ManeuverMethod, ManeuverTarget};
use crate::target::Target;

/// スラスタとして使える部品の役割。
const THRUSTER_ROLES: [Role; 2] = [Role::PropulsionThruster, Role::Thruster];

pub(super) fn check_maneuvers(ctx: &mut Ctx) {
    let design = ctx.design;
    for m in design.maneuvers.values() {
        let entity = EntityKey::Maneuver(m.id.to_string());
        check_thruster(ctx, design, m, &entity);
        check_after(ctx, design, m, &entity);
        check_target(ctx, m, &entity);
        check_duty(ctx, m, &entity);
        if let Some(mode) = &m.constraints.mode
            && !design.modes.contains_key(mode.as_str())
        {
            ctx.error(
                "ref.mode",
                &entity,
                &["constraints", "mode"],
                Some(Target::maneuver(m.id.as_str(), Some("constraints.mode"))),
                format!("モード `{mode}` がありません"),
            );
        }
    }
}

/// デューティは 0 より大きく 1 以下。
fn check_duty(ctx: &mut Ctx, m: &Maneuver, entity: &EntityKey) {
    if let Some(d) = m.constraints.duty
        && !(d > 0.0 && d <= 1.0)
    {
        ctx.error(
            "maneuver.duty",
            entity,
            &["constraints", "duty"],
            Some(Target::maneuver(m.id.as_str(), Some("constraints.duty"))),
            format!("デューティは 0 より大きく 1 以下にしてください（{d}）"),
        );
    }
}

fn check_thruster(ctx: &mut Ctx, design: &Design, m: &Maneuver, entity: &EntityKey) {
    let id = m.id.as_str();
    let target = || Some(Target::maneuver(id, Some("thruster")));
    let Some(t) = &m.thruster else {
        if m.method != ManeuverMethod::Impulsive {
            ctx.warning(
                "maneuver.thruster_missing",
                entity,
                &["method"],
                target(),
                format!(
                    "{} の計算にはスラスタ（thruster）が要ります（ΔV だけを求めます）",
                    m.method
                ),
            );
        }
        return;
    };
    let Some(part) = design.parts.get(t.as_str()) else {
        ctx.error(
            "ref.part",
            entity,
            &["thruster"],
            target(),
            format!("部品 `{t}` がありません"),
        );
        return;
    };
    let role = design.part_kind_id(part).and_then(|k| design.kind_role(k));
    if !role.is_some_and(|r| THRUSTER_ROLES.contains(&r)) {
        ctx.warning(
            "maneuver.thruster_role",
            entity,
            &["thruster"],
            target(),
            format!(
                "部品 `{t}` はスラスタ（role = propulsion_thruster か thruster）ではありません"
            ),
        );
    }
}

/// 前のマヌーバ: 無い ID は警告（フェーズは #9 で足す）、たどって自分に戻れば循環の誤り。
fn check_after(ctx: &mut Ctx, design: &Design, m: &Maneuver, entity: &EntityKey) {
    let Some(after) = m.after.as_deref() else {
        return;
    };
    let target = || Some(Target::maneuver(m.id.as_str(), Some("after")));
    if !design.maneuvers.contains_key(after) {
        ctx.warning(
            "maneuver.after_unknown",
            entity,
            &["after"],
            target(),
            format!("前のマヌーバ `{after}` がありません（フェーズはまだ扱わないので、最初のマヌーバとして扱います）"),
        );
        return;
    }
    let mut at = Some(after);
    for _ in 0..design.maneuvers.len() {
        let Some(id) = at else {
            return;
        };
        if id == m.id.as_str() {
            ctx.error(
                "maneuver.cycle",
                entity,
                &["after"],
                target(),
                format!("マヌーバ `{}` の after をたどると自分に戻ります", m.id),
            );
            return;
        }
        at = design.maneuvers.get(id).and_then(|p| p.after.as_deref());
    }
}

fn check_target(ctx: &mut Ctx, m: &Maneuver, entity: &EntityKey) {
    let id = m.id.as_str();
    for (key, q) in m.target.entries() {
        if let Some((sev, code, msg)) =
            q.and_then(|q| check_quantity_dimension(q, ManeuverTarget::dimension(key)))
        {
            let field = format!("target.{key}");
            let target = Some(Target::maneuver(id, Some(&field)));
            ctx.report(sev, code, entity, &["target", key], target, msg);
        }
    }
    let t = &m.target;
    if t.altitude.is_some() && (t.perigee_altitude.is_some() || t.apogee_altitude.is_some()) {
        ctx.error(
            "maneuver.target_conflict",
            entity,
            &["target", "altitude"],
            Some(Target::maneuver(id, Some("target.altitude"))),
            "altitude（円軌道）と perigee_altitude・apogee_altitude は一緒に書けません",
        );
    }
    if t.body.is_some() {
        let target = || Some(Target::maneuver(id, Some("target.body")));
        if t.altitude.is_none() && t.perigee_altitude.is_none() {
            ctx.error(
                "maneuver.body_orbit",
                entity,
                &["target", "body"],
                target(),
                "目標の天体を回る軌道（altitude か perigee_altitude）を書いてください",
            );
        }
        if m.method == ManeuverMethod::LowThrust {
            ctx.error(
                "maneuver.body_method",
                entity,
                &["method"],
                target(),
                "天体間の遷移は impulsive か finite_burn で計算します",
            );
        }
    }
    if let Some(q) = &m.via_altitude {
        let target = || Some(Target::maneuver(id, Some("via_altitude")));
        if let Some((sev, code, msg)) = check_quantity_dimension(q, "Length") {
            ctx.report(sev, code, entity, &["via_altitude"], target(), msg);
        }
        if matches!(
            m.method,
            ManeuverMethod::LowThrust | ManeuverMethod::Propagated
        ) {
            ctx.warning(
                "maneuver.via_ignored",
                entity,
                &["via_altitude"],
                target(),
                format!(
                    "{} では via_altitude（バイエリプティック遷移）を使いません",
                    m.method
                ),
            );
        }
    }
}
