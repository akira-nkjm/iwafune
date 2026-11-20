//! 電源の概要の解析の表の行（`AnalysisRunResult.overview`）に足すもの。主な結果と判定のまとまりは系に依らない
//! [`super`]（実行の結果を作るときに入る）、ここは電源の解析ごとの小さな棒グラフの選び方と、解析をまたいで
//! 同じ表だけを持つ。棒グラフは結果の出力の JSON を型に戻して作る（電源の出力の型は `sat_analysis::power`）。
//!
//! | 解析 | 棒グラフ |
//! |---|---|
//! | `power.mode_load` | モードごとの消費電力（成長余裕込み。いちばん大きいモードに対する長さ） |
//! | `power.orbit_generation` | 軌道平均の発電の BOL・EOL（大きいほうに対する長さ）と日照率 |
//! | `power.energy_balance` | ケースとモードの並びごとの最大 DoD と上限（判定の位置は概要の上段の判定と同じ） |
//! | `power.sizing` | 太陽電池の面積と電池容量の今と必要（判定を決めた必要値） |

use super::{overview, share_of_largest};
use crate::analysis::labels::label;
use crate::analysis::run_typed_in;
use crate::dto::{AnalysisLabel, AnalysisRunResult, OverviewBar, OverviewChart, OverviewReference};
use crate::units::quantity_auto;
use crate::workspace::Snapshot;
use meter::{LimitKind, Meter, area_bar, capacity_bar, dod_bar};
use sat_analysis::power::{
    EnergyBalance, EnergyBalanceOutput, ModeLoad, ModeLoadOutput, OrbitGeneration,
    OrbitGenerationOutput, Sizing, SizingOutput,
};
use sat_analysis::spec::{Analysis, Verdict};
use sat_units::typed::{Fraction, NamedKind, Power};
use serde::{Serialize, de::DeserializeOwned};

mod meter;

/// 電源の解析の結果なら、概要の行に棒グラフと隠す表を足す。電源でない解析・出力の無い結果は何もしない。
/// `snapshot` と `case` は、隠す表のために同じ設計・ケースで別の解析を回すのに使う（入力を丸ごと与えた実行は `snapshot` が無く、隠さない）。
pub(super) fn add(result: &mut AnalysisRunResult, snapshot: Option<&Snapshot>, case: Option<&str>) {
    let Some(output) = result.output.clone() else {
        return;
    };
    let (chart, hidden) = match result.id.as_str() {
        ModeLoad::ID => (read(&output).map(|o| mode_load(&o)), Vec::new()),
        OrbitGeneration::ID => (read(&output).map(|o| orbit_generation(&o)), Vec::new()),
        EnergyBalance::ID => {
            let out: Option<EnergyBalanceOutput> = read(&output);
            let hidden = out
                .as_ref()
                .zip(snapshot)
                .map(|(out, s)| hidden_tables(out, s, case))
                .unwrap_or_default();
            (out.as_ref().map(energy_balance), hidden)
        }
        Sizing::ID => (read(&output).map(|o| sizing(&o)), Vec::new()),
        _ => return,
    };
    result.overview = overview(result, chart, hidden);
}

/// 出力の JSON を型に戻す（戻せなければ棒グラフ無し）。
fn read<T: DeserializeOwned>(output: &serde_json::Value) -> Option<T> {
    serde_json::from_value(output.clone()).ok()
}

/// 「データを見る」で隠す表（電力収支の出力のうち、ほかの解析の行に同じ中身の表があるもの）。位置は出力の JSON の
/// キー（Web が隠す表を引く）。同じ設計・ケースでほかの解析を回し、中身が同じときだけ隠す
/// （回せない・出力が無いときは隠さない）。
///
/// | 隠す表 | 同じ表を持つ解析 |
/// |---|---|
/// | `mode_loads.modes` | `power.mode_load` の `modes` |
/// | `eclipses` | `power.orbit_generation` の `eclipses` |
fn hidden_tables(
    balance: &EnergyBalanceOutput,
    snapshot: &Snapshot,
    case: Option<&str>,
) -> Vec<Vec<String>> {
    let mut hidden = Vec::new();
    let same_as_load = run_typed_in::<ModeLoad>(snapshot, case)
        .ok()
        .and_then(|r| r.output)
        .is_some_and(|load| same_table(&balance.mode_loads.modes, &load.modes));
    if same_as_load {
        hidden.push(vec!["mode_loads".to_owned(), "modes".to_owned()]);
    }
    let same_as_orbit = run_typed_in::<OrbitGeneration>(snapshot, case)
        .ok()
        .and_then(|r| r.output)
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

/// 判定のメーター（`meter::verdict_view` の位置の計算）を棒 1 本にする。位置は 0〜100 → 0〜1。
fn meter(key: &str, l: AnalysisLabel, v: Meter) -> OverviewBar {
    let reference = match v.kind {
        LimitKind::AtMost => label("上限", "Limit"),
        LimitKind::AtLeast => label("必要", "Required"),
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
