//! 系ごとのモードの見方（電源の消費電力・C&DH のデータの発生量・熱の発熱・AOCS の姿勢則）。
//! 値・合計・判定はここで作り、web は並べるだけ（`POST /system_modes/{system}`）。

mod aocs;
mod devices;
mod failure;
mod power;

use crate::dto::{ModeSummaryScope, SystemModesRequest};
use crate::error::Result;
use crate::mode_systems::mode_systems;
use crate::session::Session;
use crate::workspace::{Snapshot, Workspace};
pub use failure::fails;

/// 系別のモード表示（POST /system_modes/{system}）。下書きは保存と同じ Op を書き込まずに当てる。
/// `req.analyzed` が真なら、電源は解析も回してモードごとの電力の行き先・判定・グラフを足す（`req.case` のケースで）。
pub fn system_modes(
    ws: &Workspace,
    system: sat_analysis::spec::System,
    req: &SystemModesRequest,
) -> Result<crate::dto::SystemModesView> {
    let session = Session::open(ws, &req.ops)?;
    let mut view = system_modes_of(session.snapshot(), system)?;
    if req.analyzed && system == sat_analysis::spec::System::Power {
        power::attach(&session, req.case.as_deref(), &mut view.modes)?;
    }
    view.summary = summary_rows(system, req.summary);
    Ok(view)
}

/// 表の足もとの行（系ごとの見せ方はここだけ。web は系の名前や行の見出しで選ばない）。
/// 電源は合計・使用可能電力・余裕（判定は余裕の行）、C&DH と熱は合計 1 行（判定付き）、AOCS は専用のカードなので行なし。
/// `scope` が `Totals` なら、電源は合計の行だけ（ほかの系は元から合計だけなので同じ）。
fn summary_rows(
    system: sat_analysis::spec::System,
    scope: ModeSummaryScope,
) -> Vec<crate::dto::ModeSummaryRow> {
    use crate::dto::{ModeSummaryField as F, ModeSummaryLabel as L, ModeSummaryRow};
    use sat_analysis::spec::System;
    let row = |label, field, verdict| ModeSummaryRow {
        label,
        field,
        verdict,
    };
    match system {
        System::Power if scope == ModeSummaryScope::Totals => vec![row(L::Power, F::Total, false)],
        System::Power => vec![
            row(L::Power, F::Total, false),
            row(L::Available, F::Available, false),
            row(L::Margin, F::Margin, true),
        ],
        System::Cdh => vec![row(L::DataRate, F::Total, true)],
        System::Thermal => vec![row(L::Heat, F::Total, true)],
        _ => Vec::new(),
    }
}

fn system_modes_of(
    s: &Snapshot,
    system: sat_analysis::spec::System,
) -> Result<crate::dto::SystemModesView> {
    use crate::dto::{SystemModeView, SystemModesView};
    use sat_analysis::spec::System;
    if !mode_systems().contains(&system) {
        return Err(crate::error::ServiceError::InvalidInput(
            "This subsystem has no mode view".into(),
        ));
    }
    if system == System::Aocs {
        return Ok(SystemModesView {
            system,
            modes: aocs::views(s)?,
            missing: Vec::new(),
            summary: summary_rows(system, ModeSummaryScope::All),
        });
    }
    let ctx = fune_projection::Context::bare(&s.design, &s.resolved);
    let mut modes = Vec::new();
    let mut missing = fune_projection::MissingInputs::new();
    if system == System::Power {
        let input = fune_projection::power::mode_load_input(&ctx, &mut missing);
        let out = sat_analysis::catalog::run_typed::<sat_analysis::power::ModeLoad>(&input)
            .map_err(mode_analysis_error)?;
        for mode in out.modes {
            let mut sunlit = power_mode_values(&s.design, &mode, false)?;
            let mut eclipse = power_mode_values(&s.design, &mode, true)?;
            for part in &input.parts {
                sunlit.parts.entry(part.id.clone()).or_insert(None);
                eclipse.parts.entry(part.id.clone()).or_insert(None);
            }
            modes.push(SystemModeView {
                id: mode.id.clone(),
                sunlit,
                eclipse,
                aocs: None,
                power: None,
            });
        }
    } else {
        modes = matrix_modes(&ctx, system)?;
    }
    Ok(SystemModesView {
        system,
        modes,
        missing: crate::fixes::complete(&s.design, &s.resolved, missing.into_vec()),
        summary: summary_rows(system, ModeSummaryScope::All),
    })
}

fn matrix_modes(
    ctx: &fune_projection::Context<'_>,
    system: sat_analysis::spec::System,
) -> Result<Vec<crate::dto::SystemModeView>> {
    use crate::dto::SystemModeView;
    use sat_analysis::spec::System;
    let mut modes = Vec::new();
    for mode in ctx.modes() {
        let (sunlit, eclipse) = match system {
            System::Cdh => (
                data_mode_values(ctx, &mode)?,
                data_mode_values(ctx, &mode.in_eclipse())?,
            ),
            System::Thermal => (
                heat_mode_values(ctx, &mode.id, false)?,
                heat_mode_values(ctx, &mode.id, true)?,
            ),
            _ => (Default::default(), Default::default()),
        };
        modes.push(SystemModeView {
            id: mode.id.clone(),
            sunlit,
            eclipse,
            aocs: None,
            power: None,
        });
    }
    Ok(modes)
}

fn mode_analysis_error(e: sat_analysis::spec::AnalysisError) -> crate::error::ServiceError {
    crate::error::ServiceError::Analysis(e.to_string())
}

/// 共通のモード値。集計と欠損の判定はサービスで済ませ、Web に渡す。
fn mode_values(
    parts: impl IntoIterator<Item = (String, Option<f64>)>,
    dimension: &str,
    checks: &[sat_analysis::spec::Check],
) -> crate::dto::ModeValuesView {
    use sat_analysis::spec::Verdict;
    let parts: std::collections::BTreeMap<_, _> = parts.into_iter().collect();
    let total = parts.values().flatten().copied().reduce(|a, b| a + b);
    let incomplete = parts.is_empty() || parts.values().any(Option::is_none);
    let verdict = Verdict::worst(checks.iter().map(|c| c.verdict).chain([if incomplete {
        Verdict::Warning
    } else {
        Verdict::Ok
    }]));
    let q = |v| crate::units::quantity_auto(v, dimension);
    crate::dto::ModeValuesView {
        parts: parts
            .into_iter()
            .map(|(id, value)| (id, value.map(q)))
            .collect(),
        total: total.map(q),
        verdict,
        ..Default::default()
    }
}

fn power_mode_values(
    design: &fune_model::design::Design,
    mode: &sat_analysis::power::mode_load::ModeLoadMode,
    eclipse: bool,
) -> Result<crate::dto::ModeValuesView> {
    use sat_units::typed::{NamedKind, Power};
    let parts = mode.parts.iter().map(|p| {
        (
            p.part.clone(),
            if eclipse { p.mean_eclipse } else { p.mean }.map(|v| v.value()),
        )
    });
    let mut view = mode_values(parts, Power::name(), &mode.checks);
    let result = crate::budget::mode_rollup(design, mode, eclipse)?;
    let q = |v| crate::units::quantity_auto(v, Power::name());
    if !result.rows.is_empty() {
        view.total = Some(q(result.total_with_growth));
    }
    view.available = result.allowed.map(q);
    view.margin = view.total.as_ref().and(result.margin.map(q));
    view.verdict =
        sat_analysis::spec::Verdict::worst(view.verdict.into_iter().chain(result.verdict));
    Ok(view)
}

/// HK とミッションの発生レート（圧縮・パケット化前）。既存の射影と台数評価を使う。
fn data_mode_values(
    ctx: &fune_projection::Context<'_>,
    mode: &sat_analysis::modes::ModeDef,
) -> Result<crate::dto::ModeValuesView> {
    use sat_units::typed::{DataRate, NamedKind};
    let parts = fune_projection::cdh::data_parts(ctx);
    let (rates, checks) = sat_analysis::cdh::data_budget::mode_part_rates(mode, &parts)
        .map_err(mode_analysis_error)?;
    let values = parts.iter().zip(rates).map(|(p, rates)| {
        (
            p.id.clone(),
            rates[0].zip(rates[1]).map(|(hk, mission)| hk + mission),
        )
    });
    Ok(mode_values(values, DataRate::name(), &checks))
}

fn heat_mode_values(
    ctx: &fune_projection::Context<'_>,
    mode: &str,
    eclipse: bool,
) -> Result<crate::dto::ModeValuesView> {
    use sat_units::typed::{NamedKind, Power};
    let Some(mut input) = ctx
        .design
        .analysis
        .thermal
        .as_ref()
        .and_then(|s| fune_projection::thermal::loads(ctx, s, Some(mode)))
    else {
        return Ok(mode_values([], Power::name(), &[]));
    };
    if eclipse {
        input.mode = input.mode.in_eclipse();
    }
    let (parts, checks) = input.evaluate_parts().map_err(mode_analysis_error)?;
    Ok(mode_values(
        parts
            .into_iter()
            .map(|(id, heat)| (id, heat.map(|v| v.value()))),
        Power::name(),
        &checks,
    ))
}

/// 系の ID を既存の model と analysis の対応で読む。
pub fn parse_system(text: &str) -> Result<sat_analysis::spec::System> {
    let system = text
        .parse()
        .map_err(|e: fune_model::enum_text::EnumTextError| {
            crate::error::ServiceError::InvalidInput(e.to_string())
        })?;
    Ok(fune_projection::system::to_analysis(system))
}

#[cfg(test)]
mod aocs_tests;
#[cfg(test)]
mod tests;
