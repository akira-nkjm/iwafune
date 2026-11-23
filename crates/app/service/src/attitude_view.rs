//! 実効姿勢則の軸の表示と、既存の設定編集へのパス。
use crate::dto::{AocsAttitudeView, AocsAxisView, AocsLawField};
use fune_model::design::AnalysisSystem;
use fune_model::value::{Quaternion, Vector3, quaternion_to_text, vector_to_text};
use fune_projection::{Context, MissingInputs};
use sat_astro::attitude::AttitudeLaw;
use sat_astro::math::Vec3;

pub(crate) fn view(ctx: &Context<'_>, mode: &str) -> AocsAttitudeView {
    let prefix = AnalysisSystem::Aocs
        .setting_path()
        .child(&["law", mode])
        .to_string();
    let setting = ctx
        .design
        .analysis
        .aocs
        .as_ref()
        .and_then(|s| s.law.get(mode));
    let effective = ctx.attitude_law(Some(mode), &mut MissingInputs::new());
    view_of(&prefix, setting, effective)
}

/// フェーズの `[law.<モード>]` の姿勢則と実効軸（欄のパスは `fune phase set-field` の `law.<モード>.<キー>`）。
pub(crate) fn phase_view(
    mode: &str,
    setting: &fune_model::design::AttitudeLawSetting,
) -> AocsAttitudeView {
    view_of(
        &format!("law.{mode}"),
        Some(setting),
        Some(fune_projection::context::law_of(setting)),
    )
}

/// 設定（`prefix` はその表のパス）と実効の姿勢則から、欄と軸を作る。
fn view_of(
    prefix: &str,
    setting: Option<&fune_model::design::AttitudeLawSetting>,
    effective: Option<AttitudeLaw>,
) -> AocsAttitudeView {
    let law = setting.map(|s| s.law);
    let mut fields = vec![field(
        prefix,
        "law",
        law.map(|l| l.to_string()).unwrap_or_default(),
    )];
    let axes = effective.as_ref().map(axes).unwrap_or_default();
    if let Some(law) = effective {
        fields.extend(axis_fields(prefix, setting, law));
    }
    AocsAttitudeView { law, fields, axes }
}

fn axis_fields(
    path: &str,
    setting: Option<&fune_model::design::AttitudeLawSetting>,
    law: AttitudeLaw,
) -> Vec<AocsLawField> {
    let axis = |name: &str, value: Option<&Vector3>, fallback: Vec3| {
        field(
            path,
            name,
            vector_to_text(value.unwrap_or(&authored(fallback))),
        )
    };
    match law {
        AttitudeLaw::SunPointing {
            axis: primary,
            secondary,
        } => vec![
            axis("axis", setting.and_then(|s| s.axis.as_ref()), primary),
            axis(
                "secondary",
                setting.and_then(|s| s.secondary.as_ref()),
                secondary,
            ),
        ],
        AttitudeLaw::NadirPointing {
            axis: primary,
            along_track,
        } => vec![
            axis("axis", setting.and_then(|s| s.axis.as_ref()), primary),
            axis(
                "along_track",
                setting.and_then(|s| s.along_track.as_ref()),
                along_track,
            ),
        ],
        AttitudeLaw::Inertial { q_body_to_eci: q } => vec![field(
            path,
            "orientation",
            quaternion_to_text(&Quaternion {
                w: q.w,
                x: q.x,
                y: q.y,
                z: q.z,
            }),
        )],
    }
}

fn axes(law: &AttitudeLaw) -> Vec<AocsAxisView> {
    match law {
        AttitudeLaw::SunPointing { axis, secondary } => vec![
            axis_view(*axis, "太陽", "Sun"),
            axis_view(*secondary, "軌道面の法線", "Orbit normal"),
        ],
        AttitudeLaw::NadirPointing { axis, along_track } => vec![
            axis_view(*axis, "地心", "Nadir"),
            axis_view(*along_track, "進行方向", "Along track"),
        ],
        AttitudeLaw::Inertial { q_body_to_eci: q } => [Vec3::Z, Vec3::X]
            .into_iter()
            .map(|v| {
                let target = format!("ECI {}", vector(q.normalized().rotate(v)));
                axis_view(v, &target, &target)
            })
            .collect(),
    }
}

fn field(prefix: &str, name: &str, text: String) -> AocsLawField {
    AocsLawField {
        field: name.into(),
        path: format!("{prefix}.{name}"),
        text,
    }
}

fn authored(v: Vec3) -> Vector3 {
    Vector3 {
        x: v.x,
        y: v.y,
        z: v.z,
        unit: None,
        frame: None,
    }
}

fn vector(v: Vec3) -> String {
    format!(
        "[{}, {}, {}]",
        crate::units::number_text(v.x),
        crate::units::number_text(v.y),
        crate::units::number_text(v.z)
    )
}

fn axis_view(body: Vec3, target: &str, target_en: &str) -> AocsAxisView {
    AocsAxisView {
        body: vector(body),
        target: target.into(),
        target_en: target_en.into(),
    }
}
