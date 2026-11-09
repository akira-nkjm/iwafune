//! C&DH の射影: `cdh.data_budget`。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::parts::by_role;
use crate::quantity::qty;
use fune_model::Target;
use fune_model::path::SettingPath;
use fune_model::resolve::{ResolvedPart, SiValue};
use sat_analysis::cdh::DataBudgetInput;
use sat_analysis::cdh::data_budget::{
    DataBudgetModeShare, DataBudgetPart, DataBudgetRates, DataBudgetStorage,
};
use sat_units::typed::{Qty, Time};

/// `cdh.data_budget`
pub fn data_budget(ctx: &Context<'_>) -> Result<DataBudgetInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let parts: Vec<DataBudgetPart> = ctx
        .resolved
        .parts
        .values()
        .filter(|p| p.value("hk_rate").is_some() || p.value("mission_rate").is_some())
        .map(|p| DataBudgetPart {
            id: p.id.clone(),
            count: crate::parts::count_of(p),
            hk_rate: rates(p, "hk_rate"),
            mission_rate: rates(p, "mission_rate"),
        })
        .collect();
    if parts.is_empty() {
        m.push(
            Target::any_part("hk_rate"),
            "HK・ミッションデータのレートを持つ部品がありません",
        );
    }
    let mode_shares = mode_shares(ctx, &mut m);
    // 容量の合計（員数込み）は解析が求めるので、1 台あたりの容量と員数をそのまま渡す。
    let storage: Vec<DataBudgetStorage> = by_role(ctx, "obc")
        .filter_map(|p| {
            Some(DataBudgetStorage {
                part: p.id.clone(),
                capacity: Qty::from_raw(p.scalar("storage_capacity")?),
                count: crate::parts::count_of(p),
            })
        })
        .collect();
    if storage.is_empty() {
        m.push(
            Target::any_of_roles(&["obc"], Some("storage_capacity")),
            "搭載ストレージの容量（role = obc の部品）",
        );
    }
    let s = ctx.design.analysis.cdh.as_ref();
    let retention = qty::<Time>(
        s.and_then(|s| s.retention.as_ref()),
        SettingPath::analysis(&["cdh", "retention"]),
        "地上に降ろすまで保持する期間",
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
            min_storage_margin: s.and_then(|s| s.storage_margin),
            compression_ratio: s.and_then(|s| s.compression_ratio),
            hk_overhead: s.and_then(|s| s.packet_overhead),
        })
    })
}

fn rates(p: &ResolvedPart, attr: &str) -> DataBudgetRates {
    match p.value(attr) {
        Some(SiValue::Scalar(s)) => DataBudgetRates {
            default: Some(Qty::from_raw(s.value)),
            by_state: Default::default(),
        },
        Some(SiValue::ByState(map)) => DataBudgetRates {
            default: None,
            by_state: map
                .iter()
                .map(|(k, v)| (k.clone(), Qty::from_raw(v.value)))
                .collect(),
        },
        _ => DataBudgetRates {
            default: None,
            by_state: Default::default(),
        },
    }
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
                SettingPath::analysis(&["power", "mode_share"]),
                "軌道平均で各モードにいる時間の割合",
            );
            Vec::new()
        }
    }
}
