//! カードと行の表示。部品の値は `query::values::value_view`、設定は `query::settings::setting_view`、書かれた量は
//! `query::values::authored_quantity`、解析の主な出力は KPI（`AnalysisRunResult.kpis`）。表示の文字列はどれも
//! `sat_units::format`（service の `units`）で作ったもの。

use super::cards::CardKey;
use crate::dto::{AnalysisEntry, AnalysisRunResult, FlowNode, FlowNodeKind, FlowRow};
use crate::query::values::{authored_quantity, value_view};
use crate::workspace::Snapshot;
use fune_model::Target;
use fune_model::path::SettingPath;
use sat_analysis::spec::AnalysisArea;
use std::collections::BTreeMap;

/// 読んだ行の ID → カードに出した行の ID（設定の表は子ごとの行に分ける）。
pub(super) type Shown = BTreeMap<String, Vec<String>>;

/// カード 1 枚と、読んだ行がどの行に出たか。
pub(super) fn card(s: &Snapshot, key: &CardKey, rows: &[(String, Target)]) -> (FlowNode, Shown) {
    let mut node = match key {
        CardKey::Part(id) => part_node(s, id),
        CardKey::Setting(group, file) => setting_node(group, file),
        CardKey::Modes => node(
            key,
            FlowNodeKind::Modes,
            ("運用モード", "Operating modes"),
            Some(Target::Modes),
        ),
        CardKey::Case(id) => case_node(s, key, id),
    };
    let mut shown = Shown::new();
    for (row, target) in rows {
        let made = match key {
            CardKey::Part(id) => vec![part_row(s, id, row, target)],
            CardKey::Setting(..) => setting_rows(s, row, target),
            CardKey::Modes => vec![mode_row(s, row, target)],
            CardKey::Case(id) => vec![case_row(s, id, row, target)],
        };
        shown.insert(row.clone(), made.iter().map(|r| r.id.clone()).collect());
        node.rows.extend(made);
    }
    if matches!(key, CardKey::Setting(g, _) if *g == SettingPath::conditions(&["orbit"])) {
        node.rows.extend(orbit_period(s));
    }
    (node, shown)
}

/// 解析のカード（行は KPI）。
pub(super) fn analysis(e: &AnalysisEntry, r: &AnalysisRunResult) -> FlowNode {
    let rows = r
        .kpis
        .iter()
        .map(|k| FlowRow {
            id: serde_json::to_value(k.key)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default(),
            label: k.label.clone(),
            label_en: k.label_en.clone(),
            display: k.value.as_ref().map(|v| v.display.clone()),
            set: k.value.is_some(),
            derived: false,
            verdict: k.verdict,
            target: None,
        })
        .collect();
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
        rows,
        analysis: Some(e.info.id.clone()),
        verdict: r.verdict,
        status: Some(r.status),
    }
}

fn node(
    key: &CardKey,
    kind: FlowNodeKind,
    (ja, en): (&str, &str),
    target: Option<Target>,
) -> FlowNode {
    FlowNode {
        id: key.node_id(),
        kind,
        label: ja.to_owned(),
        label_en: en.to_owned(),
        sub: None,
        system: None,
        target,
        rows: Vec::new(),
        analysis: None,
        verdict: None,
        status: None,
    }
}

fn row(id: &str, label: (String, String), display: Option<String>, target: &Target) -> FlowRow {
    FlowRow {
        id: id.to_owned(),
        set: display.is_some(),
        label: label.0,
        label_en: label.1,
        display,
        derived: false,
        verdict: None,
        target: Some(target.clone()),
    }
}

fn part_node(s: &Snapshot, id: &str) -> FlowNode {
    let key = CardKey::Part(id.to_owned());
    let p = s.resolved.parts.get(id);
    let label = p
        .and_then(|p| p.label.clone())
        .unwrap_or_else(|| id.to_owned());
    let mut n = node(
        &key,
        FlowNodeKind::Part,
        (&label, id),
        Some(Target::Part { id: id.to_owned() }),
    );
    n.sub = p.and_then(|p| p.kind.clone());
    n.system = p
        .and_then(|p| p.system)
        .map(fune_projection::system::to_analysis);
    n
}

fn part_row(s: &Snapshot, part: &str, attr: &str, target: &Target) -> FlowRow {
    let display = s
        .resolved
        .parts
        .get(part)
        .and_then(|p| p.attr.get(attr))
        .and_then(|rv| value_view(attr, rv).display);
    let def = s.design.attribute(attr);
    let label = def.map_or_else(|| attr.to_owned(), |d| d.label.clone());
    let label_en = def
        .and_then(|d| d.label_en.clone())
        .unwrap_or_else(|| attr.to_owned());
    row(attr, (label, label_en), display, target)
}

fn setting_node(group: &SettingPath, file: &str) -> FlowNode {
    node(
        &CardKey::Setting(group.clone(), file.to_owned()),
        FlowNodeKind::Setting,
        (file, file),
        Some(group.clone().into()),
    )
}

/// 設定の行。葉ならその値、子が葉だけの表なら 1 行（値を「 · 」でつなぐ）、子が表なら子ごとの行（モードの並びなど）。
fn setting_rows(s: &Snapshot, row_id: &str, target: &Target) -> Vec<FlowRow> {
    let Target::Param(fune_model::path::ParamPath::Setting(path)) = target else {
        return Vec::new();
    };
    let entries: Vec<_> = s
        .design
        .setting_entries()
        .into_iter()
        .filter(|e| e.path.scope == path.scope && e.path.segments.starts_with(&path.segments))
        .collect();
    let depth = path.segments.len();
    let nested = entries.iter().any(|e| e.path.segments.len() > depth + 1);
    let label = |id: &str| (id.to_owned(), id.to_owned());
    if !nested {
        return vec![row(
            row_id,
            label(row_id),
            joined(s, &entries, depth),
            target,
        )];
    }
    // 子の並びは書かれた順（モードの並びは区間の順）
    let mut children: Vec<(String, Vec<_>)> = Vec::new();
    for e in entries {
        let child = e.path.segments[depth].clone();
        match children.iter_mut().find(|(c, _)| *c == child) {
            Some((_, es)) => es.push(e),
            None => children.push((child, vec![e])),
        }
    }
    children
        .into_iter()
        .map(|(child, es)| {
            let id = format!("{row_id}.{child}");
            row(&id, label(&id), joined(s, &es, depth + 1), target)
        })
        .collect()
}

/// 設定の葉の表示をつなぐ。読んだパスそのものの葉は値だけ、その下の葉は「キー: 値」（深さ `depth` より下のキー）。
/// 書かれていない葉は飛ばし、どれも無ければ None。
fn joined(
    s: &Snapshot,
    entries: &[fune_model::design::SettingEntry],
    depth: usize,
) -> Option<String> {
    let parts: Vec<String> = entries
        .iter()
        .filter_map(|e| {
            let shown = crate::query::settings::setting_view(&s.design, e).display?;
            let key = e.path.segments.get(depth..).unwrap_or_default().join(".");
            Some(if key.is_empty() {
                shown
            } else {
                format!("{key}: {shown}")
            })
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

/// 軌道の周期（書かれた軌道から sat-astro が求める。軌道が組み立てられなければ出さない）。
fn orbit_period(s: &Snapshot) -> Option<FlowRow> {
    let ctx = fune_projection::Context::bare(&s.design, &s.resolved);
    let orbit = ctx.orbit(&mut fune_projection::MissingInputs::new())?;
    let q = crate::units::quantity_auto(orbit.period_s(), "Time");
    Some(FlowRow {
        id: "period".to_owned(),
        label: "周期".to_owned(),
        label_en: "Orbital period".to_owned(),
        display: Some(q.display),
        set: true,
        derived: true,
        verdict: None,
        target: None,
    })
}

fn mode_row(s: &Snapshot, id: &str, target: &Target) -> FlowRow {
    let mode = s.design.modes.get(id);
    let label = mode
        .and_then(|m| m.label.clone())
        .unwrap_or_else(|| id.to_owned());
    let duration = mode
        .and_then(|m| m.max_duration.as_ref())
        .map(|q| authored_quantity(q, Some("Time")).display);
    let mut r = row(id, (label, id.to_owned()), duration, target);
    r.set = mode.is_some();
    r
}

fn case_node(s: &Snapshot, key: &CardKey, id: &str) -> FlowNode {
    let label = s
        .design
        .cases
        .get(id)
        .and_then(|c| c.label.clone())
        .unwrap_or_else(|| id.to_owned());
    let mut n = node(key, FlowNodeKind::Case, (&label, id), None);
    n.target = Some(Target::case(id, None));
    n.sub = Some(id.to_owned());
    n
}

fn case_row(s: &Snapshot, id: &str, field: &str, target: &Target) -> FlowRow {
    let case = s.design.cases.get(id);
    let display = case.and_then(|c| match field {
        "mode" => c.mode.as_ref().map(|m| m.to_string()),
        "beta" => c
            .beta
            .as_ref()
            .map(|q| authored_quantity(q, Some("Angle")).display),
        "orbits" => c.orbits.map(|n| crate::units::number_text(f64::from(n))),
        _ => None,
    });
    row(field, (field.to_owned(), field.to_owned()), display, target)
}
