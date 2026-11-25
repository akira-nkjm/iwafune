//! 文章から 1 つの値を引くための項目（Typst の `#value("…")`）。キーは構造で決める。
//! `budget.mass.{total,limit,margin}`・`part.<ID>.mass`（1 個あたり）・`kpi.<解析 ID>.<KPI>[@<ケース>]`。

use super::common::{pick_label, value_entry};
use super::{Ctx, Out};
use crate::budget::PartLine;
use crate::dto::{AnalysisRunResult, BudgetView, QuantityView};
use crate::units::quantity_auto;
use fune_report::labels::{Text, t};

/// 質量バジェットの合計・上限・余裕と、部品ごとの 1 個あたりの質量。
pub(super) fn mass(
    ctx: &Ctx,
    out: &mut Out,
    view: &BudgetView,
    total: &QuantityView,
    lines: &[PartLine],
) {
    let name = |text: Text| t(ctx.lang, text).to_owned();
    out.values.push(value_entry(
        "budget.mass.total".into(),
        name(Text::ValueMassTotal),
        total,
    ));
    for (key, q, text) in [
        ("budget.mass.limit", &view.limit, Text::ValueMassLimit),
        ("budget.mass.margin", &view.margin, Text::ValueMassMargin),
    ] {
        if let Some(q) = q {
            out.values.push(value_entry(key.into(), name(text), q));
        }
    }
    for l in lines {
        let q = quantity_auto(l.value, "Mass");
        let label = super::common::part_name(ctx, &l.part);
        out.values
            .push(value_entry(format!("part.{}.mass", l.part), label, &q));
    }
}

/// 解析の KPI（値のあるもの）。ケースがあれば `@<ケース>` を付ける。
pub(super) fn kpis(ctx: &Ctx, out: &mut Out, r: &AnalysisRunResult, case: Option<&str>) {
    for k in &r.kpis {
        let Some(q) = &k.value else { continue };
        let Ok(serde_json::Value::String(key)) = serde_json::to_value(k.key) else {
            continue;
        };
        let at = case.map(|c| format!("@{c}")).unwrap_or_default();
        let label = pick_label(ctx.lang, &k.label, Some(&k.label_en));
        out.values
            .push(value_entry(format!("kpi.{}.{key}{at}", r.id), label, q));
    }
}
