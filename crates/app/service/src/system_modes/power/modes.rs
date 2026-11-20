//! 電源のモードごとの表示（`PowerModeView`）の組み立て。物理値は型付きの解析の入力・出力から読み、位置と表示文字列を添える（JSON に書いて読み直さない）。
use crate::dto::PowerModeView;
use sat_analysis::power::{
    energy_balance::{EnergyBalanceInput, EnergyBalanceLoad, EnergyBalanceOutput},
    mode_load::ModeLoadOutput,
    orbit_generation::OrbitGenerationOutput,
    sizing::SizingOutput,
};
use sat_analysis::spec::Verdict;

use super::{budget, charts};

/// 負荷の解析（`power.mode_load`）から、モードの並びと判定。
pub(super) fn mode_load(out: &ModeLoadOutput) -> Vec<PowerModeView> {
    out.modes
        .iter()
        .map(|m| PowerModeView {
            id: m.id.clone(),
            label: if m.label.is_empty() {
                m.id.clone()
            } else {
                m.label.clone()
            },
            verdict: Verdict::worst(m.checks.iter().map(|c| c.verdict)),
            sunlit: None,
            eclipse: None,
            charts: Vec::new(),
        })
        .collect()
}

/// 軌道発電の解析（`power.orbit_generation`）から、発電のグラフだけのモード（電力収支が使えないときの代わり）。
pub(super) fn orbital(out: &OrbitGenerationOutput) -> Vec<PowerModeView> {
    charts::orbital(out)
}

/// 電力収支の解析（`power.energy_balance`）から、モードごとの積み上げ棒・判定・グラフ。
pub(super) fn balance(input: &EnergyBalanceInput, out: &EnergyBalanceOutput) -> Vec<PowerModeView> {
    let mut sunlit = Vec::new();
    let mut eclipse = Vec::new();
    let mut modes: Vec<PowerModeView> = out
        .cases
        .iter()
        .enumerate()
        .map(|(index, case)| {
            let mode_id = input
                .cases
                .iter()
                .find(|c| c.id == case.id)
                .and_then(|c| match &c.load {
                    EnergyBalanceLoad::Mode { mode } => Some(mode.as_str()),
                    _ => None,
                })
                .unwrap_or(&case.id);
            let loads = out.mode_loads.modes.iter().find(|m| m.id == mode_id);
            sunlit.push(loads.map(|l| budget::collect(l, case, false, input.load_growth())));
            eclipse.push(loads.map(|l| budget::collect(l, case, true, input.load_growth())));
            PowerModeView {
                id: mode_id.into(),
                label: if case.label.is_empty() {
                    mode_id.into()
                } else {
                    case.label.clone()
                },
                verdict: Verdict::worst(case.checks.iter().map(|c| c.verdict)),
                sunlit: None,
                eclipse: None,
                charts: charts::build(out, index, mode_id),
            }
        })
        .collect();
    // 幅は同じ照明条件の全モードで共通の目盛り
    for ((mode, s), e) in modes
        .iter_mut()
        .zip(budget::finish(sunlit))
        .zip(budget::finish(eclipse))
    {
        mode.sunlit = s;
        mode.eclipse = e;
    }
    modes
}

/// サイジングの解析（`power.sizing`）から、モードの並びと判定。
pub(super) fn sizing(out: &SizingOutput) -> Vec<PowerModeView> {
    out.modes
        .iter()
        .map(|m| PowerModeView {
            id: m.id.clone(),
            label: m.id.clone(),
            verdict: out
                .mode_checks
                .get(&m.id)
                .and_then(|c| Verdict::worst(c.iter().map(|c| c.verdict))),
            sunlit: None,
            eclipse: None,
            charts: Vec::new(),
        })
        .collect()
}

#[cfg(test)]
mod tests;
