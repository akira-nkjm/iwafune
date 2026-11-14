//! 系ごとのモードの見方（電源の消費電力・C&DH のデータの発生量・熱の発熱・AOCS の姿勢則）。
//! 値・合計・判定はここで作り、web は並べるだけ（`POST /system_modes/{system}`）。

mod aocs;
mod attitude;
mod devices;

use crate::dto::PartViewRequest;
use crate::error::Result;
use crate::workspace::{Snapshot, Workspace};

/// 系別のモード表示（POST /system_modes/{system}）。下書きは保存と同じ Op を書き込まずに当てる。
pub fn system_modes(
    ws: &Workspace,
    system: sat_analysis::spec::System,
    req: &PartViewRequest,
) -> Result<crate::dto::SystemModesView> {
    let s = crate::preview::snapshot_with(ws, &req.ops)?;
    system_modes_of(&s, system)
}

fn system_modes_of(
    s: &Snapshot,
    system: sat_analysis::spec::System,
) -> Result<crate::dto::SystemModesView> {
    use crate::dto::{SystemModeView, SystemModesView};
    use sat_analysis::spec::System;
    if system == System::Aocs {
        return Ok(SystemModesView {
            system,
            modes: aocs::views(s)?,
        });
    }
    let ctx = fune_projection::Context::bare(&s.design, &s.resolved);
    let mut modes = Vec::new();
    if !matches!(system, System::Power | System::Cdh | System::Thermal) {
        return Err(crate::error::ServiceError::InvalidInput(
            "この系にはモード表示がありません".into(),
        ));
    }
    if system == System::Power {
        let mut missing = fune_projection::MissingInputs::new();
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
            });
        }
    } else {
        modes = matrix_modes(&ctx, system)?;
    }
    Ok(SystemModesView { system, modes })
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
    let (limit, margin) = crate::budget::allocated(design, crate::dto::BudgetKind::Power);
    let items: Vec<_> = mode
        .parts
        .iter()
        .filter_map(|p| {
            let value = if eclipse { p.mean_eclipse } else { p.mean }?;
            let mut item =
                sat_analysis::budget::BudgetItem::counted(p.part.clone(), value.value(), 1);
            item.growth = p.growth_margin.value();
            Some(item)
        })
        .collect();
    let result =
        sat_analysis::budget::budget_rollup(&items, limit, margin).map_err(mode_analysis_error)?;
    let q = |v| crate::units::quantity_auto(v, Power::name());
    if !items.is_empty() {
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
