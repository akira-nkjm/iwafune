//! AOCS の射影: `aocs.rw_unloading`・`aocs.unload_time`・`aocs.sensor_fov`。慣性は structure の射影から sat-analysis で求める。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::parts::{self, by_role, direction_body};
use crate::quantity::vec_si;
use crate::{structure, timeline};
use fune_model::Target;
use fune_model::design::setting_dims as dims;
use fune_model::path::SettingPath;
use sat_analysis::aocs::disturbance::{DisturbanceSpacecraft, DisturbanceSurface};
use sat_analysis::aocs::sensor_fov::SensorFovSensor;
use sat_analysis::aocs::unload_time::{UnloadTimeField, UnloadTimeMtq, UnloadTimeThruster};
use sat_analysis::aocs::wheels::RwWheel;
use sat_analysis::aocs::{RwUnloadingInput, SensorFovInput, UnloadTimeInput};
use sat_astro::math::Vec3;
use sat_units::typed::{
    Angle, AngularMomentum, Force, Fraction, MagneticDipoleMoment, Qty, Time, Torque,
};

/// センサの役割（禁止角を調べる対象）。
const SENSOR_ROLES: &[&str] = &["star_tracker", "sun_sensor", "camera"];

fn settings<'a>(ctx: &'a Context<'_>) -> Option<&'a fune_model::design::AocsSettings> {
    ctx.design.analysis.aocs.as_ref()
}

/// リアクションホイール（role = reaction_wheel）。
pub fn wheels(ctx: &Context<'_>, m: &mut MissingInputs) -> Vec<RwWheel> {
    let out: Vec<RwWheel> = by_role(ctx, "reaction_wheel")
        .filter_map(|p| {
            let why = "リアクションホイールの値";
            let axis = direction_body(ctx, p, "spin_axis", "回転軸（部品座標）", m);
            let h = parts::part_qty::<AngularMomentum>(p, "max_momentum", why, m);
            let t = parts::part_qty::<Torque>(p, "max_torque", why, m);
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
            Target::any_of_roles(&["reaction_wheel"], None),
            "リアクションホイール（role = reaction_wheel）の部品がありません",
        );
    }
    out
}

/// 擾乱の計算に使う機体の性質（慣性・圧力中心・残留磁気）。
fn spacecraft(ctx: &Context<'_>, m: &mut MissingInputs) -> Option<DisturbanceSpacecraft> {
    let summary = structure::summary(ctx, m)?;
    let s = settings(ctx);
    // ベクトルの設定（パスは設定の次元の表の欄から。単位どおりに SI へ直す）
    let v = |x: Option<&fune_model::value::Vector3>, path: SettingPath, m: &mut MissingInputs| {
        x.and_then(|x| vec_si(x, path, m)).unwrap_or(Vec3::ZERO)
    };
    let size = reference_box(ctx, m);
    let surface =
        |coef: Option<f64>, cp: Vec3| Some(DisturbanceSurface::from_box(size?, coef?, cp));
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
    Some(DisturbanceSpacecraft {
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
    })
}

/// 取付の根の部品（構体）の外形を渡す。面積の近似は解析が行う。
fn reference_box(ctx: &Context<'_>, m: &mut MissingInputs) -> Option<Vec3> {
    let roots: Vec<_> = ctx
        .resolved
        .parts
        .values()
        .filter(|p| p.mount.is_none() && p.vec3("size").is_some())
        .collect();
    let root = match roots.as_slice() {
        [] => return None,
        [root] => root,
        _ => {
            let ids: Vec<_> = roots.iter().map(|p| p.id.as_str()).collect();
            m.push(
                Target::any_part("size"),
                format!(
                    "機体外形の候補が複数あり曖昧です（{}）。取付関係を指定してください",
                    ids.join("、")
                ),
            );
            return None;
        }
    };
    let [x, y, z] = root.vec3("size")?;
    Some(Vec3::new(x, y, z))
}

/// RW の角運動量のうち使ってよい割合（書かれていなければ `None`。既定値は解析が持つ）。
fn usable_momentum_fraction(ctx: &Context<'_>) -> Option<Fraction> {
    settings(ctx)
        .and_then(|s| s.usable_momentum_fraction)
        .map(Fraction::new)
}

/// センサ FoV の時系列の周回数（禁止角への侵入の割合は 1 周で数える）。
const SENSOR_FOV_ORBITS: u32 = 1;

/// アンロード時間の地磁気の大きさを平均する周回数。
const UNLOAD_FIELD_ORBITS: u32 = 1;

/// `aocs.rw_unloading`
pub fn rw_unloading(ctx: &Context<'_>) -> Result<RwUnloadingInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let mode = timeline::case_mode_id(ctx);
    let t = timeline::timeline(ctx, mode.as_deref(), timeline::DEFAULT_ORBITS, &mut m);
    let wheels = wheels(ctx, &mut m);
    let sc = spacecraft(ctx, &mut m);
    m.finish(|| {
        Some(RwUnloadingInput {
            timeline: t?,
            spacecraft: sc?,
            wheels,
            usable_momentum_fraction: usable_momentum_fraction(ctx),
            required_unload_interval: None,
        })
    })
}

/// `aocs.unload_time`: 最大のホイールの使える角運動量を MTQ で捨てる時間。
pub fn unload_time(ctx: &Context<'_>) -> Result<UnloadTimeInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let wheels = wheels(ctx, &mut m);
    let mtq: Vec<UnloadTimeMtq> = by_role(ctx, "magnetorquer")
        .filter_map(|p| {
            Some(UnloadTimeMtq {
                id: p.id.clone(),
                axis: direction_body(ctx, p, "axis", "双極子の向き（部品座標）", &mut m)?,
                max_dipole: parts::part_qty::<MagneticDipoleMoment>(
                    p,
                    "max_dipole",
                    "MTQ の値",
                    &mut m,
                )?,
            })
        })
        .collect();
    let field_sampling = timeline::orbits_sampling(ctx, UNLOAD_FIELD_ORBITS, &mut m);
    let thrusters = thrusters(ctx, &mut m);
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

/// スラスタ（role = thruster）。位置は取付をたどった機体座標の部品原点、推力の向きは部品座標の `direction` を機体座標に回したもの。
fn thrusters(ctx: &Context<'_>, m: &mut MissingInputs) -> Vec<UnloadTimeThruster> {
    by_role(ctx, "thruster")
        .filter_map(|p| {
            let why = "スラスタの値";
            let thrust = parts::part_qty::<Force>(p, "thrust", why, m);
            let isp = parts::part_qty::<Time>(p, "isp", why, m);
            let direction = direction_body(ctx, p, "direction", "推力の向き（部品座標）", m);
            let (position, _) = crate::pose::body_pose(ctx.resolved, &p.id)?;
            Some(UnloadTimeThruster {
                id: p.id.clone(),
                thrust: thrust?,
                isp: isp?,
                position,
                direction: direction?,
            })
        })
        .collect()
}

/// `aocs.sensor_fov`: 視線（boresight）を持つセンサ。
pub fn sensor_fov(ctx: &Context<'_>) -> Result<SensorFovInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let mode = timeline::case_mode_id(ctx);
    let t = timeline::timeline(ctx, mode.as_deref(), SENSOR_FOV_ORBITS, &mut m);
    let angle =
        |p: &fune_model::resolve::ResolvedPart, a: &str| p.scalar(a).map(Qty::<Angle>::from_raw);
    let sensors: Vec<SensorFovSensor> = ctx
        .resolved
        .parts
        .values()
        .filter(|p| p.role.as_deref().is_some_and(|r| SENSOR_ROLES.contains(&r)))
        .filter_map(|p| {
            Some(SensorFovSensor {
                id: p.id.clone(),
                boresight: direction_body(ctx, p, "boresight", "視線の向き（部品座標）", &mut m)?,
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
            "視線を持つセンサ（role = star_tracker / sun_sensor / camera）がありません",
        );
    }
    m.finish(|| {
        Some(SensorFovInput {
            timeline: t?,
            sensors,
        })
    })
}
