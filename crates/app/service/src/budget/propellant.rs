//! 質量のバジェットの推薬（`BudgetView.propellant`）。
//!
//! **推薬の値は推進系の解析 `propulsion.propellant` の積む推薬（`loaded_propellant`）だけから取る**（ここで推薬を計算しない）。
//! 推進系が無ければ数えず（`fune_projection::propulsion::present`）、入力が足りない・解析が通らなければ理由を返す。
//! 推薬込み（wet）の合計と判定は、乾燥の部品に成長余裕 0 の 1 行を足して、乾燥と同じ `budget_rollup` で出す。

use super::rollup;
use crate::dto::{BudgetKind, BudgetPropellant, BudgetTotal, FillRatio};
use crate::error::Result;
use crate::units::quantity_auto;
use fune_projection::Context;
use sat_analysis::budget::BudgetItem;
use sat_analysis::propulsion::Propellant;
use sat_analysis::spec::Analysis;

/// 推進系の解析から読んだ推薬（合計に足す前）。
pub(super) enum Load {
    /// 積む推薬 [kg]
    Loaded(f64),
    /// 数えない理由
    Omitted(BudgetPropellant),
}

/// 推進系の解析を回して、積む推薬を読む。
pub(super) fn load(ctx: &Context<'_>) -> Load {
    if !fune_projection::propulsion::present(ctx) {
        return Load::Omitted(BudgetPropellant::NoPropulsion);
    }
    let input = match fune_projection::propulsion::maneuvers(ctx) {
        Ok(input) => input,
        Err(m) => {
            return Load::Omitted(BudgetPropellant::Missing {
                missing: m.into_vec(),
            });
        }
    };
    match sat_analysis::catalog::run_typed::<Propellant>(&input) {
        Ok(out) => Load::Loaded(out.loaded_propellant.value()),
        Err(e) => Load::Omitted(BudgetPropellant::Failed {
            message: e.to_string(),
        }),
    }
}

/// 推薬の表示。数えたときは乾燥の部品 `dry` に推薬の 1 行を足し、全体と同じ上限・システム余裕で集計と判定をする。
pub(super) fn view(
    load: Load,
    dry: &[BudgetItem],
    limit: Option<f64>,
    system_margin: f64,
) -> Result<BudgetPropellant> {
    let kg = match load {
        Load::Loaded(kg) => kg,
        Load::Omitted(v) => return Ok(v),
    };
    let q = |v: f64| quantity_auto(v, BudgetKind::Mass.dimension());
    let mut items = dry.to_vec();
    items.push(BudgetItem {
        id: Propellant::ID.to_owned(),
        group: String::new(),
        value: kg,
        count: 1,
        growth: 0.0,
    });
    let r = rollup(&items, limit, system_margin)?;
    Ok(BudgetPropellant::Counted {
        analysis: Propellant::ID.to_owned(),
        loaded: q(kg),
        wet: Box::new(BudgetTotal {
            total: q(r.total_with_growth),
            margin: r.margin.map(q),
            verdict: r.verdict,
            fill: FillRatio::used_over_limit(r.total_with_growth, limit),
        }),
    })
}
