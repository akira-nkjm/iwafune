//! C&DH の射影: `cdh.data_budget`。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::parts::{by_role, state_values};
use crate::quantity::setting;
use fune_model::Target;
use fune_model::builtin::Role;
use fune_model::design::AnalysisSystem;
use fune_model::design::setting_dims as dims;
use sat_analysis::cdh::DataBudgetInput;
use sat_analysis::cdh::data_budget::{DataBudgetModeShare, DataBudgetPart, DataBudgetStorage};
use sat_units::typed::{Fraction, Qty};

/// `cdh.data_budget`
pub fn data_budget(ctx: &Context<'_>) -> Result<DataBudgetInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let parts = data_parts(ctx);
    if parts.is_empty() {
        m.push(
            Target::any_part("hk_rate"),
            "No part with an HK or mission data rate",
        );
    }
    let mode_shares = mode_shares(ctx, &mut m);
    // 容量の合計（員数込み）は解析が求めるので、1 台あたりの容量と員数をそのまま渡す。
    let storage: Vec<DataBudgetStorage> = by_role(ctx, Role::Obc)
        .filter_map(|p| {
            Some(DataBudgetStorage {
                part: p.id.clone(),
                capacity: Qty::from_raw(ctx.scalar(p, "storage_capacity")?),
                count: crate::parts::count_of(p),
            })
        })
        .collect();
    if storage.is_empty() {
        m.push(
            Target::any_of_roles(&[Role::Obc], Some("storage_capacity")),
            "Onboard storage capacity (part with role = obc)",
        );
    }
    let s = ctx.design.analysis.cdh.as_ref();
    let retention = setting(
        &dims::CDH_RETENTION,
        &[],
        s.and_then(|s| s.retention.as_ref()),
        "Retention period until downlink",
        &mut m,
    );
    m.finish(|| {
        Some(DataBudgetInput {
            parts,
            modes: ctx.modes(),
            mode_shares,
            storage,
            retention: retention?,
            downlink_per_day: None,
            storage_reserve: s.and_then(|s| s.storage_margin).map(Fraction::new),
            compression_ratio: s.and_then(|s| s.compression_ratio),
            hk_overhead: s.and_then(|s| s.packet_overhead),
        })
    })
}

/// 1 日に占めるモードの割合（`analysis.power.mode_share`。ケースがあればそのモードだけで 1）。
pub fn mode_shares(ctx: &Context<'_>, m: &mut MissingInputs) -> Vec<DataBudgetModeShare> {
    if let Some(mode) = ctx.case_mode() {
        return vec![DataBudgetModeShare {
            mode: mode.id.to_string(),
            fraction: 1.0,
        }];
    }
    let shares = ctx.design.analysis.power.as_ref().map(|p| &p.mode_share);
    match shares {
        Some(s) if !s.is_empty() => s
            .iter()
            .map(|(mode, fraction)| DataBudgetModeShare {
                mode: mode.clone(),
                fraction: *fraction,
            })
            .collect(),
        _ => {
            m.push(
                AnalysisSystem::Power.setting_path().child(&["mode_share"]),
                "Fraction of time in each mode, orbit-averaged",
            );
            Vec::new()
        }
    }
}

/// データを発生する部品の射影（モード表示とデータ量で共用）。
pub fn data_parts(ctx: &Context<'_>) -> Vec<DataBudgetPart> {
    ctx.resolved
        .parts
        .values()
        .filter(|p| p.value("hk_rate").is_some() || p.value("mission_rate").is_some())
        .map(|p| DataBudgetPart {
            id: p.id.clone(),
            count: crate::parts::count_of(p),
            redundancy: crate::parts::redundancy(p),
            states: p.states.clone(),
            hk_rate: state_values(ctx, p, "hk_rate").unwrap_or_default(),
            mission_rate: state_values(ctx, p, "mission_rate").unwrap_or_default(),
        })
        .collect()
}
