//! 機器を役割別に集約。稼働台・予備・duty は解析と共通の counted_states を使う。
use crate::dto::{AocsDeviceGroup, AocsDeviceState, AocsDeviceView};
use crate::error::Result;
use fune_model::role::{Role, STATE_OFF};
use fune_projection::{Context, parts};
use sat_analysis::modes::{ModeDef, Population, counted_states};
use sat_units::typed::{Fraction, NamedKind};

const ROLES: &[Role] = &[
    Role::StarTracker,
    Role::SunSensor,
    Role::Gps,
    Role::Magnetometer,
    Role::Gyro,
    Role::ReactionWheel,
    Role::Magnetorquer,
    Role::Thruster,
    Role::Camera,
];

pub(super) fn groups(ctx: &Context<'_>, mode: &ModeDef) -> Result<Vec<AocsDeviceGroup>> {
    let kinds = fune_model::builtin::kinds();
    ROLES
        .iter()
        .map(|&role| {
            let label = kinds.values().find(|k| k.role == Some(role));
            let mut group = AocsDeviceGroup {
                role,
                label: label.map_or_else(|| role.to_string(), |l| l.label.clone()),
                label_en: label.and_then(|l| l.label_en.clone()),
                active_count: 0,
                total_count: 0,
                devices: Vec::new(),
            };
            for p in parts::by_role(ctx, role) {
                let count = parts::count_of(p);
                group.total_count += count;
                let states = mode
                    .assignments
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
                    .map_err(super::mode_analysis_error)?;
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
        })
        .collect()
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
