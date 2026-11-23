//! フェーズ（`phases/<ID>.toml`）の検査と、ケースの `phase` の参照。
//!
//! フェーズの `modes` と `[law.<モード>]` のキーはモードがあること、期間は時間の次元、`[orbit]` は設計の軌道と同じ検査
//! （`settings::check_orbit` と設定の次元の表）。ケースの `phase` はフェーズがあること。ケースのモードがフェーズの
//! `modes`（空でなければ）に無ければ警告する。

use super::Ctx;
use super::settings::{check_orbit, phase_orbit_scope};
use super::system::check_dim;
use crate::design::settings_view::entries_under;
use crate::design::{Case, EntityKey, Phase};
use crate::path::SettingPath;
use crate::target::Target;

pub(super) fn check_phases(ctx: &mut Ctx) {
    let design = ctx.design;
    for phase in design.phases.values() {
        let entity = EntityKey::Phase(phase.id.to_string());
        let id = phase.id.as_str();
        check_dim(
            ctx,
            &entity,
            "duration",
            Target::phase(id, Some("duration")),
            phase.duration.as_ref(),
            "Time",
        );
        check_mode_refs(ctx, phase, &entity);
        if let Some(o) = &phase.orbit {
            let entries = entries_under(
                &SettingPath::conditions(&["orbit"]),
                &serde_json::to_value(o).unwrap_or_default(),
            );
            let mut s = phase_orbit_scope(ctx, id);
            s.dimensions(&entries);
            check_orbit(&mut s, o);
        }
    }
    for case in design.cases.values() {
        check_case_phase(ctx, case);
    }
}

/// `modes` の各モードと `[law.<モード>]` のキーはモードがあること。
fn check_mode_refs(ctx: &mut Ctx, phase: &Phase, entity: &EntityKey) {
    let design = ctx.design;
    let id = phase.id.as_str();
    for mode in &phase.modes {
        if !design.modes.contains_key(mode.as_str()) {
            ctx.error(
                "ref.mode",
                entity,
                &["modes"],
                Some(Target::phase(id, Some("modes"))),
                format!("Mode `{mode}` is not defined"),
            );
        }
    }
    for mode in phase.law.keys() {
        if !design.modes.contains_key(mode.as_str()) {
            let field = format!("law.{mode}");
            ctx.error(
                "ref.mode",
                entity,
                &["law", mode],
                Some(Target::phase(id, Some(&field))),
                format!("Mode `{mode}` is not defined"),
            );
        }
    }
}

/// ケースの `phase` はフェーズがあること。ケースのモードがフェーズの `modes`（空でなければ）に無ければ警告。
fn check_case_phase(ctx: &mut Ctx, case: &Case) {
    let Some(phase_id) = &case.phase else {
        return;
    };
    let entity = EntityKey::Case(case.id.to_string());
    let target = Some(Target::case(case.id.as_str(), Some("phase")));
    let Some(phase) = ctx.design.phases.get(phase_id.as_str()) else {
        ctx.error(
            "ref.phase",
            &entity,
            &["phase"],
            target,
            format!("Phase `{phase_id}` is not defined"),
        );
        return;
    };
    if let Some(mode) = &case.mode
        && !phase.modes.is_empty()
        && !phase.modes.contains(mode)
    {
        ctx.warning(
            "case.mode_not_in_phase",
            &entity,
            &["phase"],
            target,
            format!("Mode `{mode}` is not listed in the modes of phase `{phase_id}`"),
        );
    }
}
