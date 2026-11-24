//! AOCS の射影: `aocs.rw_unloading`・`aocs.unload_time`・`aocs.sensor_fov`・`aocs.slew_time`。慣性・重心・質量は structure の射影から sat-analysis で求める。
//! スラスタ（RCS と推力誤差）は [`thrust`]。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::parts::{self, by_role, direction_body};
use crate::quantity::{setting, vec_si};
use crate::{structure, timeline};
use fune_model::Target;
use fune_model::builtin::Role;
use fune_model::design::setting_dims as dims;
use fune_model::path::SettingPath;
use sat_analysis::aocs::disturbance::{DisturbanceSpacecraft, DisturbanceSurface};
use sat_analysis::aocs::sensor_fov::SensorFovSensor;
use sat_analysis::aocs::unload_time::{UnloadTimeField, UnloadTimeMtq};
use sat_analysis::aocs::wheels::RwWheel;
use sat_analysis::aocs::{RwUnloadingInput, SensorFovInput, SlewTimeInput, UnloadTimeInput};
use sat_analysis::spec::SamplingSpan;
use sat_analysis::structure::mass_properties::MassPropertiesSummary;
use sat_astro::math::Vec3;
use sat_units::typed::{Angle, AngularMomentum, Fraction, MagneticDipoleMoment, Qty, Time, Torque};

mod thrust;

/// センサの役割（禁止角を調べる対象）。
const SENSOR_ROLES: &[Role] = &[Role::StarTracker, Role::SunSensor, Role::Camera];

fn settings<'a>(ctx: &'a Context<'_>) -> Option<&'a fune_model::design::AocsSettings> {
    ctx.design.analysis.aocs.as_ref()
}

/// リアクションホイール（role = reaction_wheel）。
pub fn wheels(ctx: &Context<'_>, m: &mut MissingInputs) -> Vec<RwWheel> {
    let out: Vec<RwWheel> = by_role(ctx, Role::ReactionWheel)
        .filter_map(|p| {
            let why = "Reaction wheel values";
            let axis = direction_body(ctx, p, "spin_axis", "Spin axis (part frame)", m);
            let h = parts::part_qty::<AngularMomentum>(ctx, p, "max_momentum", why, m);
            let t = parts::part_qty::<Torque>(ctx, p, "max_torque", why, m);
            Some(RwWheel {
                id: p.id.clone(),
                axis: axis?,
                max_momentum: h?,
                max_torque: t?,
            })
        })
        .collect();
    if out.is_empty() {
        m.push(
            Target::any_of_roles(&[Role::ReactionWheel], None),
            "No reaction wheel part (role = reaction_wheel)",
        );
    }
    out
}

/// 擾乱の計算に使う機体の性質（慣性・圧力中心・残留磁気）。推力誤差は呼び出し側が足す。
fn spacecraft(
    ctx: &Context<'_>,
    summary: &MassPropertiesSummary,
    m: &mut MissingInputs,
) -> DisturbanceSpacecraft {
    let s = settings(ctx);
    // ベクトルの設定（パスは設定の次元の表の欄から。単位どおりに SI へ直す）
    let v = |x: Option<&fune_model::value::Vector3>, path: SettingPath, m: &mut MissingInputs| {
        x.and_then(|x| vec_si(x, path, m))
    };
    let size = reference_box(ctx, m);
    let surface =
        |coef: Option<f64>, cp: Option<Vec3>| Some(DisturbanceSurface::from_box(size?, coef?, cp));
    let aero_cp = v(
        s.and_then(|s| s.cp_offset_aero.as_ref()),
        dims::AOCS_CP_OFFSET_AERO.at(&[]),
        m,
    );
    let srp_cp = v(
        s.and_then(|s| s.cp_offset_srp.as_ref()),
        dims::AOCS_CP_OFFSET_SRP.at(&[]),
        m,
    );
    DisturbanceSpacecraft {
        inertia: summary.inertia,
        aero: surface(s.and_then(|s| s.drag_coefficient), aero_cp),
        srp: surface(s.and_then(|s| s.srp_coefficient), srp_cp),
        residual_dipole: v(
            s.and_then(|s| s.residual_dipole.as_ref()),
            dims::AOCS_RESIDUAL_DIPOLE.at(&[]),
            m,
        ),
        atmosphere: Default::default(),
        density_scale: None,
        thrust: None,
    }
}

/// 取付の根の部品（構体）の外形を渡す。面積の近似は解析が行う（空力・太陽輻射圧トルクと高度低下が共有する）。
pub(crate) fn reference_box(ctx: &Context<'_>, m: &mut MissingInputs) -> Option<Vec3> {
    let roots = crate::outline::candidates(ctx.resolved);
    let root = match roots.as_slice() {
        [] => return None,
        [root] => root,
        _ => {
            let ids: Vec<_> = roots.iter().map(|p| p.id.as_str()).collect();
            m.push(
                Target::any_part("size"),
                format!(
                    "The body outline is ambiguous: several candidates ({}). Specify the mount relation",
                    ids.join(", ")
                ),
            );
            return None;
        }
    };
    let size = ctx.vec3(root, "size")?;
    let [x, y, z] = crate::outline::body_extents(ctx.resolved, &root.id, size);
    Some(Vec3::new(x, y, z))
}

/// RW の角運動量のうち使ってよい割合（書かれていなければ `None`。既定値は解析が持つ）。
fn usable_momentum_fraction(ctx: &Context<'_>) -> Option<Fraction> {
    settings(ctx)
        .and_then(|s| s.usable_momentum_fraction)
        .map(Fraction::new)
}

/// `aocs.rw_unloading`
pub fn rw_unloading(ctx: &Context<'_>) -> Result<RwUnloadingInput, MissingInputs> {
    rw_unloading_for_mode(ctx, timeline::case_mode_id(ctx).as_deref())
}

/// 明示したモードの姿勢則で RW を評価する入力。ケースの軌道条件は保持する。
/// モードで噴射するスラスタがあれば推力誤差を擾乱に足し、スラスタがあれば RCS で捨てる 1 日の推薬も求める。
pub fn rw_unloading_for_mode(
    ctx: &Context<'_>,
    mode: Option<&str>,
) -> Result<RwUnloadingInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let t = timeline::timeline(ctx, mode, None, &mut m);
    let wheels = wheels(ctx, &mut m);
    let thrusters = thrust::thrusters(ctx, &mut m);
    let summary = structure::summary(ctx, &mut m);
    let sc = summary.as_ref().map(|mp| DisturbanceSpacecraft {
        thrust: thrust::disturbance(ctx, mode, &thrusters, mp, &mut m),
        ..spacecraft(ctx, mp, &mut m)
    });
    let rcs = summary.as_ref().and_then(|mp| thrust::rcs(&thrusters, mp));
    m.finish(|| {
        Some(RwUnloadingInput {
            timeline: t?,
            spacecraft: sc?,
            wheels,
            usable_momentum_fraction: usable_momentum_fraction(ctx),
            required_unload_interval: None,
            rcs,
        })
    })
}

/// `aocs.unload_time`: 最大のホイールの使える角運動量を MTQ で捨てる時間。
pub fn unload_time(ctx: &Context<'_>) -> Result<UnloadTimeInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let wheels = wheels(ctx, &mut m);
    let mtq: Vec<UnloadTimeMtq> = by_role(ctx, Role::Magnetorquer)
        .filter_map(|p| {
            Some(UnloadTimeMtq {
                id: p.id.clone(),
                axis: direction_body(ctx, p, "axis", "Dipole direction (part frame)", &mut m)?,
                max_dipole: parts::part_qty::<MagneticDipoleMoment>(
                    ctx,
                    p,
                    "max_dipole",
                    "Magnetorquer values",
                    &mut m,
                )?,
            })
        })
        .collect();
    let field_sampling = timeline::sampling_of(ctx, SamplingSpan::OrbitAverage, &mut m);
    let thrusters = thrust::thrusters(ctx, &mut m);
    let center_of_mass = if thrusters.is_empty() {
        None
    } else {
        structure::summary(ctx, &mut m).map(|s| s.center_of_mass)
    };
    m.finish(|| {
        Some(UnloadTimeInput {
            momentum: None,
            wheels,
            usable_momentum_fraction: usable_momentum_fraction(ctx),
            momentum_direction: None,
            magnetorquers: mtq,
            field: Some(UnloadTimeField::Orbit {
                sampling: field_sampling?,
            }),
            mtq_efficiency: settings(ctx)
                .and_then(|s| s.unload_dipole_fraction)
                .map(Fraction::new),
            thrusters,
            center_of_mass,
            required_time: None,
        })
    })
}

/// `aocs.sensor_fov`: 視線（boresight）を持つセンサ。
pub fn sensor_fov(ctx: &Context<'_>) -> Result<SensorFovInput, MissingInputs> {
    sensor_fov_for_mode(ctx, timeline::case_mode_id(ctx).as_deref())
}

/// 明示したモードの姿勢則で視野を評価する入力。センサの選択は通常の解析と同じ。
pub fn sensor_fov_for_mode(
    ctx: &Context<'_>,
    mode: Option<&str>,
) -> Result<SensorFovInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let t = timeline::timeline_over(ctx, mode, SamplingSpan::OrbitAverage, &mut m);
    let angle = |p: &fune_model::resolve::ResolvedPart, a: &str| {
        ctx.scalar(p, a).map(Qty::<Angle>::from_raw)
    };
    let sensors: Vec<SensorFovSensor> = ctx
        .resolved
        .parts
        .values()
        .filter(|p| p.role.is_some_and(|r| SENSOR_ROLES.contains(&r)))
        .filter_map(|p| {
            Some(SensorFovSensor {
                id: p.id.clone(),
                boresight: direction_body(
                    ctx,
                    p,
                    "boresight",
                    "Boresight direction (part frame)",
                    &mut m,
                )?,
                fov_half_angle: angle(p, "fov_half_angle"),
                exclusion_sun: angle(p, "exclusion_sun"),
                exclusion_earth: angle(p, "exclusion_earth"),
                exclusion_moon: angle(p, "exclusion_moon"),
                max_intrusion_fraction: None,
            })
        })
        .collect();
    if sensors.is_empty() {
        m.push(
            Target::any_of_roles(SENSOR_ROLES, Some("boresight")),
            "No sensor with a boresight (role = star_tracker / sun_sensor / camera)",
        );
    }
    m.finish(|| {
        Some(SensorFovInput {
            timeline: t?,
            sensors,
        })
    })
}

/// `aocs.slew_time`: 設定の角度（`slew_angle`）を、機体の x・y・z の最悪の軸まわりに回る時間。慣性は質量特性から。
pub fn slew_time(ctx: &Context<'_>) -> Result<SlewTimeInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let wheels = wheels(ctx, &mut m);
    let summary = structure::summary(ctx, &mut m);
    let s = settings(ctx);
    let angle = setting(
        &dims::AOCS_SLEW_ANGLE,
        &[],
        s.and_then(|s| s.slew_angle.as_ref()),
        "Slew angle",
        &mut m,
    );
    // 姿勢変更の時間の上限は要求が持つ（`aocs.slew_time` の `slew_time`）。無ければ判定しない
    let required_time = ctx
        .criterion("aocs.slew_time", "slew_time")
        .map(|c| Qty::<Time>::from_raw(c.si));
    m.finish(|| {
        Some(SlewTimeInput {
            inertia: summary?.inertia,
            wheels,
            usable_momentum_fraction: usable_momentum_fraction(ctx),
            angle: angle?,
            axis: None,
            required_time,
        })
    })
}
