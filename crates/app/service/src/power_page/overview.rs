//! 電源の概要の解析の表（`PowerAnalysisView.overview`）。主な結果と判定のまとまりは系に依らない
//! `analysis::overview`、ここは電源の解析ごとの小さな棒グラフの選び方と、解析をまたいで同じ表だけを持つ。
//!
//! | 解析 | 棒グラフ |
//! |---|---|
//! | `power.mode_load` | モードごとの消費電力（成長余裕込み。いちばん大きいモードに対する長さ） |
//! | `power.orbit_generation` | 軌道平均の発電の BOL・EOL（大きいほうに対する長さ）と日照率 |
//! | `power.energy_balance` | ケースとモードの並びごとの最大 DoD と上限（判定の位置は概要の上段の判定と同じ） |
//! | `power.sizing` | 太陽電池の面積と電池容量の今と必要（判定を決めた必要値） |

use super::presentation::{area_bar, capacity_bar, dod_bar, label};
use crate::analysis::overview::{overview, share_of_largest};
use crate::dto::{
    AnalysisLabel, AnalysisOverview, AnalysisRunResult, OverviewBar, OverviewChart,
    OverviewReference, PowerLimitKind, PowerVerdictView,
};
use crate::units::quantity_auto;
use sat_analysis::power::{
    EnergyBalance, EnergyBalanceOutput, ModeLoad, ModeLoadOutput, OrbitGeneration,
    OrbitGenerationOutput, Sizing, SizingOutput,
};
use sat_analysis::spec::{Analysis, Verdict};
use sat_units::typed::{Fraction, NamedKind, Power};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// 解析をまたいで同じ表: （表を隠す解析, 表の位置, 同じ表を持つ解析, その表の位置）。両方の結果があり、中身が同じときだけ隠す。
const SAME_TABLES: &[(&str, &[&str], &str, &[&str])] = &[
    (
        <EnergyBalance as Analysis>::ID,
        &["mode_loads", "modes"],
        <ModeLoad as Analysis>::ID,
        &["modes"],
    ),
    (
        <EnergyBalance as Analysis>::ID,
        &["eclipses"],
        <OrbitGeneration as Analysis>::ID,
        &["eclipses"],
    ),
];

/// ページの解析の並びの順に、各解析の概要。
pub(super) fn overviews(results: &[&AnalysisRunResult]) -> Vec<AnalysisOverview> {
    results
        .iter()
        .map(|r| overview(r, chart(r), hidden_tables(r, results)))
        .collect()
}

/// 「データを見る」で隠す表（ほかの解析の行に同じ中身の表があるもの）。
fn hidden_tables(r: &AnalysisRunResult, all: &[&AnalysisRunResult]) -> Vec<Vec<String>> {
    let table = |result: &AnalysisRunResult, at: &[&str]| -> Option<Value> {
        at.iter()
            .try_fold(result.output.as_ref()?, |v, k| v.get(k))
            .cloned()
    };
    SAME_TABLES
        .iter()
        .filter(|(id, ..)| *id == r.id)
        .filter(|(_, at, other, other_at)| {
            let mine = table(r, at);
            mine.is_some()
                && all
                    .iter()
                    .any(|o| o.id == *other && table(o, other_at) == mine)
        })
        .map(|(_, at, ..)| at.iter().map(|k| (*k).to_owned()).collect())
        .collect()
}

/// 解析ごとの小さな棒グラフ（出力が無いか型に戻せなければ null）。
fn chart(r: &AnalysisRunResult) -> Option<OverviewChart> {
    let output = r.output.as_ref()?;
    match r.id.as_str() {
        id if id == <ModeLoad as Analysis>::ID => Some(mode_load(&typed(output)?)),
        id if id == <OrbitGeneration as Analysis>::ID => Some(orbit_generation(&typed(output)?)),
        id if id == <EnergyBalance as Analysis>::ID => Some(energy_balance(&typed(output)?)),
        id if id == <Sizing as Analysis>::ID => Some(sizing(&typed(output)?)),
        _ => None,
    }
}

fn typed<T: DeserializeOwned>(output: &Value) -> Option<T> {
    serde_json::from_value(output.clone()).ok()
}

/// 名前が日本語だけのもの（設計データのモード・ケースの表示名）。
fn name(s: &str) -> AnalysisLabel {
    label(s, s)
}

/// モードごとの消費電力（成長余裕込み）。
fn mode_load(out: &ModeLoadOutput) -> OverviewChart {
    let largest = out
        .modes
        .iter()
        .map(|m| m.total_with_growth.value())
        .fold(0.0, f64::max);
    let bars = out
        .modes
        .iter()
        .map(|m| {
            let value = quantity_auto(m.total_with_growth.value(), Power::name());
            OverviewBar {
                key: m.id.clone(),
                label: name(if m.label.is_empty() { &m.id } else { &m.label }),
                fill: share_of_largest(Some(&value), largest),
                value: Some(value),
                reference: None,
                verdict: Verdict::worst(m.checks.iter().map(|c| c.verdict)),
                note: None,
            }
        })
        .collect();
    OverviewChart {
        label: label(
            "モードごとの消費電力（成長余裕込み）",
            "Load per mode (with growth)",
        ),
        bars,
    }
}

/// 軌道平均の発電の BOL・EOL と日照率。
fn orbit_generation(out: &OrbitGenerationOutput) -> OverviewChart {
    let largest = out
        .orbit_average_bol
        .value()
        .max(out.orbit_average_eol.value());
    let power = |key: &str, l: AnalysisLabel, si: f64| {
        let value = quantity_auto(si, Power::name());
        OverviewBar {
            key: key.into(),
            label: l,
            fill: share_of_largest(Some(&value), largest),
            value: Some(value),
            reference: None,
            verdict: None,
            note: None,
        }
    };
    let fraction = out.sunlit_fraction.value();
    OverviewChart {
        label: label(
            "軌道平均の発電と日照率",
            "Orbit-average generation and sunlit fraction",
        ),
        bars: vec![
            power(
                "bol",
                label("軌道平均の発電（BOL）", "Orbit-average generation (BOL)"),
                out.orbit_average_bol.value(),
            ),
            power(
                "eol",
                label("軌道平均の発電（EOL）", "Orbit-average generation (EOL)"),
                out.orbit_average_eol.value(),
            ),
            OverviewBar {
                key: "sunlit_fraction".into(),
                label: label("日照率", "Sunlit fraction"),
                value: Some(quantity_auto(fraction, Fraction::name())),
                fill: fraction.clamp(0.0, 1.0),
                reference: None,
                verdict: None,
                note: None,
            },
        ],
    }
}

/// ケースとモードの並びごとの最大 DoD と上限。
fn energy_balance(out: &EnergyBalanceOutput) -> OverviewChart {
    let cases = out
        .cases
        .iter()
        .chain(out.sequence.iter().map(|s| &s.balance));
    OverviewChart {
        label: label("ケースごとの最大 DoD", "Maximum DoD per case"),
        bars: cases
            .map(|c| {
                let l = name(if c.label.is_empty() { &c.id } else { &c.label });
                meter(&c.id, l, dod_bar(c, out.dod_limit))
            })
            .collect(),
    }
}

/// 太陽電池の面積と電池容量の今と必要。
fn sizing(out: &SizingOutput) -> OverviewChart {
    let area = area_bar(out, out.required_area, Some(&out.checks));
    let capacity = capacity_bar(out, out.required_capacity, Some(&out.checks));
    OverviewChart {
        label: label("今の値と必要な値", "Current and required"),
        bars: vec![
            meter("area", area.label.clone(), area),
            meter("capacity", capacity.label.clone(), capacity),
        ],
    }
}

/// 判定のメーター（上段の判定と同じ位置の計算）を棒 1 本にする。位置は 0〜100 → 0〜1。
fn meter(key: &str, l: AnalysisLabel, v: PowerVerdictView) -> OverviewBar {
    let reference = match v.kind {
        PowerLimitKind::AtMost => label("上限", "Limit"),
        PowerLimitKind::AtLeast => label("必要", "Required"),
    };
    OverviewBar {
        key: key.into(),
        label: l,
        value: v.value,
        fill: (v.fill_end / 100.0).clamp(0.0, 1.0),
        reference: Some(OverviewReference {
            label: reference,
            value: v.limit,
            at: (v.limit_at / 100.0).clamp(0.0, 1.0),
        }),
        verdict: v.verdict,
        note: Some(v.note),
    }
}

#[cfg(test)]
mod tests;
