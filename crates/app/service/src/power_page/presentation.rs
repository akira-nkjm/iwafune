//! 電源ページの表示用集約。物理値は解析出力から読み、位置と表示文字列を添える。
use crate::dto::*;
use sat_analysis::power::{
    energy_balance::{EnergyBalanceInput, EnergyBalanceLoad, EnergyBalanceOutput},
    sizing::SizingOutput,
};
use sat_analysis::spec::{Check, Verdict};
use sat_units::typed::{Area, Energy, Fraction, NamedKind};
use serde_json::Value;

mod budget;
mod charts;

pub(super) fn label(ja: &str, en: &str) -> AnalysisLabel {
    AnalysisLabel {
        label: ja.into(),
        label_en: en.into(),
    }
}

pub(super) fn build(
    id: &str,
    input: Option<&Value>,
    output: Option<&Value>,
) -> Option<PowerPageView> {
    let output = output?;
    match id {
        "power.energy_balance" => {
            let out: EnergyBalanceOutput = serde_json::from_value(output.clone()).ok()?;
            let input: EnergyBalanceInput = serde_json::from_value(input?.clone()).ok()?;
            Some(balance(&input, &out, output))
        }
        "power.mode_load" => {
            let out: sat_analysis::power::mode_load::ModeLoadOutput =
                serde_json::from_value(output.clone()).ok()?;
            Some(PowerPageView {
                modes: out
                    .modes
                    .into_iter()
                    .map(|m| PowerModeView {
                        id: m.id.clone(),
                        label: if m.label.is_empty() { m.id } else { m.label },
                        verdict: Verdict::worst(m.checks.iter().map(|c| c.verdict)),
                        sunlit: None,
                        eclipse: None,
                        bars: Vec::new(),
                        charts: Vec::new(),
                    })
                    .collect(),
                ..Default::default()
            })
        }
        "power.orbit_generation" => charts::orbital(output),
        "power.sizing" => {
            let out: SizingOutput = serde_json::from_value(output.clone()).ok()?;
            Some(sizing(&out))
        }
        _ => None,
    }
}

fn balance(input: &EnergyBalanceInput, out: &EnergyBalanceOutput, json: &Value) -> PowerPageView {
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
            sunlit.push(loads.map(|l| budget::collect(l, case, false, input.load_growth)));
            eclipse.push(loads.map(|l| budget::collect(l, case, true, input.load_growth)));
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
                bars: vec![
                    bullet::<Fraction>(
                        "dod",
                        label("最大 DoD", "Maximum DoD"),
                        Some(case.max_dod.value()),
                        out.dod_limit.value(),
                        false,
                        check(&case.checks, "dod"),
                    ),
                    bullet::<Energy>(
                        "energy_balance",
                        label("1 周のエネルギー余裕", "Energy margin per orbit"),
                        case.energy_margin_per_orbit.map(|v| v.value()),
                        0.0,
                        true,
                        check(&case.checks, "energy_balance"),
                    ),
                ],
                charts: charts::build(out, json, index, mode_id),
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
    PowerPageView {
        modes,
        ..Default::default()
    }
}

fn sizing(out: &SizingOutput) -> PowerPageView {
    PowerPageView {
        modes: out
            .modes
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
                bars: vec![
                    bullet::<Area>(
                        "area",
                        label("太陽電池の面積", "Solar Cells area"),
                        Some(out.design_area.value()),
                        m.required_area.value(),
                        true,
                        out.mode_checks
                            .get(&m.id)
                            .and_then(|c| check(c, "sap_margin")),
                    ),
                    bullet::<Energy>(
                        "capacity",
                        label("電池容量", "Battery capacity"),
                        Some(out.design_capacity.value()),
                        m.required_capacity.value(),
                        true,
                        out.mode_checks
                            .get(&m.id)
                            .and_then(|c| check(c, "battery_margin")),
                    ),
                ],
                charts: Vec::new(),
            })
            .collect(),
        ..Default::default()
    }
}

fn check(checks: &[Check], key: &str) -> Option<Verdict> {
    checks.iter().find(|c| c.key == key).map(|c| c.verdict)
}

/// 判定のバーの表示位置（0〜100）。比（DoD など）は 0〜100 % に固定し、ほかは設計値と限界の大きい方の 1.15 倍まで。
/// 負の値があるときは 0 を中央に置く。`origin` は 0 の位置（設計値の棒はそこから伸ばす）。
fn bullet<K: NamedKind>(
    key: &str,
    label: AnalysisLabel,
    value: Option<f64>,
    limit: f64,
    minimum: bool,
    verdict: Option<Verdict>,
) -> PowerBulletView {
    let ratio = K::name() == Fraction::name();
    let extent = if ratio {
        1.0_f64.max(value.unwrap_or(0.0).abs())
    } else {
        value
            .unwrap_or(limit)
            .abs()
            .max(limit.abs())
            .max(f64::EPSILON)
            * 1.15
    };
    let negative = value.is_some_and(|v| v < 0.0) || limit < 0.0;
    let low = if negative { -extent } else { 0.0 };
    let position = |v: f64| ((v - low) / (extent - low) * 100.0).clamp(0.0, 100.0);
    let limit_marker = position(limit);
    PowerBulletView {
        key: key.into(),
        label,
        value: value.map(|v| crate::units::quantity_auto(v, K::name())),
        limit: crate::units::quantity_auto(limit, K::name()),
        verdict,
        band_start: if minimum { limit_marker } else { position(0.0) },
        band_end: if minimum { 100.0 } else { limit_marker },
        marker: value.map(position),
        limit_marker,
        origin: position(0.0),
    }
}

#[cfg(test)]
mod tests;
