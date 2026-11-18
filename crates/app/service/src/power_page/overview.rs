//! 電源の概要の解析の表の行（`AnalysisRunResult.overview`）に足すもの。主な結果と判定のまとまりは系に依らない
//! `analysis::overview`（実行の結果を作るときに入る）、ここは電源の解析ごとの小さな棒グラフの選び方と、解析をまたいで
//! 同じ表だけを持つ。
//!
//! | 解析 | 棒グラフ |
//! |---|---|
//! | `power.mode_load` | モードごとの消費電力（成長余裕込み。いちばん大きいモードに対する長さ） |
//! | `power.orbit_generation` | 軌道平均の発電の BOL・EOL（大きいほうに対する長さ）と日照率 |
//! | `power.energy_balance` | ケースとモードの並びごとの最大 DoD と上限（判定の位置は概要の上段の判定と同じ） |
//! | `power.sizing` | 太陽電池の面積と電池容量の今と必要（判定を決めた必要値） |

use super::Runs;
use super::presentation::{area_bar, capacity_bar, dod_bar, label};
use crate::analysis::TypedRun;
use crate::analysis::overview::{overview, share_of_largest};
use crate::dto::{
    AnalysisLabel, OverviewBar, OverviewChart, OverviewReference, PowerLimitKind, PowerVerdictView,
};
use crate::units::quantity_auto;
use sat_analysis::power::{
    EnergyBalanceOutput, ModeLoadOutput, OrbitGenerationOutput, SizingOutput,
};
use sat_analysis::spec::{Analysis, Verdict};
use sat_units::typed::{Fraction, NamedKind, Power};
use serde::Serialize;

/// 各解析の結果の概要（`overview`）に、電源の棒グラフと隠す表を入れる。棒グラフは型付きの出力から作る
/// （出力が無い解析は棒グラフ無し）。
pub(super) fn add_charts(runs: &mut Runs) {
    let hidden = hidden_tables(runs);
    let charts = (
        runs.load.output.as_ref().map(mode_load),
        runs.orbit.output.as_ref().map(orbit_generation),
        runs.balance.output.as_ref().map(energy_balance),
        runs.sizing.output.as_ref().map(sizing),
    );
    put(&mut runs.load, charts.0, Vec::new());
    put(&mut runs.orbit, charts.1, Vec::new());
    put(&mut runs.balance, charts.2, hidden);
    put(&mut runs.sizing, charts.3, Vec::new());
}

fn put<A: Analysis>(run: &mut TypedRun<A>, chart: Option<OverviewChart>, hidden: Vec<Vec<String>>) {
    run.result.overview = overview(&run.result, chart, hidden);
}

/// 「データを見る」で隠す表（電力収支の出力のうち、ほかの解析の行に同じ中身の表があるもの）。位置は出力の JSON の
/// キー（Web が隠す表を引く）。両方の出力があり、中身が同じときだけ隠す。
///
/// | 隠す表 | 同じ表を持つ解析 |
/// |---|---|
/// | `mode_loads.modes` | `power.mode_load` の `modes` |
/// | `eclipses` | `power.orbit_generation` の `eclipses` |
fn hidden_tables(runs: &Runs) -> Vec<Vec<String>> {
    let Some(balance) = &runs.balance.output else {
        return Vec::new();
    };
    let mut hidden = Vec::new();
    let same_as_load = runs
        .load
        .output
        .as_ref()
        .is_some_and(|load| same_table(&balance.mode_loads.modes, &load.modes));
    if same_as_load {
        hidden.push(vec!["mode_loads".to_owned(), "modes".to_owned()]);
    }
    let same_as_orbit = runs
        .orbit
        .output
        .as_ref()
        .is_some_and(|orbit| same_table(&balance.eclipses, &orbit.eclipses));
    if same_as_orbit {
        hidden.push(vec!["eclipses".to_owned()]);
    }
    hidden
}

/// 2 つの表が API に載る形で同じか（書き出せなければ違うとする）。
fn same_table<T: Serialize, U: Serialize>(a: &T, b: &U) -> bool {
    match (serde_json::to_value(a), serde_json::to_value(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
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

/// 判定のメーター（`presentation::verdict_view` の位置の計算）を棒 1 本にする。位置は 0〜100 → 0〜1。
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
