//! 質量・電力・データ量のバジェット。**集計と判定は `sat_analysis::budget::budget_rollup` だけが行う。**
//! ここは部品や解析の結果を `BudgetItem`（1 個あたりの値・員数・成長余裕）に並べ、配分（`budget/allocation.toml`）の行に割り当てるだけ。
//! 成長余裕は `fune_projection::parts::growth`、表示名は `fune_model::builtin::enum_label("budget_kind", …)`。

use crate::dto::{BudgetKind, BudgetRequest, BudgetRow, BudgetView};
use crate::error::{Result, ServiceError};
use crate::units::quantity_auto;
use crate::workspace::{Snapshot, Workspace};
use fune_model::design::{Allocation, Design, NodeRef};
use fune_model::id::Id;
use fune_model::value::units::to_si;
use fune_projection::{Context, ProjectionError};
use sat_analysis::budget::{BudgetItem, BudgetRollup, budget_rollup};
use sat_analysis::power::mode_load::{ModeLoadMode, ModeLoadOutput};
use sat_analysis::spec::Verdict;

/// バジェット（`GET`・`POST /budgets/{kind}`）。`req.ops` があれば、書き込まずに当てた設計で数える。
pub fn budget(ws: &Workspace, kind: BudgetKind, req: &BudgetRequest) -> Result<BudgetView> {
    let s = crate::preview::snapshot_with(ws, &req.ops)?;
    let ctx = Context::new(&s.design, &s.resolved, None).map_err(missing)?;
    let mut modes = Vec::new();
    let (items, analysis, mode) = match kind {
        BudgetKind::Mass => (mass_items(&s), "budget", None),
        BudgetKind::Power => {
            let out = mode_load(&ctx)?;
            modes = power_modes(&s, &out)?;
            let (items, mode) = power_items(&s, &ctx, &out);
            (items, "power.mode_load", mode)
        }
        BudgetKind::Data => (data_items(&s, &ctx)?, "cdh.data_budget", None),
    };
    let mut result = view(&s.design, kind, &items, analysis, mode)?;
    result.modes = modes;
    Ok(result)
}

/// `"mass"` などの文字列からバジェットの種類を読む。
pub fn parse_kind(text: &str) -> Result<BudgetKind> {
    text.parse().map_err(ServiceError::InvalidInput)
}

fn missing(e: impl Into<ProjectionError>) -> ServiceError {
    ServiceError::Analysis(e.into().to_string())
}

fn analysis_error(e: sat_analysis::spec::AnalysisError) -> ServiceError {
    ServiceError::Analysis(e.to_string())
}

/// 部品 → 配分の行（配分のキーになっている最も近い祖先。無ければ最上位の Group）。
fn row_of(design: &Design, part: &str) -> String {
    let all: Vec<&String> = design
        .budget
        .allocation
        .iter()
        .flat_map(|(_, a)| a.alloc.keys())
        .collect();
    if all.iter().any(|k| k.as_str() == part) {
        return part.to_owned();
    }
    let ancestors = design.ancestors(&NodeRef::Part(Id::new(part)));
    ancestors
        .iter()
        .find(|n| all.iter().any(|k| k.as_str() == n.id()))
        .or_else(|| ancestors.last())
        .map_or_else(|| "（その他）".to_owned(), |n| n.id().to_owned())
}

fn item(s: &Snapshot, part: &str, value: f64, count: u32) -> BudgetItem {
    let growth = s
        .resolved
        .parts
        .get(part)
        .and_then(|p| fune_projection::parts::growth(&s.design, p))
        .unwrap_or(0.0);
    BudgetItem {
        id: part.to_owned(),
        group: row_of(&s.design, part),
        value,
        count,
        growth,
    }
}

/// 質量: 部品ごとの 1 個あたりの質量と員数。
fn mass_items(s: &Snapshot) -> Vec<BudgetItem> {
    s.resolved
        .parts
        .values()
        .filter_map(|p| {
            Some(item(
                s,
                &p.id,
                p.scalar("mass")?,
                fune_projection::parts::count_of(p),
            ))
        })
        .collect()
}

/// 電力の解析（power.mode_load）。全体の行もモード別の行もこの 1 回の出力から作る。
fn mode_load(ctx: &Context<'_>) -> Result<ModeLoadOutput> {
    let input = fune_projection::power::mode_load(ctx).map_err(missing)?;
    sat_analysis::catalog::run_typed::<sat_analysis::power::ModeLoad>(&input)
        .map_err(analysis_error)
}

/// 1 つのモードの、部品ごとの平均消費電力（員数込み）。成長余裕と配分の行は質量と同じ `item` で決める。
fn mode_items(s: &Snapshot, mode: &ModeLoadMode) -> Vec<BudgetItem> {
    mode.parts
        .iter()
        .filter_map(|p| Some(item(s, &p.part, p.mean?.value(), 1)))
        .collect()
}

/// 電力: ケースのモード（無ければ成長余裕込みの合計が最大のモード）の部品。
fn power_items(
    s: &Snapshot,
    ctx: &Context<'_>,
    out: &ModeLoadOutput,
) -> (Vec<BudgetItem>, Option<String>) {
    let wanted = ctx.case_mode().map(|m| m.id.to_string());
    let worst = out
        .modes
        .iter()
        .filter(|m| wanted.as_ref().is_none_or(|w| &m.id == w))
        .max_by(|a, b| {
            a.total_with_growth
                .value()
                .total_cmp(&b.total_with_growth.value())
        });
    match worst {
        Some(mode) => (mode_items(s, mode), Some(mode.id.clone())),
        None => (Vec::new(), None),
    }
}

/// 電力のモードごとの合計・使用可能量・余裕。上限とシステム余裕は全体と同じ配分を使い、
/// 集計と判定も全体と同じ `budget_rollup`・`verdict` で行う。
fn power_modes(s: &Snapshot, out: &ModeLoadOutput) -> Result<Vec<BudgetRow>> {
    let q = |v: f64| quantity_auto(v, BudgetKind::Power.dimension());
    let alloc = s.design.budget.allocation.get(BudgetKind::Power);
    let limit = alloc.and_then(|a| si(&a.limit));
    let system_margin = alloc.map_or(0.0, |a| a.system_margin);
    out.modes
        .iter()
        .map(|mode| {
            let r = rollup(&mode_items(s, mode), limit, system_margin)?;
            Ok(BudgetRow {
                node: mode.id.clone(),
                label: mode.label.clone(),
                current: q(r.total),
                with_growth: q(r.total_with_growth),
                allocated: r.allowed.map(q),
                margin: r.margin.map(q),
                verdict: verdict(&r),
            })
        })
        .collect()
}

/// データ量: 部品ごとの 1 日の発生量（HK＋ミッション、員数込み）。
fn data_items(s: &Snapshot, ctx: &Context<'_>) -> Result<Vec<BudgetItem>> {
    let input = fune_projection::cdh::data_budget(ctx).map_err(missing)?;
    let out = sat_analysis::catalog::run_typed::<sat_analysis::cdh::DataBudget>(&input)
        .map_err(analysis_error)?;
    Ok(out
        .parts
        .iter()
        .map(|p| {
            let mut it = item(
                s,
                &p.part,
                p.hk_per_day.value() + p.mission_per_day.value(),
                1,
            );
            it.growth = 0.0;
            it
        })
        .collect())
}

fn si(x: &fune_model::value::Quantity) -> Option<f64> {
    to_si(x.value, &x.unit).ok()
}

fn rollup(items: &[BudgetItem], limit: Option<f64>, margin: f64) -> Result<BudgetRollup> {
    budget_rollup(items, limit, margin).map_err(analysis_error)
}

fn verdict(r: &BudgetRollup) -> Verdict {
    r.verdict.unwrap_or(Verdict::Ok)
}

/// 配分の行（配分のキーの順、そのあとに配分の無い行）。行ごとの判定も `budget_rollup` で行う。
fn rows(
    design: &Design,
    alloc: Option<&Allocation>,
    items: &[BudgetItem],
    dim: &str,
) -> Result<Vec<BudgetRow>> {
    let q = |v: f64| quantity_auto(v, dim);
    let mut keys: Vec<String> = alloc
        .map(|a| a.alloc.keys().cloned().collect())
        .unwrap_or_default();
    let extra: Vec<String> = items
        .iter()
        .map(|i| i.group.clone())
        .filter(|g| !keys.contains(g))
        .collect();
    keys.extend(extra);
    keys.dedup();
    let mut out = Vec::new();
    for key in keys {
        let group: Vec<BudgetItem> = items.iter().filter(|i| i.group == key).cloned().collect();
        let allocated = alloc.and_then(|a| a.alloc.get(&key)).and_then(si);
        let r = rollup(&group, allocated, 0.0)?;
        out.push(BudgetRow {
            label: design
                .groups
                .get(key.as_str())
                .and_then(|g| g.label.clone())
                .unwrap_or_else(|| key.clone()),
            node: key,
            current: q(r.total),
            with_growth: q(r.total_with_growth),
            allocated: allocated.map(q),
            margin: r.margin.map(q),
            verdict: verdict(&r),
        });
    }
    Ok(out)
}

fn view(
    design: &Design,
    kind: BudgetKind,
    items: &[BudgetItem],
    analysis: &str,
    mode: Option<String>,
) -> Result<BudgetView> {
    let dim = kind.dimension();
    let q = |v: f64| quantity_auto(v, dim);
    let alloc = design.budget.allocation.get(kind);
    let limit = alloc.and_then(|a| si(&a.limit));
    let system_margin = alloc.map_or(0.0, |a| a.system_margin);
    let total = rollup(items, limit, system_margin)?;
    let key = kind.to_string();
    let label = fune_model::builtin::enum_label("budget_kind", &key);
    let mut notes = Vec::new();
    if alloc.is_none() {
        notes.push(format!("budget/allocation.toml に [{key}] がありません"));
    }
    Ok(BudgetView {
        kind,
        label: label.map_or_else(|| key.clone(), |l| l.label.clone()),
        label_en: label.and_then(|l| l.label_en.clone()),
        limit: limit.map(q),
        system_margin,
        system_margin_display: crate::units::percent(system_margin),
        total: q(total.total_with_growth),
        margin: total.margin.map(q),
        verdict: verdict(&total),
        rows: rows(design, alloc, items, dim)?,
        modes: Vec::new(),
        analysis: analysis.to_owned(),
        mode,
        notes,
    })
}
