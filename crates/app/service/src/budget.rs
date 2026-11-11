//! 質量・電力・データ量のバジェット。**集計と判定は `sat_analysis::budget::budget_rollup` だけが行う。**
//! ここは部品や解析の結果を `BudgetItem`（1 個あたりの値・員数・成長余裕）に並べ、配分（`budget/allocation.toml`）の行に割り当てるだけ。
//! 成長余裕は `fune_projection::parts::growth`、表示名は `fune_model::builtin::enum_label("budget_kind", …)`。
//! 入力が足りなければ、解析（`analysis::run_analysis`）と同じく `missing` に並べて返す（エラーにしない）。

use crate::dto::{BudgetKind, BudgetRequest, BudgetRow, BudgetView, FillRatio};
use crate::error::{Result, ServiceError};
use crate::units::quantity_auto;
use crate::workspace::{Snapshot, Workspace};
use fune_model::design::{Allocation, AnalysisSystem, Design, NodeRef};
use fune_model::id::Id;
use fune_model::value::units::to_si;
use fune_projection::{Context, MissingInputs};
use sat_analysis::budget::{BudgetItem, BudgetRollup, budget_rollup};
use sat_analysis::power::mode_load::{ModeLoadMode, ModeLoadOutput};
use sat_analysis::spec::Verdict;

/// バジェット（`GET`・`POST /budgets/{kind}`）。`req.ops` があれば書き込まずに当てた設計で、`req.case` があればそのケースで数える。
/// 編集が通らなければ保存と同じエラー（検証なら診断付き）。入力が足りなければ `missing` に並べた表示を返す。
pub fn budget(ws: &Workspace, kind: BudgetKind, req: &BudgetRequest) -> Result<BudgetView> {
    let s = crate::preview::snapshot_with(ws, &req.ops)?;
    let mut view = header(&s.design, kind, req.case.clone());
    match count(&s, kind, req.case.as_deref()) {
        Ok(counted) => fill(&s.design, &mut view, counted)?,
        Err(Uncounted::Missing(m)) => view.missing = m.into_vec(),
        Err(Uncounted::Failed(e)) => return Err(e),
    }
    Ok(view)
}

/// `"mass"` などの文字列からバジェットの種類を読む。
pub fn parse_kind(text: &str) -> Result<BudgetKind> {
    text.parse()
        .map_err(|e: fune_model::enum_text::EnumTextError| {
            ServiceError::InvalidInput(e.to_string())
        })
}

/// 数えた結果（配分の行に割り当てる前）。
struct Counted {
    items: Vec<BudgetItem>,
    /// 計算に使ったモード（電力のとき）
    mode: Option<String>,
    /// 電力のモードごとの行
    modes: Vec<BudgetRow>,
}

/// 数えられなかった理由。入力の不足は表示に載せ、解析の誤りはエラーにする。
enum Uncounted {
    Missing(MissingInputs),
    Failed(ServiceError),
}

impl From<MissingInputs> for Uncounted {
    fn from(m: MissingInputs) -> Self {
        Self::Missing(m)
    }
}

impl From<ServiceError> for Uncounted {
    fn from(e: ServiceError) -> Self {
        Self::Failed(e)
    }
}

fn analysis_error(e: sat_analysis::spec::AnalysisError) -> ServiceError {
    ServiceError::Analysis(e.to_string())
}

/// 種類ごとの部品の値と、電力のモードごとの行。
fn count(s: &Snapshot, kind: BudgetKind, case: Option<&str>) -> Result<Counted, Uncounted> {
    let ctx = Context::new(&s.design, &s.resolved, case)?;
    let plain = |items| Counted {
        items,
        mode: None,
        modes: Vec::new(),
    };
    Ok(match kind {
        BudgetKind::Mass => plain(mass_items(s)),
        BudgetKind::Power => {
            let out = mode_load(&ctx)?;
            let (items, mode) = power_items(s, &ctx, &out);
            Counted {
                items,
                mode,
                modes: power_modes(s, &out)?,
            }
        }
        BudgetKind::Data => plain(data_items(s, &ctx)?),
    })
}

/// 値を計算する解析の ID。
fn analysis_of(kind: BudgetKind) -> &'static str {
    match kind {
        BudgetKind::Mass => "budget",
        BudgetKind::Power => "power.mode_load",
        BudgetKind::Data => "cdh.data_budget",
    }
}

/// 部品 → 配分の行。部品・組立品の明示配分を優先し、無ければ部品の系（未所属は「その他」）。
fn row_of(design: &Design, alloc: Option<&Allocation>, part: &str) -> Option<String> {
    let allocated = |id: &str| alloc.is_some_and(|a| a.alloc.contains_key(id));
    if allocated(part) {
        return Some(part.to_owned());
    }
    let ancestors = design.ancestors(&NodeRef::Part(Id::new(part)));
    ancestors
        .iter()
        .find(|n| matches!(n, NodeRef::Part(_)) && allocated(n.id()))
        .map(|n| n.id().to_owned())
        .or_else(|| design.system_of(part).map(|s| s.to_string()))
}

/// 1 部品の値。配分の行は `rows` が `row_of` で決める（`budget_rollup` の `by_group` は使わない）。
fn item(s: &Snapshot, part: &str, value: f64, count: u32) -> BudgetItem {
    let growth = s
        .resolved
        .parts
        .get(part)
        .and_then(|p| fune_projection::parts::growth(&s.design, p))
        .unwrap_or(0.0);
    BudgetItem {
        id: part.to_owned(),
        group: String::new(),
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
fn mode_load(ctx: &Context<'_>) -> Result<ModeLoadOutput, Uncounted> {
    let input = fune_projection::power::mode_load(ctx)?;
    Ok(
        sat_analysis::catalog::run_typed::<sat_analysis::power::ModeLoad>(&input)
            .map_err(analysis_error)?,
    )
}

/// 1 つのモードの、部品ごとの平均消費電力（員数込み）。成長余裕と配分の行は質量と同じ `item` で決める。
fn mode_items(s: &Snapshot, mode: &ModeLoadMode) -> Vec<BudgetItem> {
    mode.parts
        .iter()
        .filter_map(|p| Some(item(s, &p.part, p.mean?.value(), 1)))
        .collect()
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
fn power_items(
    s: &Snapshot,
    ctx: &Context<'_>,
    out: &ModeLoadOutput,
) -> (Vec<BudgetItem>, Option<String>) {
    let wanted = ctx.case_mode().map(|m| m.id.to_string());
    match budget_mode(out, wanted.as_deref()) {
        Some(mode) => (mode_items(s, mode), Some(mode.id.clone())),
        None => (Vec::new(), None),
    }
}

/// 電力のモードごとの合計・使用可能量・余裕。上限とシステム余裕は全体と同じ配分を使い、
/// 集計と判定も全体と同じ `budget_rollup`・`verdict` で行う。
fn power_modes(s: &Snapshot, out: &ModeLoadOutput) -> Result<Vec<BudgetRow>> {
    let q = |v: f64| quantity_auto(v, BudgetKind::Power.dimension());
    let (limit, system_margin) = allocated(&s.design, BudgetKind::Power);
    out.modes
        .iter()
        .map(|mode| {
            let r = rollup(&mode_items(s, mode), limit, system_margin)?;
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

/// データ量: 部品ごとの 1 日の発生量（HK＋ミッション、員数込み）。
fn data_items(s: &Snapshot, ctx: &Context<'_>) -> Result<Vec<BudgetItem>, Uncounted> {
    let input = fune_projection::cdh::data_budget(ctx)?;
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

/// 配分の上限（SI）とシステム余裕（比。配分が無ければ 0）。
fn allocated(design: &Design, kind: BudgetKind) -> (Option<f64>, f64) {
    let alloc = design.budget.allocation.get(kind);
    (
        alloc.and_then(|a| a.limit.as_ref()).and_then(si),
        alloc.map_or(0.0, |a| a.system_margin),
    )
}

fn rollup(items: &[BudgetItem], limit: Option<f64>, margin: f64) -> Result<BudgetRollup> {
    budget_rollup(items, limit, margin).map_err(analysis_error)
}

fn verdict(r: &BudgetRollup) -> Verdict {
    r.verdict.unwrap_or(Verdict::Ok)
}

/// 配分の行（配分のキーの順、そのあとに配分の無い行、最後に「その他」）。行ごとの判定も `budget_rollup` で行う。
fn rows(
    design: &Design,
    alloc: Option<&Allocation>,
    items: &[BudgetItem],
    dim: &str,
) -> Result<Vec<BudgetRow>> {
    let q = |v: f64| quantity_auto(v, dim);
    let placed: Vec<(Option<String>, &BudgetItem)> = items
        .iter()
        .map(|i| (row_of(design, alloc, &i.id), i))
        .collect();
    let mut keys: Vec<Option<String>> = alloc
        .map(|a| a.alloc.keys().cloned().map(Some).collect())
        .unwrap_or_default();
    for (key, _) in &placed {
        if !keys.contains(key) {
            keys.push(key.clone());
        }
    }
    keys.sort_by_key(Option::is_none);
    let mut out = Vec::new();
    for key in keys {
        let group: Vec<BudgetItem> = placed
            .iter()
            .filter(|(k, _)| *k == key)
            .map(|(_, i)| (*i).clone())
            .collect();
        let allocated = key
            .as_ref()
            .and_then(|k| alloc.and_then(|a| a.alloc.get(k)))
            .and_then(si);
        let r = rollup(&group, allocated, 0.0)?;
        out.push(BudgetRow {
            label: key.as_ref().map(|k| {
                k.parse::<AnalysisSystem>()
                    .ok()
                    .and_then(|s| fune_model::builtin::labels().section.get(s.as_str()))
                    .map(|l| l.label.clone())
                    .or_else(|| design.parts.get(k.as_str()).and_then(|p| p.label.clone()))
                    .unwrap_or_else(|| k.clone())
            }),
            label_en: key
                .as_ref()
                .and_then(|k| k.parse::<AnalysisSystem>().ok())
                .and_then(|s| fune_model::builtin::labels().section.get(s.as_str()))
                .and_then(|l| l.label_en.clone()),
            system: key
                .as_ref()
                .and_then(|k| {
                    k.parse::<AnalysisSystem>()
                        .ok()
                        .or_else(|| design.system_of(k))
                })
                .map(fune_projection::system::to_analysis),
            node: key,
            current: q(r.total),
            with_growth: q(r.total_with_growth),
            allocated: allocated.map(q),
            margin: r.margin.map(q),
            verdict: verdict(&r),
            fill: FillRatio::used_over_limit(r.total_with_growth, allocated),
        });
    }
    Ok(out)
}

/// 数える前に決まる欄（見出し・上限・システム余裕・配分の注記）。合計と行は `fill` が入れる。
fn header(design: &Design, kind: BudgetKind, case: Option<String>) -> BudgetView {
    let dim = kind.dimension();
    let (limit, system_margin) = allocated(design, kind);
    let key = kind.to_string();
    let label = fune_model::builtin::enum_label("budget_kind", &key);
    let mut notes = Vec::new();
    if design.budget.allocation.get(kind).is_none() {
        notes.push(format!("budget/allocation.toml に [{key}] がありません"));
    }
    BudgetView {
        kind,
        system: crate::query::settings::budget_system(kind),
        label: label.map_or_else(|| key.clone(), |l| l.label.clone()),
        label_en: label.and_then(|l| l.label_en.clone()),
        case,
        limit: limit.map(|v| quantity_auto(v, dim)),
        system_margin: crate::units::percent(system_margin),
        total: None,
        margin: None,
        verdict: None,
        fill: None,
        rows: Vec::new(),
        modes: Vec::new(),
        analysis: analysis_of(kind).to_owned(),
        mode: None,
        missing: Vec::new(),
        notes,
    }
}

/// 数えた結果から合計・余裕・判定・行を入れる。
fn fill(design: &Design, view: &mut BudgetView, counted: Counted) -> Result<()> {
    let dim = view.kind.dimension();
    let q = |v: f64| quantity_auto(v, dim);
    let (limit, system_margin) = allocated(design, view.kind);
    let total = rollup(&counted.items, limit, system_margin)?;
    view.total = Some(q(total.total_with_growth));
    view.margin = total.margin.map(q);
    view.verdict = total.verdict;
    view.fill = FillRatio::used_over_limit(total.total_with_growth, limit);
    view.rows = rows(
        design,
        design.budget.allocation.get(view.kind),
        &counted.items,
        dim,
    )?;
    view.modes = counted.modes;
    view.mode = counted.mode;
    Ok(())
}
