//! 解析のカード。行は解析の主な結果（`analysis::overview::main_kpis`。KPI を定義の順に 3 つまで。概要の解析の表と同じ）で、名前は KPI の表示名、値は KPI の表示
//! （判定を数える KPI は数の文）、判定は KPI の判定。どの系の解析でも同じ組み立て（解析ごとの分岐を書かない）。
//! 定常か時系列かの印と説明は解析の登録（`AnalysisInfo`）から。

use crate::dto::{AnalysisEntry, AnalysisRunResult, FlowNode, FlowNodeKind, FlowRow, KpiView};
use sat_analysis::spec::AnalysisArea;

/// 解析のカード（行は KPI）。
pub(super) fn node(e: &AnalysisEntry, r: &AnalysisRunResult) -> FlowNode {
    FlowNode {
        id: super::analysis_node_id(&e.info.id),
        kind: FlowNodeKind::Analysis,
        label: e.info.title.clone(),
        label_en: e.info.title_en.clone(),
        sub: Some(e.info.id.clone()),
        system: match e.info.area {
            AnalysisArea::System(s) => Some(s),
            AnalysisArea::Condition(_) => None,
        },
        target: None,
        rows: crate::analysis::overview::main_kpis(&r.kpis)
            .iter()
            .map(row)
            .collect(),
        analysis: Some(e.info.id.clone()),
        verdict: r.verdict,
        status: Some(r.status),
        analysis_kind: Some(e.info.kind),
        description: Some(e.info.description.clone()),
        description_en: Some(e.info.description_en.clone()),
        members: Vec::new(),
    }
}

/// KPI 1 つの行（ID は KPI のキーの綴り）。
fn row(k: &KpiView) -> FlowRow {
    let (display, display_en) = match (&k.value, k.tally) {
        (Some(v), _) => (Some(v.display.clone()), None),
        (None, Some(t)) if t.flagged == 0 => (
            Some(format!("すべて OK（{}）", t.checked)),
            Some(format!("All OK ({})", t.checked)),
        ),
        (None, Some(t)) => (
            Some(format!("{} / {} が要確認", t.flagged, t.checked)),
            Some(format!("{} of {} flagged", t.flagged, t.checked)),
        ),
        (None, None) => (None, None),
    };
    FlowRow {
        id: serde_json::to_value(k.key)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default(),
        label: k.label.clone(),
        label_en: k.label_en.clone(),
        // 出力はある（値が定まらない KPI は「—」。書かれていない値ではない）
        set: true,
        display,
        display_en,
        derived: false,
        varies: None,
        verdict: k.verdict,
        target: None,
    }
}
