//! 電力のバジェット。値を計算する解析は `power.mode_load` 1 回で、全体の行もモード別の行もその出力から作る。
//! **モード別の合計・余裕・判定の集計は [`mode_rollup`] だけ**（バジェットのモード別の行も、系のモード表
//! `system_modes` の電源も使う）。成長余裕は解析が部品ごとに適用した値（`ModeLoadPart::growth_margin`。成熟度の引き当ては
//! 射影の `parts::growth`、書かれていなければ解析の既定値）で、集計と判定は `sat_analysis::budget::budget_rollup`。

use super::{Uncounted, allocated, analysis_error, rollup, verdict};
use crate::dto::{BudgetKind, BudgetRow, FillRatio};
use crate::error::Result;
use crate::units::quantity_auto;
use fune_model::design::Design;
use fune_projection::Context;
use sat_analysis::budget::{BudgetItem, BudgetRollup};
use sat_analysis::power::mode_load::{ModeLoadMode, ModeLoadOutput};

/// 電力の解析（power.mode_load）。全体の行もモード別の行もこの 1 回の出力から作る。
pub(super) fn mode_load(ctx: &Context<'_>) -> Result<ModeLoadOutput, Uncounted> {
    let input = fune_projection::power::mode_load(ctx)?;
    Ok(
        sat_analysis::catalog::run_typed::<sat_analysis::power::ModeLoad>(&input)
            .map_err(analysis_error)?,
    )
}

/// 1 つのモードの、部品ごとの平均消費電力（員数込み。日照中 `eclipse = false`、蝕中 `true`）。
/// 平均が定まらない部品は含めない。成長余裕は解析が適用した値。
pub(crate) fn mode_items(mode: &ModeLoadMode, eclipse: bool) -> Vec<BudgetItem> {
    mode.parts
        .iter()
        .filter_map(|p| {
            let value = if eclipse { p.mean_eclipse } else { p.mean }?;
            let mut item = BudgetItem::counted(p.part.clone(), value.value(), 1);
            item.growth = p.growth_margin.value();
            Some(item)
        })
        .collect()
}

/// 1 つのモードの電力の集計と判定。上限とシステム余裕は電力の配分（`budget/allocation.toml`）。
pub(crate) fn mode_rollup(
    design: &Design,
    mode: &ModeLoadMode,
    eclipse: bool,
) -> Result<BudgetRollup> {
    let (limit, system_margin) = allocated(design, BudgetKind::Power);
    rollup(&mode_items(mode, eclipse), limit, system_margin)
}

/// 電力のバジェットで数えるモード: ケースのモード（`case_mode`。ケースが無いかモードを持たなければ、
/// 成長余裕込みの合計が最大のモード）。**モードの選び方はここだけ**（KPI の `mode_load` も使う）。
pub(crate) fn budget_mode<'a>(
    out: &'a ModeLoadOutput,
    case_mode: Option<&str>,
) -> Option<&'a ModeLoadMode> {
    out.modes
        .iter()
        .filter(|m| case_mode.is_none_or(|w| m.id == w))
        .max_by(|a, b| {
            a.total_with_growth
                .value()
                .total_cmp(&b.total_with_growth.value())
        })
}

/// 電力: バジェットのモード（`budget_mode`）の部品。
pub(super) fn items(ctx: &Context<'_>, out: &ModeLoadOutput) -> (Vec<BudgetItem>, Option<String>) {
    let wanted = ctx.case_mode().map(|m| m.id.to_string());
    match budget_mode(out, wanted.as_deref()) {
        Some(mode) => (mode_items(mode, false), Some(mode.id.clone())),
        None => (Vec::new(), None),
    }
}

/// 電力のモードごとの合計・使用可能量・余裕（日照中の平均）。集計と判定は [`mode_rollup`]。
pub(super) fn modes(design: &Design, out: &ModeLoadOutput) -> Result<Vec<BudgetRow>> {
    let q = |v: f64| quantity_auto(v, BudgetKind::Power.dimension());
    out.modes
        .iter()
        .map(|mode| {
            let r = mode_rollup(design, mode, false)?;
            Ok(BudgetRow {
                node: Some(mode.id.clone()),
                label: Some(mode.label.clone()),
                label_en: None,
                system: None,
                current: q(r.total),
                with_growth: q(r.total_with_growth),
                allocated: r.allowed.map(q),
                margin: r.margin.map(q),
                verdict: verdict(&r),
                fill: FillRatio::used_over_limit(r.total_with_growth, r.allowed),
            })
        })
        .collect()
}
