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
                    verdict_view::<Fraction>(
                        "dod",
                        label("最大 DoD", "Maximum DoD"),
                        // 判定と同じ運用の値（最長継続時間で評価したときはその値）
                        Some(
                            case.by_max_duration
                                .as_ref()
                                .map_or(case.max_dod.value(), |d| d.max_dod.value()),
                        ),
                        out.dod_limit.value(),
                        PowerLimitKind::AtMost,
                        check(&case.checks, "dod"),
                    ),
                    verdict_view::<Energy>(
                        "energy_balance",
                        label("1 周のエネルギー余裕", "Energy margin per orbit"),
                        case.by_max_duration
                            .as_ref()
                            .map_or(case.energy_margin_per_orbit, |d| d.energy_margin_per_orbit)
                            .map(|v| v.value()),
                        0.0,
                        PowerLimitKind::AtLeast,
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
            .map(|m| (m, m.required(out.basis)))
            .map(|(m, (area, capacity))| PowerModeView {
                id: m.id.clone(),
                label: m.id.clone(),
                verdict: out
                    .mode_checks
                    .get(&m.id)
                    .and_then(|c| Verdict::worst(c.iter().map(|c| c.verdict))),
                sunlit: None,
                eclipse: None,
                bars: vec![
                    verdict_view::<Area>(
                        "area",
                        label("太陽電池の面積", "Solar Cells area"),
                        Some(out.design_area.value()),
                        area.value(),
                        PowerLimitKind::AtLeast,
                        out.mode_checks
                            .get(&m.id)
                            .and_then(|c| check(c, "sap_margin")),
                    ),
                    verdict_view::<Energy>(
                        "capacity",
                        label("電池容量", "Battery capacity"),
                        Some(out.design_capacity.value()),
                        capacity.value(),
                        PowerLimitKind::AtLeast,
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

/// メーターの右端（限界・必要値の何倍まで描くか）。限界の線はその 1/1.5 の位置になる。
const METER_SPAN: f64 = 1.5;

/// 判定 1 つ。限界が 0 でない量はメーターを「限界に対する割合」（0〜150 %）で描き、
/// 限界が 0 の量（1 周の収支など）は 0 を中央に置いた符号つきのメーターにする。
pub(super) fn verdict_view<K: NamedKind>(
    key: &str,
    label: AnalysisLabel,
    value: Option<f64>,
    limit: f64,
    kind: PowerLimitKind,
    verdict: Option<Verdict>,
) -> PowerVerdictView {
    let fmt = |v: f64| crate::units::quantity_auto(v, K::name()).display;
    let pct = |r: f64| crate::units::quantity_auto(r, Fraction::name()).display;
    let (fill_start, fill_end, limit_at, note, badness) = match value {
        None => (
            0.0,
            0.0,
            100.0 / METER_SPAN,
            self::label("—", "—"),
            f64::NEG_INFINITY,
        ),
        Some(v) if limit.abs() > f64::EPSILON => {
            let r = v / limit;
            let end = (r / METER_SPAN).clamp(0.0, 1.0) * 100.0;
            let note = match kind {
                PowerLimitKind::AtMost => self::label(
                    &format!("上限の {}", pct(r)),
                    &format!("{} of limit", pct(r)),
                ),
                PowerLimitKind::AtLeast => self::label(
                    &format!("必要の {}", pct(r)),
                    &format!("{} of required", pct(r)),
                ),
            };
            let badness = if kind == PowerLimitKind::AtMost {
                r
            } else {
                -r
            };
            (0.0, end, 100.0 / METER_SPAN, note, badness)
        }
        Some(v) => {
            // 限界が 0: 0 を中央に、|v| の 1.15 倍を端にする
            let span = (v.abs() * 1.15).max(f64::EPSILON);
            let pos = (50.0 + v / span * 50.0).clamp(0.0, 100.0);
            let note = if v < 0.0 {
                self::label(
                    &format!("{} 不足", fmt(-v)),
                    &format!("short by {}", fmt(-v)),
                )
            } else {
                self::label(
                    &format!("{} の余裕", fmt(v)),
                    &format!("{} to spare", fmt(v)),
                )
            };
            (pos.min(50.0), pos.max(50.0), 50.0, note, -v)
        }
    };
    PowerVerdictView {
        key: key.into(),
        label,
        value: value.map(|v| crate::units::quantity_auto(v, K::name())),
        limit: crate::units::quantity_auto(limit, K::name()),
        verdict,
        kind,
        mode: None,
        note,
        fill_start,
        fill_end,
        limit_at,
        badness,
    }
}

/// 判定ごとに全モードの最悪をとる（違反 → 警告 → OK の順、同じなら悪さの大きいもの）。並びは最初に出た順。
pub(super) fn worst_verdicts(modes: &[PowerModeView]) -> Vec<PowerVerdictView> {
    let severity = |v: Option<Verdict>| match v {
        Some(Verdict::Violation) => 3,
        Some(Verdict::Warning) => 2,
        Some(Verdict::Ok) => 1,
        None => 0,
    };
    let mut out: Vec<PowerVerdictView> = Vec::new();
    for mode in modes {
        for bar in &mode.bars {
            let candidate = PowerVerdictView {
                mode: Some(mode.id.clone()),
                ..bar.clone()
            };
            match out.iter_mut().find(|b| b.key == bar.key) {
                Some(best) => {
                    let worse = (severity(candidate.verdict), candidate.badness)
                        > (severity(best.verdict), best.badness);
                    if worse {
                        *best = candidate;
                    }
                }
                None => out.push(candidate),
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
