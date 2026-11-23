//! 質量・電力・データ量のバジェット。**集計と判定は `sat_analysis::budget::budget_rollup` だけが行う。**
//! ここは部品や解析の結果を `BudgetItem`（1 個あたりの値・員数・成長余裕）に並べ、配分（`budget/allocation.toml`）の行に割り当てるだけ。
//! 成長余裕は `fune_projection::parts::growth`、表示名は `fune_model::builtin::enum_label("budget_kind", …)`。
//! 入力が足りなければ、解析（`analysis::run_analysis`）と同じく `missing` に並べて返す（エラーにしない）。
//! 質量の推薬（推進系の解析の積む推薬）と推薬込みの合計は [`propellant`]。

use crate::dto::{BudgetKind, BudgetRequest, BudgetRow, BudgetUncounted, BudgetView, FillRatio};
use crate::error::{Result, ServiceError};
use crate::units::quantity_auto;
use crate::workspace::{Snapshot, Workspace};
use fune_model::design::{Allocation, AnalysisSystem, Design, NodeRef};
use fune_model::id::Id;
use fune_model::value::units::to_si;
use fune_projection::{Context, MissingInputs};
use sat_analysis::budget::{BudgetItem, BudgetRollup, budget_rollup};
use sat_analysis::spec::Verdict;

pub(crate) use power::{budget_mode, mode_rollup};
pub mod failure;
mod power;
mod propellant;
mod uncounted;

/// バジェット（`GET`・`POST /budgets/{kind}`）。`req.ops` があれば書き込まずに当てた設計で、`req.case` があればそのケースで数える。
/// 編集が通らなければ保存と同じエラー（検証なら診断付き）。入力が足りなければ `missing` に並べた表示を返す。
pub fn budget(ws: &Workspace, kind: BudgetKind, req: &BudgetRequest) -> Result<BudgetView> {
    let s = crate::preview::snapshot_with(ws, &req.ops)?;
    let mut view = header(&s.design, kind, req.case.clone());
    match count(&s, kind, req.case.as_deref()) {
        Ok(counted) => fill(&s.design, &mut view, counted)?,
        Err(Uncounted::Missing(m)) => {
            view.missing = crate::fixes::complete(&s.design, &s.resolved, m.into_vec())
        }
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

/// 質量の部品 1 行（報告書の質量表。値は `Counted` の 1 個あたりの質量・員数・成長余裕と、割り当てた配分の行）。
#[derive(Debug, Clone)]
pub(crate) struct PartLine {
    pub part: String,
    /// 配分の行（`BudgetRow.node` と同じ。どれにも属さなければ None）
    pub row: Option<String>,
    pub value: f64,
    pub count: u32,
    pub growth: f64,
}

/// 質量の部品の行と、集計（`budget_rollup`）の合計。入力が足りなければ空（`budget` の `missing` が理由を持つ）。
pub(crate) fn mass_part_lines(s: &Snapshot) -> Result<(Vec<PartLine>, f64)> {
    let ctx = match Context::new(&s.design, &s.resolved, None) {
        Ok(ctx) => ctx,
        Err(_) => return Ok((Vec::new(), 0.0)),
    };
    let items = mass_items(s, &ctx);
    let alloc = s.design.budget.allocation.get(BudgetKind::Mass);
    let (limit, margin) = allocated(&s.design, BudgetKind::Mass);
    let total = rollup(&items, limit, margin)?.total;
    let lines = items
        .iter()
        .map(|i| PartLine {
            part: i.id.clone(),
            row: row_of(&s.design, alloc, &i.id),
            value: i.value,
            count: i.count,
            growth: i.growth,
        })
        .collect();
    Ok((lines, total))
}

/// 数えた結果（配分の行に割り当てる前）。
struct Counted {
    items: Vec<BudgetItem>,
    /// 計算に使ったモード（電力のとき）
    mode: Option<String>,
    /// 電力のモードごとの行
    modes: Vec<BudgetRow>,
    /// 質量の推薬（質量のときだけ）
    propellant: Option<propellant::Load>,
    uncounted: Vec<BudgetUncounted>,
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
    let plain = |items, uncounted| Counted {
        items,
        mode: None,
        modes: Vec::new(),
        propellant: None,
        uncounted,
    };
    Ok(match kind {
        BudgetKind::Mass => Counted {
            propellant: Some(propellant::load(&ctx)),
            ..plain(mass_items(s, &ctx), uncounted::mass(s))
        },
        BudgetKind::Power => {
            let out = power::mode_load(&ctx)?;
            let (items, mode) = power::items(&ctx, &out);
            Counted {
                items,
                mode,
                modes: power::modes(&s.design, &out)?,
                propellant: None,
                uncounted: uncounted::power(s, &out),
            }
        }
        BudgetKind::Data => plain(data_items(s, &ctx)?, Vec::new()),
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

/// 質量: 部品ごとの 1 個あたりの質量と員数。質量はケースの `side` が指す側で読む（`Context::scalar`。電力・データ量と同じ入口）。
fn mass_items(s: &Snapshot, ctx: &Context<'_>) -> Vec<BudgetItem> {
    s.resolved
        .parts
        .values()
        .filter_map(|p| Some(item(s, &p.id, ctx.scalar(p, "mass")?, p.count_or_one())))
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

/// 配分の上限（SI。**上限は要求が持つ**: `verify.budget` がこの種類の要求のうち最も厳しいもの。`Design::budget_limit`）と
/// システム余裕（比。配分が無ければ 0）。
pub(crate) fn allocated(design: &Design, kind: BudgetKind) -> (Option<f64>, f64) {
    let alloc = design.budget.allocation.get(kind);
    (
        design.budget_limit(kind).map(|c| c.si),
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
        notes.push(format!("budget/allocation.toml has no [{key}]"));
    }
    if design.budget_limit(kind).is_none() {
        notes.push(format!(
            "No requirement sets the {key} limit (an at_most requirement with verify.budget = \"{key}\")"
        ));
    }
    BudgetView {
        kind,
        system: crate::query::settings::budget_system(kind),
        label: label.map_or_else(|| key.clone(), |l| l.label.clone()),
        label_en: label.and_then(|l| l.label_en.clone()),
        case,
        limit: limit.map(|v| quantity_auto(v, dim)),
        limit_requirement: design.budget_limit(kind).map(|c| c.requirement),
        system_margin: crate::units::percent(system_margin),
        total: None,
        margin: None,
        verdict: None,
        fill: None,
        rows: Vec::new(),
        modes: Vec::new(),
        propellant: None,
        analysis: analysis_of(kind).to_owned(),
        mode: None,
        missing: Vec::new(),
        uncounted: Vec::new(),
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
    view.propellant = counted
        .propellant
        .map(|load| propellant::view(load, &counted.items, limit, system_margin))
        .transpose()?;
    view.modes = counted.modes;
    view.uncounted = counted.uncounted;
    view.mode = counted.mode;
    Ok(())
}
