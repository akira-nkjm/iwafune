//! `fune flow <系> [--case <ケース>]`: 解析のフロー図（`fune_service::flow::system_flow`。`GET /flow/{system}` と同じもの）。
//! `--json` は `FlowView` をそのまま出す。表示はカードごとの行の表と、つながりの表（カードの名前は日本語の表示名）。

use super::analyze::{kind_text, verdict_cell};
use crate::output::{C, Out};
use fune_service::Workspace;
use fune_service::dto::{FlowEdgeKind, FlowNode, FlowNodeKind, FlowRow, FlowView};

/// フロー図を出す（終了コードは 0。フロー図の無い系は service のエラー）。
pub fn run(ws: &Workspace, system: &str, case: Option<&str>, out: &Out) -> anyhow::Result<u8> {
    let system = fune_service::system_modes::parse_system(system)?;
    let view = fune_service::flow::system_flow(ws, system, case)?;
    if out.json {
        out.json(&view)?;
        return Ok(0);
    }
    for node in &view.nodes {
        print_card(node, out);
    }
    print_edges(&view, out);
    Ok(0)
}

/// カードの種類の言い方。
fn kind_name(kind: FlowNodeKind) -> &'static str {
    match kind {
        FlowNodeKind::Part => "部品",
        FlowNodeKind::Setting => "設定",
        FlowNodeKind::Modes => "運用モード",
        FlowNodeKind::Case => "解析ケース",
        FlowNodeKind::Analysis => "解析",
    }
}

/// カード 1 枚: 見出し（種類・名前・補足）と行の表。
fn print_card(node: &FlowNode, out: &Out) {
    let mut title = format!("{}: {}", kind_name(node.kind), node.label);
    if let Some(kind) = node.analysis_kind {
        title.push_str(&format!("（{}）", kind_text(kind)));
    }
    out.heading(&title);
    let members: Vec<&str> = node.members.iter().map(|m| m.id.as_str()).collect();
    let sub = if members.is_empty() {
        node.sub.clone()
    } else {
        Some(members.join(", "))
    };
    if let Some(sub) = sub {
        println!("{}", out.dim(sub));
    }
    if let Some(description) = &node.description {
        println!("{}", out.dim(description));
    }
    if !node.rows.is_empty() {
        let mut t = out.table(&["", "項目", "値"]).numeric(&[2]);
        for r in &node.rows {
            let mark = r.verdict.map_or(C::Dim(String::new()), verdict_cell);
            t.row(vec![mark, C::Text(r.label.clone()), value(r)]);
        }
        t.print();
    }
    println!();
}

/// 行の値（求めた値は印を添える。まとめたカードで部品ごとに違えば部品の数、書かれていなければ「未設定」）。
fn value(r: &FlowRow) -> C {
    match (&r.display, r.varies) {
        (Some(d), _) if r.derived => C::Num(format!("{d}（求めた値）")),
        (Some(d), _) => C::Num(d.clone()),
        (None, Some(n)) => C::Dim(format!("{n} 部品で違う")),
        (None, None) if r.set => C::Dim("—".into()),
        (None, None) => C::Dim("未設定".into()),
    }
}

/// 線の始まり（種類・始点のカード・始点の行）。
type Start<'a> = (FlowEdgeKind, &'a str, Option<&'a str>);

/// つながりの表（値の行 → 解析・解析 → 解析）。同じ始点の行から出る線は 1 行にまとめ、行き先を並べる。
fn print_edges(view: &FlowView, out: &Out) {
    if view.edges.is_empty() {
        return;
    }
    let node = |id: &str| view.nodes.iter().find(|n| n.id == id);
    let name = |id: &str| node(id).map_or_else(|| id.to_owned(), |n| n.label.clone());
    let row_name = |id: &str, row: &str| {
        node(id)
            .and_then(|n| n.rows.iter().find(|r| r.id == row))
            .map_or_else(|| row.to_owned(), |r| r.label.clone())
    };
    // (種類, 始点, 始点の行) → 行き先（線の並びのまま）
    let mut grouped: Vec<(Start<'_>, Vec<String>)> = Vec::new();
    for e in &view.edges {
        let key = (e.kind, e.source.as_str(), e.source_row.as_deref());
        match grouped.iter_mut().find(|(k, _)| *k == key) {
            Some((_, to)) => to.push(name(&e.target)),
            None => grouped.push((key, vec![name(&e.target)])),
        }
    }
    out.heading("つながり");
    let mut t = out.table(&["から", "値", "へ"]);
    for ((kind, source, row), to) in grouped {
        let row = match (kind, row) {
            (FlowEdgeKind::Input, Some(r)) => C::Text(row_name(source, r)),
            _ => C::Dim("（結果）".to_owned()),
        };
        t.row(vec![C::Strong(name(source)), row, C::Text(to.join("、"))]);
    }
    t.print();
}
