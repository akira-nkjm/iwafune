//! 電源のモードごとの電力の表示（`SystemModeView.power`）。電源の解析 4 つ（負荷・軌道発電・電力収支・サイジング）を
//! 型付きで回し（`analysis::run_typed_in`。ケースの解決と不足入力・失敗の扱いは解析の入口と同じ）、その入力・出力を型のまま
//! 表示の組み立てに渡す（JSON に書いて読み直さない）。**電源のモードの見方が使う解析の一覧はここだけ。**
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `modes` | 解析の型付きの出力から、モードごとの積み上げ棒・判定・グラフの表示定義へ |
//! | `budget` | 負荷・発電・充放電の集計を積み上げ棒の区間に写す |
//! | `charts` | グラフのチャンネルと共通横軸・時間帯の定義 |
use crate::Result;
use crate::analysis::{TypedRun, run_typed_in};
use crate::dto::{ModeValuesView, PowerModeView, SystemModeView};
use crate::workspace::Snapshot;
use sat_analysis::power::{EnergyBalance, ModeLoad, OrbitGeneration, Sizing};
use sat_analysis::spec::Verdict;

mod budget;
mod charts;
mod modes;

/// 電源の解析 4 つの型付きの実行結果。
struct Runs {
    load: TypedRun<ModeLoad>,
    orbit: TypedRun<OrbitGeneration>,
    balance: TypedRun<EnergyBalance>,
    sizing: TypedRun<Sizing>,
}

/// 保存済みの（または下書きを当てた）設計の電源の解析を回し、モードごとの表示を `views`（部品別の値の表）に足す。
/// 解析の不足入力・失敗は得られた解析だけで組み立てる（元の理由は各解析の結果が持つ）。
pub(super) fn attach(
    snapshot: &Snapshot,
    case: Option<&str>,
    views: &mut Vec<SystemModeView>,
) -> Result<()> {
    let runs = Runs {
        load: run_typed_in(snapshot, case)?,
        orbit: run_typed_in(snapshot, case)?,
        balance: run_typed_in(snapshot, case)?,
        sizing: run_typed_in(snapshot, case)?,
    };
    let mut modes = assemble(&runs);
    sort_modes(snapshot, &mut modes);
    for mode in modes {
        match views.iter_mut().find(|v| v.id == mode.id) {
            Some(view) => view.power = Some(mode),
            None => views.push(SystemModeView {
                id: mode.id.clone(),
                sunlit: ModeValuesView::default(),
                eclipse: ModeValuesView::default(),
                aocs: None,
                power: Some(mode),
            }),
        }
    }
    Ok(())
}

/// モードは設計の表示順（fune.toml の `[modes] order`）に並べる。一覧に無いものは後ろ。
fn sort_modes(snapshot: &Snapshot, modes: &mut [PowerModeView]) {
    let order: Vec<String> = snapshot
        .design
        .ordered_modes()
        .iter()
        .map(|m| m.id.to_string())
        .collect();
    modes.sort_by_key(|m| {
        order
            .iter()
            .position(|id| *id == m.id)
            .unwrap_or(usize::MAX)
    });
}

/// 解析ごとの表示をモードごとに統合する。電力収支が使えない場合でも、取得できた軌道発電のグラフを表示する。
fn assemble(runs: &Runs) -> Vec<PowerModeView> {
    let pages = [
        runs.load.output.as_ref().map(modes::mode_load),
        runs.balance
            .input
            .as_ref()
            .zip(runs.balance.output.as_ref())
            .map(|(input, out)| modes::balance(input, out)),
        runs.sizing.output.as_ref().map(modes::sizing),
    ];
    let mut merged: Vec<PowerModeView> = Vec::new();
    for incoming in pages.into_iter().flatten().flatten() {
        if let Some(mode) = merged.iter_mut().find(|m| m.id == incoming.id) {
            merge_mode(mode, incoming);
        } else {
            merged.push(incoming);
        }
    }
    let orbital = runs
        .orbit
        .output
        .as_ref()
        .map(modes::orbital)
        .unwrap_or_default();
    if merged.is_empty() {
        return orbital;
    }
    if let Some(orbit) = orbital.first() {
        for mode in &mut merged {
            if mode.charts.is_empty() {
                mode.charts.clone_from(&orbit.charts);
            }
        }
    }
    merged
}

fn merge_mode(mode: &mut PowerModeView, incoming: PowerModeView) {
    mode.verdict = Verdict::worst(mode.verdict.into_iter().chain(incoming.verdict));
    if incoming.sunlit.is_some() {
        mode.sunlit = incoming.sunlit;
    }
    if incoming.eclipse.is_some() {
        mode.eclipse = incoming.eclipse;
    }
    if !incoming.charts.is_empty() {
        mode.charts = incoming.charts;
    }
}

#[cfg(test)]
mod page_tests;
