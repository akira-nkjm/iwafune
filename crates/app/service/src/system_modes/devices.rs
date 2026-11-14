//! 機器を役割別に集約。稼働台・予備・duty は解析と共通の counted_states を使う。
use crate::dto::{AocsDeviceGroup, AocsDeviceState, AocsDeviceView};
use crate::error::Result;
use fune_model::builtin::{self, Label};
use fune_model::design::AnalysisSystem;
use fune_model::resolve::ResolvedPart;
use fune_model::role::{Role, STATE_OFF};
use fune_projection::{Context, parts};
use sat_analysis::modes::{ModeDef, Population, counted_states};
use sat_units::typed::{Fraction, NamedKind};

/// 同じ役割の並びの先頭に置く役割（この順。ほかの役割はこのあとに表示名の順、役割の無いものは最後）。
const ROLE_ORDER: &[Role] = &[
    Role::StarTracker,
    Role::SunSensor,
    Role::Gps,
    Role::Magnetometer,
    Role::Gyro,
    Role::ReactionWheel,
    Role::Magnetorquer,
    Role::Thruster,
];

/// AOCS の系に属する部品を役割ごとにまとめる（部品の無い役割は出さない）。
/// 系の決め方は `ResolvedPart.system`（置き場所 → 役割の系）。種別に役割の無い部品は「その他」に入れる。
pub(super) fn groups(ctx: &Context<'_>, mode: &ModeDef) -> Result<Vec<AocsDeviceGroup>> {
    let parts: Vec<&ResolvedPart> = parts::in_system(ctx, AnalysisSystem::Aocs).collect();
    let mut roles: Vec<Option<Role>> = parts.iter().map(|p| p.role).collect();
    roles.sort_by_cached_key(|r| order_key(*r));
    roles.dedup();
    roles
        .into_iter()
        .map(|role| {
            let members = parts.iter().filter(|p| p.role == role);
            group(role, members, mode)
        })
        .collect()
}

/// 並べる順: `ROLE_ORDER` の役割 → ほかの役割（表示名の順）→ 役割なし。
fn order_key(role: Option<Role>) -> (usize, String) {
    match role {
        Some(r) => match ROLE_ORDER.iter().position(|o| *o == r) {
            Some(i) => (i, String::new()),
            None => (ROLE_ORDER.len(), label_of(r).label),
        },
        None => (ROLE_ORDER.len() + 1, String::new()),
    }
}

/// 役割の表示名（組み込みの語彙。無ければ役割の ID）。
fn label_of(role: Role) -> Label {
    builtin::role_label(role).unwrap_or_else(|| Label {
        label: role.to_string(),
        label_en: None,
    })
}

fn group<'a>(
    role: Option<Role>,
    members: impl Iterator<Item = &'a &'a ResolvedPart>,
    mode: &ModeDef,
) -> Result<AocsDeviceGroup> {
    let label = match role {
        Some(r) => label_of(r),
        None => builtin::role_other_label(),
    };
    let mut group = AocsDeviceGroup {
        role,
        label: label.label,
        label_en: label.label_en,
        active_count: 0,
        total_count: 0,
        devices: Vec::new(),
    };
    for p in members {
        let count = parts::count_of(p);
        group.total_count += count;
        let states = part_states(p, count, mode)?;
        let active_count = states.as_ref().map(|states| active_count(states));
        group.active_count += active_count.unwrap_or(0);
        group.devices.push(AocsDeviceView {
            part: p.id.clone(),
            active: active_count.map(|n| n > 0),
            states: states
                .unwrap_or_default()
                .into_iter()
                .map(|(state, fraction, count)| AocsDeviceState {
                    state,
                    count,
                    fraction: crate::units::quantity_auto(fraction, Fraction::name()),
                })
                .collect(),
        });
    }
    Ok(group)
}

/// モードで割り当てられた状態（台数・時間割合）。モードに割り当てが無い部品は None。
fn part_states(
    p: &ResolvedPart,
    count: u32,
    mode: &ModeDef,
) -> Result<Option<Vec<(String, f64, u32)>>> {
    mode.assignments
        .iter()
        .find(|a| a.part == p.id)
        .map(|a| {
            counted_states(
                a,
                Population {
                    count,
                    redundancy: parts::redundancy(p),
                    states: &p.states,
                },
            )
        })
        .transpose()
        .map_err(super::mode_analysis_error)
}

fn active_count(states: &[(String, f64, u32)]) -> u32 {
    // duty 状態で同じ稼働台を重複して数えず、常時その状態にいる予備台も含める。
    let active = states
        .iter()
        .filter(|(s, f, _)| s != STATE_OFF && *f > 0.0 && *f < 1.0)
        .map(|(_, _, n)| *n)
        .max()
        .unwrap_or(0);
    let constant: u32 = states
        .iter()
        .filter(|(s, f, _)| s != STATE_OFF && *f == 1.0)
        .map(|(_, _, n)| *n)
        .sum();
    active + constant
}
