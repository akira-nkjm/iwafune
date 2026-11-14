//! 電源ページ全体の取得。解析入口のケース解決とエラー表現をそのまま使い、表示定義を Rust で統合する。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `presentation` | 解析出力から流れ・ブレットバー・グラフの表示定義へ |
//!
//! フロー図（`PowerPageView.flow`）は `crate::flow` が同じ解析の一覧から作る。
use crate::dto::{PowerAnalysisView, PowerModeView, PowerPageView, RunAnalysisRequest};
use crate::{Result, Workspace, analysis};

mod presentation;

const ANALYSES: &[&str] = &[
    "power.mode_load",
    "power.orbit_generation",
    "power.energy_balance",
    "power.sizing",
];

/// 指定ケースの電源ページ全体を返す。ケース省略時も各解析と同じ規則で解決する。
/// 解析の不足入力・失敗は元の結果として返し、得られた解析だけで表示を組み立てる。
pub fn power_page(ws: &Workspace, case: Option<&str>) -> Result<PowerPageView> {
    let req = RunAnalysisRequest {
        case: case.map(str::to_owned),
        ..Default::default()
    };
    let catalog = analysis::analysis_catalog()?;
    let analyses = ANALYSES
        .iter()
        .map(|id| {
            let entry = catalog
                .iter()
                .find(|e| e.info.id == *id)
                .cloned()
                .ok_or_else(|| crate::ServiceError::not_found("解析", *id))?;
            let result = analysis::run_analysis(ws, id, &req)?;
            Ok(PowerAnalysisView { entry, result })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut page = assemble(analyses);
    let snapshot = ws.snapshot()?;
    let shown: Vec<_> = page
        .analyses
        .iter()
        .map(|a| (&a.entry, &a.result))
        .collect();
    page.flow = crate::flow::flow(&snapshot, case, &shown);
    // モードは設計の表示順（fune.toml の `[modes] order`）に並べる。一覧に無いものは後ろ
    let order: Vec<String> = snapshot
        .design
        .ordered_modes()
        .iter()
        .map(|m| m.id.to_string())
        .collect();
    page.modes.sort_by_key(|m| {
        order
            .iter()
            .position(|id| *id == m.id)
            .unwrap_or(usize::MAX)
    });
    Ok(page)
}

fn assemble(analyses: Vec<PowerAnalysisView>) -> PowerPageView {
    let mut modes: Vec<PowerModeView> = Vec::new();
    let mut orbital = Vec::new();
    for analysis in &analyses {
        let r = &analysis.result;
        let Some(page) = presentation::build(&r.id, r.input.as_ref(), r.output.as_ref()) else {
            continue;
        };
        if r.id == "power.orbit_generation" {
            orbital = page.modes;
            continue;
        }
        for incoming in page.modes {
            if let Some(mode) = modes.iter_mut().find(|m| m.id == incoming.id) {
                merge_mode(mode, incoming);
            } else {
                modes.push(incoming);
            }
        }
    }
    // 電力収支が使えない場合でも、取得できた軌道発電のグラフを表示する。
    if modes.is_empty() {
        modes = orbital;
    } else if let Some(orbit) = orbital.first() {
        for mode in &mut modes {
            if mode.charts.is_empty() {
                mode.charts.clone_from(&orbit.charts);
            }
        }
    }
    let verdicts = presentation::worst_verdicts(&modes);
    PowerPageView {
        modes,
        verdicts,
        analyses,
        flow: Default::default(),
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
    mode.bars.extend(incoming.bars);
    if !incoming.charts.is_empty() {
        mode.charts = incoming.charts;
    }
}

#[cfg(test)]
mod tests;
