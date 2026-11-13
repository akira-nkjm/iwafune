//! モードの参照・状態・台数・継続時間の検査。

use super::{Ctx, system::check_dim};
use crate::design::{EntityKey, Mode, StateAssign};
use crate::target::Target;

/// モード: 部品の参照・状態の語彙・時間割合・表に無い部品。
pub(super) fn check_modes(ctx: &mut Ctx) {
    let design = ctx.design;
    for id in design.project.modes.iter().flat_map(|m| &m.order) {
        if !design.modes.contains_key(id.as_str()) {
            ctx.warning(
                "mode.order_missing",
                &EntityKey::Project,
                &["modes", "order"],
                Some(Target::Modes),
                format!("表示順のモード `{id}` がありません"),
            );
        }
    }
    for mode in design.modes.values() {
        let entity = EntityKey::Mode(mode.id.to_string());
        check_dim(
            ctx,
            &entity,
            "max_duration",
            Target::mode(mode.id.as_str(), None),
            mode.max_duration.as_ref(),
            "Time",
        );
        for (part_id, assign) in &mode.eclipse {
            check_mode_row(ctx, mode, &entity, part_id.as_str(), assign, "eclipse");
            if !mode.states.contains_key(part_id) {
                ctx.warning(
                    "mode.eclipse_unassigned",
                    &entity,
                    &["eclipse", part_id.as_str()],
                    Some(Target::mode(mode.id.as_str(), Some(part_id.as_str()))),
                    "蝕の上書き先が states にありません",
                );
            }
        }
        for (part_id, assign) in &mode.states {
            check_mode_row(ctx, mode, &entity, part_id.as_str(), assign, "states");
        }
        for part in design.parts.values() {
            if !design.part_states(part).is_empty() && !mode.states.contains_key(part.id.as_str()) {
                ctx.warning(
                    "mode.unassigned",
                    &entity,
                    &["states"],
                    Some(Target::mode(mode.id.as_str(), Some(part.id.as_str()))),
                    format!(
                        "モード `{}` に部品 `{}` の状態がありません（0 W とはみなしません）",
                        mode.id, part.id
                    ),
                );
            }
        }
    }
}

/// モードの表の 1 行（部品の参照・状態の語彙・時間割合）。
fn check_mode_row(
    ctx: &mut Ctx,
    mode: &Mode,
    entity: &EntityKey,
    part_id: &str,
    assign: &StateAssign,
    section: &str,
) {
    let design = ctx.design;
    let keys = [section, part_id];
    let path = Some(Target::mode(mode.id.as_str(), Some(part_id)));
    let Some(part) = design.parts.get(part_id) else {
        let message = format!("モード `{}` の部品 `{part_id}` がありません", mode.id);
        ctx.error("ref.part", entity, &keys, path, message);
        return;
    };
    let states = design.part_states(part);
    check_population(ctx, entity, &keys, path.clone(), part, assign, states);
    for state in std::iter::once(&assign.state).chain(assign.duty.keys()) {
        if !states.iter().any(|s| s == state) {
            let message = format!(
                "`{part_id}` の状態 `{state}` は種別にありません（{}）",
                states.join(" / ")
            );
            ctx.error("state.unknown", entity, &keys, path.clone(), message);
        }
    }
    let total: f64 = assign.duty.values().sum();
    if assign.duty.contains_key(&assign.state)
        || assign.duty.values().any(|d| !(0.0..=1.0).contains(d))
        || total > 1.0 + super::SHARE_TOLERANCE
    {
        let message =
            format!("`{part_id}` の時間割合は 0〜1 で、合計 1 以下にします（合計 {total}）");
        ctx.error("mode.duty", entity, &keys, path, message);
    }
}

fn check_population(
    ctx: &mut Ctx,
    entity: &EntityKey,
    keys: &[&str],
    path: Option<Target>,
    part: &crate::design::Part,
    a: &StateAssign,
    states: &[String],
) {
    use crate::design::Redundancy;
    let mut error = |code, message| ctx.error(code, entity, keys, path.clone(), message);
    if a.active.is_some_and(|n| n > part.count) {
        error("mode.active", "active は count 以下にします");
    }
    if a.active.is_some() && part.redundancy == Redundancy::None {
        error(
            "mode.redundancy",
            "active には cold または warm の redundancy が必要です",
        );
    }
    if a.active.unwrap_or(part.count) < part.count {
        let spare = match part.redundancy {
            Redundancy::None => None,
            Redundancy::Cold => Some(crate::builtin::STATE_OFF),
            Redundancy::Warm => a.spare_state.as_deref().or_else(|| {
                ["standby", "idle"]
                    .into_iter()
                    .find(|s| states.iter().any(|v| v == s))
            }),
        };
        match spare {
            Some(s) if states.iter().any(|v| v == s) => {}
            _ => error("mode.spare_state", "予備の状態が種別の語彙から決まりません"),
        }
    }
    if a.spare_state.as_ref().is_some_and(|s| !states.contains(s)) {
        error("mode.spare_state", "spare_state は種別の語彙から選びます");
    }
}
