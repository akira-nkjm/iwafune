//! 電源ページ全体の取得。電源の解析 4 つ（負荷・軌道発電・電力収支・サイジング）を型付きで回し（`analysis::run_typed_in`）、
//! その入力・出力を型のまま表示の組み立てに渡す（JSON は API の結果にだけ載る）。**電源のページの解析の一覧と並びはここだけ。**
//! ケースの解決と不足入力・失敗の表し方は解析の入口（`analysis::run_analysis`）と同じ。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `presentation` | 解析の型付きの出力から流れ・ブレットバー・グラフの表示定義へ |
//! | `overview` | 概要の解析の表の行（解析ごとの小さな棒グラフ・解析をまたいで同じ表） |
use crate::analysis::{TypedRun, run_typed_in};
use crate::dto::{AnalysisEntry, PowerAnalysisView, PowerModeView, PowerPageQuery, PowerPageView};
use crate::workspace::Snapshot;
use crate::{Result, ServiceError, Workspace};
use sat_analysis::power::{EnergyBalance, ModeLoad, OrbitGeneration, Sizing};
use sat_analysis::spec::Analysis;

mod overview;
mod presentation;

/// 電源ページの解析 4 つの型付きの実行結果。
struct Runs {
    load: TypedRun<ModeLoad>,
    orbit: TypedRun<OrbitGeneration>,
    balance: TypedRun<EnergyBalance>,
    sizing: TypedRun<Sizing>,
}

/// 指定ケースの電源ページ全体を返す（保存済みの設計。`power_page_with` の `ops` 無し）。
pub fn power_page(ws: &Workspace, case: Option<&str>) -> Result<PowerPageView> {
    power_page_with(
        ws,
        &PowerPageQuery {
            case: case.map(str::to_owned),
            ops: Vec::new(),
        },
    )
}

/// 電源ページ全体（`GET`・`POST /power_page`）。`query.ops` があれば書き込まずに当てた設計で、ケース省略時も各解析と同じ規則で
/// 解決する。編集が通らなければ保存と同じエラー。解析の不足入力・失敗は元の結果として返し、得られた解析だけで表示を組み立てる。
pub fn power_page_with(ws: &Workspace, query: &PowerPageQuery) -> Result<PowerPageView> {
    let snapshot = crate::preview::snapshot_with(ws, &query.ops)?;
    let case = query.case.as_deref();
    let mut runs = Runs {
        load: run_typed_in(&snapshot, case)?,
        orbit: run_typed_in(&snapshot, case)?,
        balance: run_typed_in(&snapshot, case)?,
        sizing: run_typed_in(&snapshot, case)?,
    };
    overview::add_charts(&mut runs);
    let mut page = assemble(&runs);
    page.analyses = analyses(runs)?;
    sort_modes(&snapshot, &mut page.modes);
    Ok(page)
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

/// 元の解析（定義と結果）の並び。
fn analyses(runs: Runs) -> Result<Vec<PowerAnalysisView>> {
    let catalog = crate::analysis::analysis_catalog()?;
    let view = |id: &str, result| -> Result<PowerAnalysisView> {
        let entry: AnalysisEntry = catalog
            .iter()
            .find(|e| e.info.id == id)
            .cloned()
            .ok_or_else(|| ServiceError::not_found("analysis", id))?;
        Ok(PowerAnalysisView { entry, result })
    };
    Ok(vec![
        view(ModeLoad::ID, runs.load.result)?,
        view(OrbitGeneration::ID, runs.orbit.result)?,
        view(EnergyBalance::ID, runs.balance.result)?,
        view(Sizing::ID, runs.sizing.result)?,
    ])
}

/// 解析ごとの表示をモードごとに統合する。電力収支が使えない場合でも、取得できた軌道発電のグラフを表示する。
fn assemble(runs: &Runs) -> PowerPageView {
    let pages = [
        runs.load.output.as_ref().map(presentation::mode_load),
        runs.balance
            .input
            .as_ref()
            .zip(runs.balance.output.as_ref())
            .map(|(input, out)| presentation::balance(input, out)),
        runs.sizing.output.as_ref().map(presentation::sizing),
    ];
    let mut modes: Vec<PowerModeView> = Vec::new();
    for page in pages.into_iter().flatten() {
        for incoming in page.modes {
            if let Some(mode) = modes.iter_mut().find(|m| m.id == incoming.id) {
                merge_mode(mode, incoming);
            } else {
                modes.push(incoming);
            }
        }
    }
    let orbital = runs
        .orbit
        .output
        .as_ref()
        .map(presentation::orbital)
        .map(|page| page.modes)
        .unwrap_or_default();
    if modes.is_empty() {
        modes = orbital;
    } else if let Some(orbit) = orbital.first() {
        for mode in &mut modes {
            if mode.charts.is_empty() {
                mode.charts.clone_from(&orbit.charts);
            }
        }
    }
    PowerPageView {
        modes,
        analyses: Vec::new(),
    }
}

fn merge_mode(mode: &mut PowerModeView, incoming: PowerModeView) {
    mode.verdict =
        sat_analysis::spec::Verdict::worst(mode.verdict.into_iter().chain(incoming.verdict));
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
mod tests;
