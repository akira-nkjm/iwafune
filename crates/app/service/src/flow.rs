//! 解析のフロー図（`dto::FlowView`）。解析ごとに射影が読んだもの（`analysis::read_targets`。中身は
//! `fune_projection::ReadLog`）をカードと行に分け、値の行 → 解析のつながりを作る。解析 → 解析のつながりは、解析が
//! 中で結果を使う解析（`sat_analysis::catalog::uses`）と、射影が別の解析の結果を読んだもの（`Target::Analysis`）。
//! **つながりを手で書かない**（どの系の解析でも同じ組み立て）。カードの置き方は Web が決める。
//!
//! 系のフロー図（[`system_flow`]。`GET /flow/{system}`・`fune flow <系>`）は、系の解析（[`flow_analyses`]）を回して組み立てる。
//! 電源のページ（`PowerPageView.flow`）は同じ解析の結果から [`flow`] を呼ぶ（解析を 2 回回さない）。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `cards` | 読んだ対象 → どのカードのどの行か |
//! | `rows` | 部品以外のカードと行の表示（設定・モード・ケース） |
//! | `analysis` | 解析のカード（行は KPI。定常か時系列かと説明） |
//! | `text` | 言語で変わる値の表示（列挙の表示名・範囲の「〜」と「–」） |
//! | `group` | 部品をまとめる（同じ役割で、同じ属性を同じ解析が読む部品を 1 枚に） |
//! | `parts` | 部品のカードと行（まとめたカードの値は範囲か、値の違う部品の数） |
//! | `fold` | 畳める部品のまとまりと並び（同じ値を同じ解析が読む部品が多いとき） |

use crate::dto::{
    AnalysisEntry, AnalysisRunResult, FlowEdge, FlowEdgeKind, FlowNode, FlowView,
    RunAnalysisRequest,
};
use crate::workspace::Snapshot;
use crate::{Result, ServiceError, Workspace};
use cards::CardKey;
use fune_model::Target;
use sat_analysis::spec::System;
use std::collections::{BTreeMap, BTreeSet};

mod analysis;
mod cards;
mod fold;
mod group;
mod parts;
mod rows;
#[cfg(test)]
mod tests;
mod text;

/// 電源のフロー図の解析（並びは解析のカードの並び）。
const POWER: &[&str] = &[
    "power.mode_load",
    "power.orbit_generation",
    "power.energy_balance",
    "power.sizing",
];

/// 系のフロー図に並べる解析（フロー図がまだ無い系は None）。**系ごとのフロー図の解析の一覧はここだけ**（電源のページも使う）。
pub(crate) fn flow_analyses(system: System) -> Option<&'static [&'static str]> {
    match system {
        System::Power => Some(POWER),
        _ => None,
    }
}

/// 系のフロー図（`GET /flow/{system}?case=…`、`fune flow <系>`）。系の解析を回し（ケースの解決は解析の実行と同じ）、
/// その結果から組み立てる。フロー図がまだ無い系は入力の誤り。
pub fn system_flow(ws: &Workspace, system: System, case: Option<&str>) -> Result<FlowView> {
    let ids = flow_analyses(system).ok_or_else(|| {
        let ready: Vec<&str> = fune_model::design::AnalysisSystem::ALL
            .iter()
            .map(|s| fune_projection::system::to_analysis(*s))
            .filter(|s| flow_analyses(*s).is_some())
            .map(crate::query::system_id)
            .collect();
        ServiceError::InvalidInput(format!(
            "系 {} のフロー図はまだありません（フロー図のある系: {}）",
            crate::query::system_id(system),
            ready.join(", ")
        ))
    })?;
    let analyses = run_all(ws, ids, case)?;
    let shown: Vec<_> = analyses.iter().map(|(e, r)| (e, r)).collect();
    Ok(flow(&ws.snapshot()?, case, &shown))
}

/// 解析を並びの順に回す（不足入力・失敗は結果のまま返す）。電源のページとフロー図が使う。
pub(crate) fn run_all(
    ws: &Workspace,
    ids: &[&str],
    case: Option<&str>,
) -> Result<Vec<(AnalysisEntry, AnalysisRunResult)>> {
    let req = RunAnalysisRequest {
        case: case.map(str::to_owned),
        ..Default::default()
    };
    let catalog = crate::analysis::analysis_catalog()?;
    ids.iter()
        .map(|id| {
            let entry = catalog
                .iter()
                .find(|e| e.info.id == *id)
                .cloned()
                .ok_or_else(|| ServiceError::not_found("解析", *id))?;
            let result = crate::analysis::run_analysis(ws, id, &req)?;
            Ok((entry, result))
        })
        .collect()
}

/// 解析の一覧（並びは解析のカードの並び）から、その解析が読んだ値と解析どうしのつながりの図を作る。
/// `case` は解析の実行と同じ明示のケース（省略時の選び方も同じ）。
pub(crate) fn flow(
    snapshot: &Snapshot,
    case: Option<&str>,
    analyses: &[(&AnalysisEntry, &AnalysisRunResult)],
) -> FlowView {
    let ids: Vec<&str> = analyses.iter().map(|(e, _)| e.info.id.as_str()).collect();
    let mut readers: BTreeMap<Target, BTreeSet<String>> = BTreeMap::new();
    let mut uses: BTreeSet<(String, String)> = BTreeSet::new();
    for id in &ids {
        for t in crate::analysis::read_targets(snapshot, id, case) {
            match t {
                Target::Analysis { id: from } => {
                    uses.insert((from, (*id).to_owned()));
                }
                t => {
                    readers.entry(t).or_default().insert((*id).to_owned());
                }
            }
        }
        for from in sat_analysis::catalog::uses(id) {
            uses.insert(((*from).to_owned(), (*id).to_owned()));
        }
    }
    let mut cards: BTreeMap<CardKey, Vec<(String, Target)>> = BTreeMap::new();
    for t in readers.keys() {
        if let Some((card, row)) = cards::locate(t) {
            cards.entry(card).or_default().push((row, t.clone()));
        }
    }
    let cards = group::group_parts(&snapshot.resolved, cards, &readers);
    let mut view = FlowView::default();
    let mut edges = Vec::new();
    let (order, folds) = fold::order(&snapshot.resolved, &cards, &readers);
    view.folds = folds;
    for key in order {
        let Some(rows) = cards.get(&key) else {
            continue;
        };
        let (node, shown) = rows::card(snapshot, &key, rows);
        for (row, target) in rows {
            for shown in shown.get(row).into_iter().flatten() {
                for analysis in readers.get(target).into_iter().flatten() {
                    edges.push(input_edge(&node, shown, analysis));
                }
            }
        }
        view.nodes.push(node);
    }
    view.nodes
        .extend(analyses.iter().map(|(e, r)| analysis::node(e, r)));
    for (from, to) in &uses {
        if ids.contains(&from.as_str()) && ids.contains(&to.as_str()) {
            edges.push(result_edge(from, to));
        }
    }
    view.edges = edges;
    view
}

/// 解析のカードの ID。
pub(crate) fn analysis_node_id(id: &str) -> String {
    format!("analysis:{id}")
}

fn input_edge(node: &FlowNode, row: &str, analysis: &str) -> FlowEdge {
    let target = analysis_node_id(analysis);
    FlowEdge {
        id: format!("{}#{row}>{target}", node.id),
        kind: FlowEdgeKind::Input,
        source: node.id.clone(),
        source_row: Some(row.to_owned()),
        target,
    }
}

fn result_edge(from: &str, to: &str) -> FlowEdge {
    let (source, target) = (analysis_node_id(from), analysis_node_id(to));
    FlowEdge {
        id: format!("{source}>{target}"),
        kind: FlowEdgeKind::Result,
        source,
        source_row: None,
        target,
    }
}
