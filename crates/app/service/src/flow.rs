//! 解析のフロー図（`dto::FlowView`）。解析ごとに射影が読んだもの（`analysis::read_targets`。中身は
//! `fune_projection::ReadLog`）をカードと行に分け、値の行 → 解析のつながりを作る。解析 → 解析のつながりは、解析が
//! 中で結果を使う解析（`sat_analysis::catalog::uses`）と、射影が別の解析の結果を読んだもの（`Target::Analysis`）。
//! **つながりを手で書かない**（どの系の解析でも同じ組み立て）。カードの置き方は Web が決める。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `cards` | 読んだ対象 → どのカードのどの行か |
//! | `rows` | カードと行の表示（部品の値・設定・モード・ケース・解析の主な出力） |
//! | `group` | 部品をまとめる（同じ役割で、同じ属性を同じ解析が読む部品を 1 枚に） |
//! | `parts` | 部品のカードと行（まとめたカードの値は範囲か、値の違う部品の数） |
//! | `fold` | 畳める部品のまとまりと並び（同じ値を同じ解析が読む部品が多いとき） |

use crate::dto::{AnalysisEntry, AnalysisRunResult, FlowEdge, FlowEdgeKind, FlowNode, FlowView};
use crate::workspace::Snapshot;
use cards::CardKey;
use fune_model::Target;
use std::collections::{BTreeMap, BTreeSet};

mod cards;
mod fold;
mod group;
mod parts;
mod rows;
#[cfg(test)]
mod tests;

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
        .extend(analyses.iter().map(|(e, r)| rows::analysis(e, r)));
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
