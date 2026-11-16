//! スラスタの射影: RCS（アンロード時間・1 日の推薬）と、モードで噴射するスラスタの推力誤差の擾乱。

use crate::context::{Context, mode_def};
use crate::missing::MissingInputs;
use crate::parts::{self, by_role, direction_body};
use fune_model::Target;
use fune_model::builtin::{Role, STATE_THRUSTER_FIRING};
use sat_analysis::aocs::disturbance::{FiringThruster, ThrustDisturbance};
use sat_analysis::aocs::rw_unloading::RwUnloadingRcsInput;
use sat_analysis::aocs::thrusters::Thruster;
use sat_analysis::structure::mass_properties::MassPropertiesSummary;
use sat_units::typed::{Force, Fraction, Time};

/// スラスタ（role = thruster）。位置は取付をたどった機体座標の部品原点、推力の向きは部品座標の `direction` を機体座標に回したもの。
pub(super) fn thrusters(ctx: &Context<'_>, m: &mut MissingInputs) -> Vec<Thruster> {
    by_role(ctx, Role::Thruster)
        .filter_map(|p| {
            let why = "Thruster values";
            let thrust = parts::part_qty::<Force>(ctx, p, "thrust", why, m);
            let isp = parts::part_qty::<Time>(ctx, p, "isp", why, m);
            let direction = direction_body(ctx, p, "direction", "Thrust direction (part frame)", m);
            let (position, _) = crate::pose::body_pose(ctx.resolved, &p.id)?;
            Some(Thruster {
                id: p.id.clone(),
                thrust: thrust?,
                isp: isp?,
                position,
                direction: direction?,
            })
        })
        .collect()
}

/// RW のアンロードを RCS で行うときの入力（スラスタが無ければ `None`）。
pub(super) fn rcs(
    thrusters: &[Thruster],
    mp: &MassPropertiesSummary,
) -> Option<RwUnloadingRcsInput> {
    (!thrusters.is_empty()).then(|| RwUnloadingRcsInput {
        thrusters: thrusters.to_vec(),
        center_of_mass: mp.center_of_mass,
        mass: mp.total_mass,
    })
}

/// モード `mode` で噴射するスラスタの推力誤差（噴射するものが無ければ `None`）。
/// 噴射の割合は、モードの割り当てで [`STATE_THRUSTER_FIRING`] にいる時間の割合（`duty` 込み）。蝕の上書きは見ない。
pub(super) fn disturbance(
    ctx: &Context<'_>,
    mode: Option<&str>,
    thrusters: &[Thruster],
    mp: &MassPropertiesSummary,
    m: &mut MissingInputs,
) -> Option<ThrustDisturbance> {
    let mode = ctx.design.modes.get(mode?)?;
    let def = mode_def(mode);
    let firing: Vec<FiringThruster> = thrusters
        .iter()
        .filter_map(|t| {
            let a = def.assignment(&t.id)?;
            let share = match a.fractions() {
                Ok(f) => f
                    .iter()
                    .filter(|(s, _)| s == STATE_THRUSTER_FIRING)
                    .map(|(_, f)| f)
                    .sum::<f64>(),
                Err(e) => {
                    m.push(
                        Target::mode(def.id.as_str(), Some(&t.id)),
                        format!("Thruster state assignment of mode {} ({e})", def.id),
                    );
                    return None;
                }
            };
            (share > 0.0).then(|| FiringThruster {
                thruster: t.clone(),
                firing_fraction: Fraction::new(share),
            })
        })
        .collect();
    (!firing.is_empty()).then_some(ThrustDisturbance {
        center_of_mass: mp.center_of_mass,
        firing,
    })
}
