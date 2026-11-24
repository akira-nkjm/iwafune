//! マヌーバ（`maneuvers/<ID>.toml`）の検査: スラスタ・前のマヌーバ・制約のモードの参照、実行の順の循環、
//! 直接書いた ΔV（`delta_v`）の次元・符号・ほかの書き方との併用、目標の軌道の量の次元と書き方（天体間の遷移の捕獲の軌道）、計算の段階に要るもの、
//! 日付つきの遷移（`[transfer]`）の形（日付の読み方・暦の範囲・天体の組は暦を持つ `fune_projection::maneuver_dates` が見る）。

use super::Ctx;
use super::values::check_quantity_dimension;
use crate::design::{
    Design, EntityKey, Maneuver, ManeuverMethod, ManeuverTarget, ManeuverTransfer,
};
use crate::target::Target;

pub(super) fn check_maneuvers(ctx: &mut Ctx) {
    let design = ctx.design;
    for m in design.maneuvers.values() {
        let entity = EntityKey::Maneuver(m.id.to_string());
        check_thruster(ctx, design, m, &entity);
        check_after(ctx, design, m, &entity);
        check_target(ctx, m, &entity);
        check_delta_v(ctx, m, &entity);
        check_duty(ctx, m, &entity);
        check_transfer(ctx, m, &entity);
        if let Some(mode) = &m.constraints.mode
            && !design.modes.contains_key(mode.as_str())
        {
            ctx.error(
                "ref.mode",
                &entity,
                &["constraints", "mode"],
                Some(Target::maneuver(m.id.as_str(), Some("constraints.mode"))),
                format!("Mode `{mode}` is not defined"),
            );
        }
    }
}

/// 直接書いた ΔV（`delta_v`）: 速度の次元で負でなく、軌道を変える書き方（`[target]`・`via_altitude`・`[transfer]`）と
/// 一緒に書かず、計算の段階は impulsive か low_thrust（有限燃焼の損失・数値伝搬は軌道の計算が要る）。
fn check_delta_v(ctx: &mut Ctx, m: &Maneuver, entity: &EntityKey) {
    let Some(q) = &m.delta_v else {
        return;
    };
    let id = m.id.as_str();
    let target = |field: &str| Some(Target::maneuver(id, Some(field)));
    if let Some((sev, code, msg)) = check_quantity_dimension(q, "Velocity") {
        ctx.report(sev, code, entity, &["delta_v"], target("delta_v"), msg);
    } else if !(q.value >= 0.0 && q.value.is_finite()) {
        ctx.error(
            "maneuver.delta_v_negative",
            entity,
            &["delta_v"],
            target("delta_v"),
            format!("delta_v must be a finite value of 0 or more ({})", q.value),
        );
    }
    if !m.target.is_empty() || m.via_altitude.is_some() || m.transfer.is_some() {
        ctx.error(
            "maneuver.delta_v_conflict",
            entity,
            &["delta_v"],
            target("delta_v"),
            "delta_v (a written allowance) cannot be combined with [target], via_altitude or [transfer], which compute ΔV from an orbit change",
        );
    }
    if matches!(
        m.method,
        ManeuverMethod::FiniteBurn | ManeuverMethod::Propagated
    ) {
        ctx.error(
            "maneuver.delta_v_method",
            entity,
            &["method"],
            target("method"),
            format!(
                "A written delta_v is used with impulsive or low_thrust, not {}",
                m.method
            ),
        );
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
            format!("Duty must be greater than 0 and at most 1 ({d})"),
        );
    }
}

/// 日付つきの遷移（`[transfer]`）: 出発・到着の両方の日付が要り、目標の天体（`target.body`）が要る。
/// 天体間の遷移で使えない計算の段階（低推力は `maneuver.body_method`、数値伝搬はここ）は誤り。
fn check_transfer(ctx: &mut Ctx, m: &Maneuver, entity: &EntityKey) {
    let Some(tr) = &m.transfer else {
        return;
    };
    let id = m.id.as_str();
    for (key, date) in tr.entries() {
        if date.is_none_or(|d| d.trim().is_empty()) {
            let field = format!("transfer.{key}");
            ctx.error(
                "maneuver.transfer_dates",
                entity,
                &["transfer"],
                Some(Target::maneuver(id, Some(&field))),
                format!("[transfer] needs both departure and arrival (`{key}` is missing)"),
            );
        }
    }
    if m.target.body.is_none() {
        ctx.error(
            "maneuver.transfer_body",
            entity,
            &["transfer"],
            Some(Target::maneuver(id, Some("transfer"))),
            "[transfer] needs target.body (the body to arrive at)",
        );
    }
    if tr.departs_by_launcher()
        && m.after
            .as_deref()
            .is_some_and(|a| ctx.design.maneuvers.contains_key(a))
    {
        ctx.warning(
            "maneuver.departure_by_launcher_after",
            entity,
            &["transfer"],
            Some(Target::maneuver(id, Some("transfer.departure_by"))),
            "The launcher injects the spacecraft onto the departure asymptote, so a maneuver before this one (`after`) does not fit; the maneuvers before it are still counted",
        );
    }
    check_flybys(ctx, m, tr, entity);
    if m.method == ManeuverMethod::Propagated {
        ctx.error(
            "maneuver.transfer_method",
            entity,
            &["method"],
            Some(Target::maneuver(id, Some("method"))),
            "Dated transfers between bodies are computed with impulsive or finite_burn",
        );
    }
}

/// 途中のスイングバイ（`[[transfer.flybys]]`）の形: 日付が空でない・近点の高度の下限は長さの正の有限の値。
/// 日付の読み方・順序・暦・天体の組は暦を持つ `fune_projection::maneuver_dates` が見る。
fn check_flybys(ctx: &mut Ctx, m: &Maneuver, tr: &ManeuverTransfer, entity: &EntityKey) {
    let id = m.id.as_str();
    let target = Some(Target::maneuver(id, Some("transfer.flybys")));
    for (i, f) in tr.flybys.iter().enumerate() {
        if f.date.trim().is_empty() {
            ctx.error(
                "maneuver.flyby_date",
                entity,
                &["transfer", "flybys"],
                target.clone(),
                format!("Flyby {} of [transfer] needs a date", i + 1),
            );
        }
        let Some(q) = &f.min_altitude else {
            continue;
        };
        if let Some((sev, code, msg)) = check_quantity_dimension(q, "Length") {
            ctx.report(
                sev,
                code,
                entity,
                &["transfer", "flybys"],
                target.clone(),
                msg,
            );
        } else if !(q.value > 0.0 && q.value.is_finite()) {
            ctx.error(
                "maneuver.flyby_altitude",
                entity,
                &["transfer", "flybys"],
                target.clone(),
                format!(
                    "The minimum altitude of flyby {} must be a finite value above 0 ({}); a periapsis at or below the surface is not accepted",
                    i + 1,
                    q.value
                ),
            );
        }
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
                    "The {} calculation needs a thruster. Only ΔV is computed",
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
            format!("Part `{t}` is not defined"),
        );
        return;
    };
    if !design.is_maneuver_thruster(part) {
        ctx.warning(
            "maneuver.thruster_role",
            entity,
            &["thruster"],
            target(),
            format!("Part `{t}` is not a thruster (role = propulsion_thruster or thruster)"),
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
            format!("Previous maneuver `{after}` is not defined. Phases are not handled yet, so this is treated as the first maneuver"),
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
                format!(
                    "Following `after` from maneuver `{}` leads back to itself",
                    m.id
                ),
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
            "altitude (circular orbit) cannot be combined with perigee_altitude or apogee_altitude",
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
                "Specify an orbit around the target body (altitude or perigee_altitude)",
            );
        }
        if m.method == ManeuverMethod::LowThrust {
            ctx.error(
                "maneuver.body_method",
                entity,
                &["method"],
                target(),
                "Transfers between bodies are computed with impulsive or finite_burn",
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
                    "{} does not use via_altitude (bi-elliptic transfer)",
                    m.method
                ),
            );
        }
    }
}
