//! `orbit.total_dose` の射影: 区間（フェーズごと。フェーズに期間が無い設計は設計の軌道とミッション期間の 1 区間）の環境の表と期間、
//! 部品の遮蔽厚さ・許容線量、設計全体の既定の遮蔽厚さと設計余裕。既定値を持たず、物理の計算もしない（表を引くのは解析）。
//! 環境の表は軌道の設定（`dose_rate_by_shielding`。フェーズの `[orbit]` を持てばそのフェーズのもの）から読む。

use crate::quantity::{curve_points, qty_opt};
use crate::{Context, MissingInputs};
use fune_model::Target;
use fune_model::design::setting_dims::{self as dims, ORBIT_DOSE_RATE_BY_SHIELDING};
use fune_model::path::SettingPath;
use sat_analysis::orbit::{DoseDepthPoint, DoseLegInput, DosePartInput, TotalDoseInput};
use sat_units::typed::{Length, Qty, Time};

/// 属性: 部品の遮蔽厚さ（等価アルミ）。
const SHIELDING: &str = "shielding";
/// 属性: 部品の許容総線量。
const TOLERANCE: &str = "tid_tolerance";

/// `orbit.total_dose` の入力。
pub fn total_dose(ctx: &Context<'_>) -> Result<TotalDoseInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let legs = legs(ctx, &mut m);
    let parts = parts(ctx, &mut m);
    let radiation = ctx
        .design
        .analysis
        .common
        .as_ref()
        .and_then(|c| c.radiation.as_ref());
    let path = |key: &str| SettingPath::analysis(&["common", "radiation", key]);
    let default_shielding = qty_opt::<Length>(
        radiation.and_then(|r| r.default_shielding.as_ref()),
        dims::RADIATION_DEFAULT_SHIELDING.at(&[]),
        &mut m,
    );
    ctx.note(dims::RADIATION_DEFAULT_SHIELDING.at(&[]));
    let design_margin = ctx.setting(
        path("design_margin"),
        radiation.and_then(|r| r.design_margin),
    );
    if legs
        .iter()
        .all(|l| l.duration.is_none() || l.environment.is_none())
    {
        m.extend(not_usable(ctx));
    }
    if parts.is_empty() {
        m.push(
            Target::any_part(TOLERANCE),
            "A part with a total dose tolerance (tid_tolerance) or a shielding thickness",
        );
    }
    m.finish(|| {
        Some(TotalDoseInput {
            legs,
            parts,
            default_shielding,
            design_margin,
        })
    })
}

/// 区間の並び。フェーズに期間が 1 つでもあればフェーズごと、なければ設計の軌道とミッション期間の 1 区間
/// （`propulsion` の RCS アンロードの区間と同じ決め方）。
fn legs(ctx: &Context<'_>, m: &mut MissingInputs) -> Vec<DoseLegInput> {
    let phases = ctx.design.ordered_phases();
    if phases.iter().any(|p| p.duration.is_some()) {
        return phases
            .into_iter()
            .map(|p| {
                let c = ctx.in_phase(p);
                let target = Target::phase(p.id.as_str(), Some("duration"));
                ctx.note(target.clone());
                DoseLegInput {
                    phase: Some(p.id.as_str().to_owned()),
                    duration: qty_opt::<Time>(p.duration.as_ref(), target, m),
                    environment: environment(&c, m),
                }
            })
            .collect();
    }
    vec![DoseLegInput {
        phase: None,
        duration: ctx.mission_duration(),
        environment: environment(ctx, m),
    }]
}

/// 効いている軌道の環境の表（SI の点の並び）。書かれていなければ `None`（足りない入力は [`not_usable`]）。
fn environment(c: &Context<'_>, m: &mut MissingInputs) -> Option<Vec<DoseDepthPoint>> {
    let key = ORBIT_DOSE_RATE_BY_SHIELDING.path();
    let target = c.orbit_target(
        &key.segments[1..]
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    c.note(target.clone());
    let curve = c.orbit_settings()?.dose_rate_by_shielding.as_ref()?;
    let points = curve_points(curve, target, m)?;
    Some(
        points
            .into_iter()
            .map(|(shielding, rate)| DoseDepthPoint { shielding, rate })
            .collect(),
    )
}

/// どの区間も評価できないときの不足入力: 環境の表（軌道の設定）と期間（フェーズ・ミッション期間）。
fn not_usable(ctx: &Context<'_>) -> MissingInputs {
    let mut m = MissingInputs::new();
    let phases = ctx.design.ordered_phases();
    let by_phase = phases.iter().any(|p| p.duration.is_some());
    let key = ORBIT_DOSE_RATE_BY_SHIELDING.path();
    let keys: Vec<&str> = key.segments[1..].iter().map(String::as_str).collect();
    let reason = "Dose rate versus shielding thickness (an environment table)";
    if by_phase {
        for p in phases {
            let c = ctx.in_phase(p);
            let has = c
                .orbit_settings()
                .is_some_and(|o| o.dose_rate_by_shielding.is_some());
            if !has {
                m.push(c.orbit_target(&keys), reason);
            }
        }
    } else {
        let has = ctx
            .orbit_settings()
            .is_some_and(|o| o.dose_rate_by_shielding.is_some());
        if !has {
            m.push(ctx.orbit_target(&keys), reason);
        }
        if ctx.mission_duration().is_none() {
            m.push(
                Context::mission_duration_target(),
                "Mission duration (an at_least requirement with input = \"mission_duration\")",
            );
        }
    }
    m
}

/// 遮蔽厚さか許容線量を書いた部品（ID 順）。
fn parts(ctx: &Context<'_>, m: &mut MissingInputs) -> Vec<DosePartInput> {
    ctx.resolved
        .parts
        .values()
        .filter(|p| p.value(SHIELDING).is_some() || p.value(TOLERANCE).is_some())
        .map(|p| {
            let shielding = ctx.scalar(p, SHIELDING).map(Qty::from_raw);
            let tolerance = ctx.scalar(p, TOLERANCE).map(Qty::from_raw);
            if p.value(SHIELDING).is_some() && shielding.is_none() {
                m.push(Target::attr(&p.id, SHIELDING), "Part shielding thickness");
            }
            DosePartInput {
                part: p.id.clone(),
                shielding,
                tolerance,
            }
        })
        .collect()
}
