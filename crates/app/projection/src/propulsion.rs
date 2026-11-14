//! 推進系の射影: `propulsion.maneuvers`・`propulsion.dv_budget`・`propulsion.propellant`・`propulsion.thruster_usage`・
//! `propulsion.orbit_history`（同じ入力 `PropulsionInput`。RCS のアンロードは ΔV バジェットだけが使う）と
//! `propulsion.mass_properties`（質量特性の入力も足す）。
//!
//! - 数値伝搬（`propagated`）のマヌーバは `[constraints]` を `ThrustConstraints` に写し、無視した制約にしない。
//!   スラスタが無ければ不足の入力にする。
//!
//! - 軌道は前提条件の軌道（`conditions/orbit.toml`）、マヌーバは `Design::ordered_maneuvers` の実行の順。
//! - 乾燥質量は `analysis.propulsion.dry_mass`、無ければ部品の質量特性の総質量（structure の射影）。
//! - スラスタは役割 `propulsion_thruster` の部品と、マヌーバが指す部品。タンクは役割 `propellant_tank` の部品。
//! - 噴射中の消費電力は部品の `power` の噴射の状態（`STATE_THRUSTER_FIRING`）の値。
//! - 推進系が有るか（[`present`]）は、マヌーバか推進系の部品（`propulsion_thruster`・`propellant_tank`）があるか。
//!   質量のバジェットの推薬と高度低下の軌道維持が同じ判断を使う。
//! - マージンなどの既定値は持たない（解析が持つ）。区分と計算の段階は model ⇄ analysis の対応で写す。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::parts::{self, by_role};
use crate::quantity::{qty_opt, setting_opt};
use crate::{aocs, structure};
use fune_model::Target;
use fune_model::builtin::{Role, STATE_THRUSTER_FIRING};
use fune_model::design::setting_dims as dims;
use fune_model::design::{Maneuver, ManeuverCategory, ManeuverMethod, ManeuverSteering};
use fune_model::resolve::{ResolvedPart, SiValue};
use sat_analysis::propulsion::{
    DvCategory, ManeuverInput, OrbitTarget, PropellantMassInput, PropellantTank, PropulsionInput,
    PropulsionThruster, ThrustConstraints, TransferMethod, UnloadingInput, UnloadingNote,
};
use sat_astro::force::thrust::ThrustSteering;
use sat_units::typed::{Force, Fraction, Impulse, MassDensity, Power, Qty, Time, Volume};

/// 設計の区分 → 解析の区分。
pub fn category(c: ManeuverCategory) -> DvCategory {
    match c {
        ManeuverCategory::Insertion => DvCategory::Insertion,
        ManeuverCategory::StationKeeping => DvCategory::StationKeeping,
        ManeuverCategory::Unloading => DvCategory::Unloading,
        ManeuverCategory::Deorbit => DvCategory::Deorbit,
        ManeuverCategory::Other => DvCategory::Other,
    }
}

/// 設計の計算の段階 → 解析の計算の段階。
pub fn method(m: ManeuverMethod) -> TransferMethod {
    match m {
        ManeuverMethod::Impulsive => TransferMethod::Impulsive,
        ManeuverMethod::FiniteBurn => TransferMethod::FiniteBurn,
        ManeuverMethod::LowThrust => TransferMethod::LowThrust,
        ManeuverMethod::Propagated => TransferMethod::Propagated,
    }
}

/// 設計の推力の向き → 解析の推力の向き。
pub fn steering(s: ManeuverSteering) -> ThrustSteering {
    match s {
        ManeuverSteering::Tangential => ThrustSteering::Tangential,
        ManeuverSteering::Inertial => ThrustSteering::Inertial,
    }
}

/// 解析の推力の向き → 設計の推力の向き（表示名は設計の語彙 `maneuver_steering`）。
pub fn model_steering(s: ThrustSteering) -> ManeuverSteering {
    match s {
        ThrustSteering::Tangential => ManeuverSteering::Tangential,
        ThrustSteering::Inertial => ManeuverSteering::Inertial,
    }
}

/// 解析の区分 → 設計の区分（表示名は設計の語彙 `maneuver_category` から引く）。
pub fn model_category(c: DvCategory) -> ManeuverCategory {
    match c {
        DvCategory::Insertion => ManeuverCategory::Insertion,
        DvCategory::StationKeeping => ManeuverCategory::StationKeeping,
        DvCategory::Unloading => ManeuverCategory::Unloading,
        DvCategory::Deorbit => ManeuverCategory::Deorbit,
        DvCategory::Other => ManeuverCategory::Other,
    }
}

/// 解析の計算の段階 → 設計の計算の段階（表示名は設計の語彙 `maneuver_method`）。
pub fn model_method(m: TransferMethod) -> ManeuverMethod {
    match m {
        TransferMethod::Impulsive => ManeuverMethod::Impulsive,
        TransferMethod::FiniteBurn => ManeuverMethod::FiniteBurn,
        TransferMethod::LowThrust => ManeuverMethod::LowThrust,
        TransferMethod::Propagated => ManeuverMethod::Propagated,
    }
}

/// `propulsion.maneuvers`・`propulsion.propellant`・`propulsion.thruster_usage`（アンロードを含めない）。
pub fn maneuvers(ctx: &Context<'_>) -> Result<PropulsionInput, MissingInputs> {
    input(ctx, false)
}

/// 推進系が有るか: マヌーバか、推進系の部品（役割 `propulsion_thruster`・`propellant_tank`）がある。
/// 無ければ推薬を数えない（質量のバジェットの推薬・高度低下の軌道維持）。有れば推進系の入力が要る。
pub fn present(ctx: &Context<'_>) -> bool {
    !ctx.design.maneuvers.is_empty()
        || by_role(ctx, Role::PropulsionThruster)
            .chain(by_role(ctx, Role::PropellantTank))
            .next()
            .is_some()
}

/// `propulsion.dv_budget`（RCS でのアンロードの ΔV を含める）。
pub fn dv_budget(ctx: &Context<'_>) -> Result<PropulsionInput, MissingInputs> {
    input(ctx, true)
}

fn input(ctx: &Context<'_>, with_unloading: bool) -> Result<PropulsionInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let orbit = ctx.orbit(&mut m);
    let s = ctx.design.analysis.propulsion.as_ref();
    let dry = match setting_opt(
        &dims::PROPULSION_DRY_MASS,
        &[],
        s.and_then(|s| s.dry_mass.as_ref()),
        &mut m,
    ) {
        Some(d) => Some(d),
        None => structure::summary(ctx, &mut m).map(|mp| mp.total_mass),
    };
    let ordered = ctx.design.ordered_maneuvers();
    if ordered.is_empty() {
        m.push(
            Target::Maneuvers,
            "マヌーバ（maneuvers/<ID>.toml）がありません",
        );
    }
    let maneuvers: Vec<ManeuverInput> = ordered
        .iter()
        .filter_map(|x| maneuver(ctx, x, &mut m))
        .collect();
    let thrusters = thrusters(ctx, &ordered, &mut m);
    let tanks = tanks(ctx, &mut m);
    let (unloading, unloading_note) = if with_unloading {
        unloading(ctx)
    } else {
        (None, None)
    };
    m.finish(|| {
        Some(PropulsionInput {
            initial_orbit: orbit?,
            dry_mass: dry?,
            thrusters,
            tanks,
            maneuvers,
            dv_margin: s.and_then(|s| s.dv_margin).map(Fraction::new),
            residual_fraction: s.and_then(|s| s.residual_fraction).map(Fraction::new),
            unloading,
            unloading_note,
        })
    })
}

/// マヌーバ 1 つ（量の次元が違えば記録して `None`）。
fn maneuver(ctx: &Context<'_>, x: &Maneuver, m: &mut MissingInputs) -> Option<ManeuverInput> {
    let id = x.id.as_str();
    let q = |key: &str, v, m: &mut MissingInputs| {
        let target = Target::maneuver(id, Some(key));
        qty_opt::<sat_units::typed::Length>(v, target, m)
    };
    let t = &x.target;
    let target = OrbitTarget {
        body: t.body.map(crate::context::central_body),
        altitude: q("target.altitude", t.altitude.as_ref(), m),
        perigee_altitude: q("target.perigee_altitude", t.perigee_altitude.as_ref(), m),
        apogee_altitude: q("target.apogee_altitude", t.apogee_altitude.as_ref(), m),
        inclination: qty_opt(
            t.inclination.as_ref(),
            Target::maneuver(id, Some("target.inclination")),
            m,
        ),
    };
    let propagated = x.method == ManeuverMethod::Propagated;
    if propagated && x.thruster.is_none() {
        m.push(
            Target::maneuver(id, Some("thruster")),
            "数値伝搬（propagated）にはスラスタ（thruster）が要ります",
        );
        return None;
    }
    if let Some(p) = &x.thruster
        && !ctx.resolved.parts.contains_key(p.as_str())
    {
        m.push(
            Target::maneuver(id, Some("thruster")),
            format!("マヌーバのスラスタ {p} の部品がありません"),
        );
        return None;
    }
    Some(ManeuverInput {
        id: id.to_owned(),
        category: category(x.category),
        method: method(x.method),
        thruster: x.thruster.as_ref().map(|p| p.to_string()),
        target,
        via_altitude: q("via_altitude", x.via_altitude.as_ref(), m),
        ignored_constraints: if propagated {
            Vec::new()
        } else {
            x.constraints
                .keys()
                .into_iter()
                .map(str::to_owned)
                .collect()
        },
        thrust: if propagated {
            thrust_constraints(x)
        } else {
            ThrustConstraints::default()
        },
    })
}

/// 数値伝搬で効く制約（`[constraints]`）。
fn thrust_constraints(x: &Maneuver) -> ThrustConstraints {
    let c = &x.constraints;
    ThrustConstraints {
        sunlit_only: c.sunlit_only.unwrap_or(false),
        mode: c.mode.as_ref().map(ToString::to_string),
        duty: c.duty.map(Fraction::new),
        steering: c.steering.map_or(ThrustSteering::default(), steering),
    }
}

/// 数値伝搬のマヌーバがあるか（あればほかの解析がマヌーバの順の軌道の推移を使う）。
pub fn has_propagated(ctx: &Context<'_>) -> bool {
    ctx.design
        .maneuvers
        .values()
        .any(|x| x.method == ManeuverMethod::Propagated)
}

/// `propulsion.mass_properties`（乾燥の部品の質量特性の入力と推進系の入力）。
pub fn mass_properties(ctx: &Context<'_>) -> Result<PropellantMassInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let structure = structure::mass_properties(ctx)
        .map_err(|e| m.extend(e))
        .ok();
    let propulsion = maneuvers(ctx).map_err(|e| m.extend(e)).ok();
    m.finish(|| {
        Some(PropellantMassInput {
            structure: structure?,
            propulsion: propulsion?,
        })
    })
}

/// 噴射の状態の消費電力（状態ごとの値ならその状態、スカラーならその値）。
fn firing_power(ctx: &Context<'_>, p: &ResolvedPart) -> Option<Qty<Power>> {
    match ctx.attr(p, "power")? {
        SiValue::ByState(map) => map
            .get(STATE_THRUSTER_FIRING)
            .map(|v| Qty::from_raw(v.value)),
        SiValue::Scalar(v) => Some(Qty::from_raw(v.value)),
        _ => None,
    }
}

/// 名前（列挙の値）の属性。
fn name_of(ctx: &Context<'_>, p: &ResolvedPart, attr: &str) -> Option<String> {
    match ctx.attr(p, attr)? {
        SiValue::Enum(s) | SiValue::Text(s) => Some(s.clone()),
        _ => None,
    }
}

/// 推進系のスラスタ（役割 `propulsion_thruster`）と、マヌーバが指す部品（ID 順）。
fn thrusters(
    ctx: &Context<'_>,
    ordered: &[&Maneuver],
    m: &mut MissingInputs,
) -> Vec<PropulsionThruster> {
    let referenced: Vec<&str> = ordered
        .iter()
        .filter_map(|x| x.thruster.as_ref().map(|p| p.as_str()))
        .collect();
    ctx.resolved
        .parts
        .values()
        .filter(|p| p.role == Some(Role::PropulsionThruster) || referenced.contains(&p.id.as_str()))
        .filter_map(|p| {
            let why = "推進系のスラスタの値";
            let thrust = parts::part_qty::<Force>(ctx, p, "thrust", why, m);
            let isp = parts::part_qty::<Time>(ctx, p, "isp", why, m);
            let rated_firings = ctx
                .scalar(p, "rated_firings")
                .filter(|n| n.is_finite() && *n >= 0.0)
                .map(|n| n.round().min(f64::from(u32::MAX)) as u32);
            Some(PropulsionThruster {
                id: p.id.clone(),
                thrust: thrust?,
                isp: isp?,
                propellant: name_of(ctx, p, "propellant"),
                min_impulse_bit: ctx
                    .scalar(p, "min_impulse_bit")
                    .map(Qty::<Impulse>::from_raw),
                rated_total_impulse: ctx.scalar(p, "rated_total_impulse").map(Qty::from_raw),
                rated_firing_time: ctx.scalar(p, "rated_firing_time").map(Qty::from_raw),
                rated_firings,
                firing_power: firing_power(ctx, p),
            })
        })
        .collect()
}

/// 推薬タンク（役割 `propellant_tank`）。
fn tanks(ctx: &Context<'_>, m: &mut MissingInputs) -> Vec<PropellantTank> {
    by_role(ctx, Role::PropellantTank)
        .filter_map(|p| {
            let volume = parts::part_qty::<Volume>(ctx, p, "tank_volume", "タンクの容積", m)?;
            Some(PropellantTank {
                id: p.id.clone(),
                volume,
                propellant: name_of(ctx, p, "propellant"),
                density: ctx
                    .scalar(p, "propellant_density")
                    .map(Qty::<MassDensity>::from_raw),
            })
        })
        .collect()
}

/// RCS でのアンロードの入力（`aocs.rw_unloading` の射影とミッション期間）。足せなければ理由。
fn unloading(ctx: &Context<'_>) -> (Option<UnloadingInput>, Option<UnloadingNote>) {
    if by_role(ctx, Role::Thruster).next().is_none() {
        return (None, Some(UnloadingNote::NoRcs));
    }
    let Ok(rw) = aocs::rw_unloading(ctx) else {
        return (None, Some(UnloadingNote::UnloadingInputsMissing));
    };
    if rw.rcs.is_none() {
        return (None, Some(UnloadingNote::NoRcs));
    }
    let mut ignored = MissingInputs::new();
    let duration = setting_opt(
        &dims::ORBIT_DURATION,
        &[],
        ctx.design
            .conditions
            .orbit
            .as_ref()
            .and_then(|o| o.duration.as_ref()),
        &mut ignored,
    );
    match duration {
        Some(d) => (
            Some(UnloadingInput {
                rw_unloading: rw,
                mission_duration: d,
            }),
            None,
        ),
        None => (None, Some(UnloadingNote::NoMissionDuration)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 区分と計算の段階の綴りは設計と解析で同じ。
    #[test]
    fn vocabularies_have_the_same_spelling_on_both_sides() {
        for c in ManeuverCategory::ALL {
            assert_eq!(serde_json::to_value(category(*c)).unwrap(), c.as_str());
            assert_eq!(model_category(category(*c)), *c);
        }
        for x in ManeuverMethod::ALL {
            assert_eq!(serde_json::to_value(method(*x)).unwrap(), x.as_str());
            assert_eq!(model_method(method(*x)), *x);
        }
        assert_eq!(DvCategory::ALL.len(), ManeuverCategory::ALL.len());
        for s in ManeuverSteering::ALL {
            assert_eq!(serde_json::to_value(steering(*s)).unwrap(), s.as_str());
            assert_eq!(model_steering(steering(*s)), *s);
        }
    }
}
