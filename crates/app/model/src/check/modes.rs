//! モードの参照・状態・台数・継続時間の検査。

use super::{Ctx, system::check_dim};
use crate::design::{EntityKey, Mode, ModeWhen, StateAssign};
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
                format!("Mode `{id}` in the display order is not defined"),
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
        if let Some(q) = mode.start.as_ref().and_then(|s| s.offset.as_ref()) {
            check_dim(
                ctx,
                &entity,
                "start",
                Target::mode(mode.id.as_str(), None),
                Some(q),
                "Time",
            );
        }
        let timed = mode.start.is_some() || mode.when.is_some_and(|w| w != ModeWhen::Any);
        if timed && mode.max_duration.is_none() {
            ctx.warning(
                "mode.timing_unused",
                &entity,
                &["max_duration"],
                Some(Target::mode(mode.id.as_str(), None)),
                "when / start only affect the longest-duration evaluation, but the mode has no max_duration",
            );
        }
        for (part_id, assign) in &mode.eclipse {
            check_mode_row(ctx, mode, &entity, part_id.as_str(), assign, "eclipse");
            if !mode.states.contains_key(part_id) {
                ctx.warning(
                    "mode.eclipse_unassigned",
                    &entity,
                    &["eclipse", part_id.as_str()],
                    Some(Target::mode(mode.id.as_str(), Some(part_id.as_str()))),
                    "The eclipse override target is not in states",
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
                        "Mode `{}` has no state for part `{}` (this is not treated as 0 W)",
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
        let message = format!("Part `{part_id}` of mode `{}` is not defined", mode.id);
        ctx.error("ref.part", entity, &keys, path, message);
        return;
    };
    let states = design.part_states(part);
    check_population(ctx, entity, &keys, path.clone(), part, assign, states);
    for state in std::iter::once(&assign.state).chain(assign.duty.keys()) {
        if !states.iter().any(|s| s == state) {
            let message = format!(
                "State `{state}` of `{part_id}` is not in the kind ({})",
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
        let message = format!(
            "The time shares of `{part_id}` must be from 0 to 1 and add up to at most 1 (total {total})"
        );
        ctx.error("mode.duty", entity, &keys, path, message);
    }
}

/// warm の予備機の既定の状態（語彙にある最初のもの）。sat-analysis の `modes::WARM_SPARE_DEFAULT_STATES` と同じ値
/// （sat-analysis は fune-model を見られないので、一致は fune-projection のテストで確かめる）。
pub const WARM_SPARE_DEFAULT_STATES: [&str; 2] = ["standby", "idle"];

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
        error("mode.active", "active must be at most count");
    }
    if a.active.is_some() && part.redundancy == Redundancy::None {
        error(
            "mode.redundancy",
            "active requires redundancy of cold or warm",
        );
    }
    if a.active.unwrap_or(part.count) < part.count {
        let spare = match part.redundancy {
            Redundancy::None => None,
            Redundancy::Cold => Some(crate::builtin::STATE_OFF),
            Redundancy::Warm => a.spare_state.as_deref().or_else(|| {
                WARM_SPARE_DEFAULT_STATES
                    .into_iter()
                    .find(|s| states.iter().any(|v| v == s))
            }),
        };
        match spare {
            Some(s) if states.iter().any(|v| v == s) => {}
            _ => error(
                "mode.spare_state",
                "The spare state cannot be determined from the kind's vocabulary",
            ),
        }
    }
    if a.spare_state.as_ref().is_some_and(|s| !states.contains(s)) {
        error(
            "mode.spare_state",
            "spare_state must be chosen from the kind's vocabulary",
        );
    }
}
