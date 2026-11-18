//! 既存の負荷・発電・充放電の集計を、モードごとの積み上げ棒に写す（物理の計算はしない）。
use super::label;
use crate::dto::*;
use sat_analysis::power::{
    energy_balance::{EnergyBalanceCaseResult, LoadGrowth},
    mode_load::ModeLoadMode,
};
use sat_units::typed::{NamedKind, Power};
use std::collections::BTreeMap;

/// 幅を決める前の棒（電力は W）。
pub(super) struct RawBudget {
    supply: f64,
    /// 余裕の少ない順に並べる値（日照: 余り − 不足、蝕: −放電）
    order: f64,
    segments: Vec<(PowerBudgetKind, Option<System>, AnalysisLabel, f64)>,
}

impl RawBudget {
    /// 棒の長さ（区間の合計）。
    fn total(&self) -> f64 {
        self.segments.iter().map(|s| s.3).sum()
    }
}

/// 1 モード・1 照明条件の区間を集める。日照は 系の消費 → 充電 → 余り → 不足、蝕は系の消費だけ。
pub(super) fn collect(
    loads: &ModeLoadMode,
    case: &EnergyBalanceCaseResult,
    eclipse: bool,
    growth: LoadGrowth,
) -> RawBudget {
    let mut groups: BTreeMap<String, f64> = BTreeMap::new();
    for part in &loads.parts {
        let value = match (eclipse, growth) {
            (false, LoadGrowth::Included) => part.mean_with_growth,
            (false, LoadGrowth::Excluded) => part.mean,
            (true, LoadGrowth::Included) => part.mean_eclipse_with_growth,
            (true, LoadGrowth::Excluded) => part.mean_eclipse,
        };
        if let Some(v) = value {
            *groups.entry(part.group.clone()).or_default() += v.value();
        }
    }
    let mut segments: Vec<_> = groups
        .into_iter()
        .filter(|(_, p)| *p > 0.0)
        .map(|(id, p)| {
            let system = id
                .parse::<fune_model::design::AnalysisSystem>()
                .ok()
                .map(fune_projection::system::to_analysis);
            (PowerBudgetKind::Load, system, system_label(system, &id), p)
        })
        .collect();
    let supply = if eclipse {
        case.load_eclipse.value()
    } else {
        let extra = [
            (
                PowerBudgetKind::Charge,
                label("電池へ充電", "Battery charge"),
                case.sunlit_average_charge,
            ),
            (
                PowerBudgetKind::Shunt,
                label("余り（シャント）", "Surplus (shunt)"),
                case.sunlit_average_shunt,
            ),
            (
                PowerBudgetKind::Deficit,
                label("不足（電池から）", "Deficit (from battery)"),
                case.sunlit_average_discharge,
            ),
        ];
        // 発電に比べて丸めの残りしかない区間（10⁻¹⁰ W など）は出さない
        let floor = case.sunlit_average_generation.value().abs() * 1e-9;
        for (kind, name, value) in extra {
            if value.value() > floor {
                segments.push((kind, None, name, value.value()));
            }
        }
        case.sunlit_average_generation.value()
    };
    let watts = |kind: PowerBudgetKind| {
        segments
            .iter()
            .filter(|s| s.0 == kind)
            .map(|s| s.3)
            .sum::<f64>()
    };
    let order = if eclipse {
        -supply
    } else {
        watts(PowerBudgetKind::Shunt) - watts(PowerBudgetKind::Deficit)
    };
    RawBudget {
        supply,
        order,
        segments,
    }
}

/// 系の表示名（組み込みの語彙）。系でなければグループ名のまま。
fn system_label(system: Option<System>, id: &str) -> AnalysisLabel {
    system
        .and_then(|s| {
            let model = fune_projection::system::from_analysis(s);
            fune_model::builtin::labels()
                .section
                .get(model.as_str())
                .map(|l| label(&l.label, l.label_en.as_deref().unwrap_or(&l.label)))
        })
        .unwrap_or_else(|| label(id, id))
}

/// 同じ照明条件の全モードで共通の最大（棒の長さ）に対する幅を付ける。
pub(super) fn finish(raws: Vec<Option<RawBudget>>) -> Vec<Option<PowerBudgetView>> {
    let max = raws
        .iter()
        .flatten()
        .map(RawBudget::total)
        .fold(0.0_f64, f64::max)
        .max(f64::EPSILON);
    raws.into_iter()
        .map(|raw| {
            raw.map(|raw| PowerBudgetView {
                supply: crate::units::quantity_auto(raw.supply, Power::name()),
                order: raw.order,
                segments: raw
                    .segments
                    .into_iter()
                    .map(|(kind, system, label, watts)| PowerBudgetSegment {
                        kind,
                        system,
                        label,
                        value: crate::units::quantity_auto(watts, Power::name()),
                        fraction: watts / max,
                    })
                    .collect(),
            })
        })
        .collect()
}
