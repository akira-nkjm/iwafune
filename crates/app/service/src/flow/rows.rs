//! カードと行の表示。部品は `parts`、設定は `query::settings::setting_view`、書かれた量は
//! `query::values::authored_quantity`、解析の主な出力は KPI（`AnalysisRunResult.kpis`。電源のページは
//! `power_page::outputs` で差し替える）。表示の文字列はどれも `sat_units::format`（service の `units`）で作ったもの。
//! 設定とケースの欄の表示名は `fune_model::builtin::{setting_label, labels}`。

use super::cards::CardKey;
use crate::dto::{AnalysisEntry, AnalysisRunResult, FlowNode, FlowNodeKind, FlowRow};
use crate::query::values::authored_quantity;
use crate::workspace::Snapshot;
use fune_model::Target;
use fune_model::builtin::{labels, setting_label};
use fune_model::path::SettingPath;
use sat_analysis::spec::AnalysisArea;
use std::collections::BTreeMap;

/// 読んだ行の ID → カードに出した行の ID（設定の表は子ごとの行に分ける）。
pub(super) type Shown = BTreeMap<String, Vec<String>>;

/// カード 1 枚と、読んだ行がどの行に出たか。
pub(super) fn card(s: &Snapshot, key: &CardKey, rows: &[(String, Target)]) -> (FlowNode, Shown) {
    let mut node = match key {
        CardKey::Part(ids) => super::parts::node(s, key, ids),
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
            CardKey::Part(ids) => vec![super::parts::row(s, ids, row, target)],
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
            display_en: None,
            varies: None,
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
        members: Vec::new(),
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
        members: Vec::new(),
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
        display_en: None,
        varies: None,
        verdict: None,
        target: Some(target.clone()),
    }
}

/// 設定のカード（名前は設定のまとまりの表示名、補足はファイル）。
fn setting_node(group: &SettingPath, file: &str) -> FlowNode {
    let (ja, en) = setting_label(group)
        .filter(|(_, rest)| rest.is_empty())
        .map_or_else(
            || (file.to_owned(), file.to_owned()),
            |(l, _)| {
                (
                    l.label.clone(),
                    l.label_en.clone().unwrap_or_else(|| l.label.clone()),
                )
            },
        );
    let mut n = node(
        &CardKey::Setting(group.clone(), file.to_owned()),
        FlowNodeKind::Setting,
        (&ja, &en),
        Some(group.clone().into()),
    );
    n.sub = Some(file.to_owned());
    n
}

/// 設定のキーの表示名（`setting_label`。親の表示名に残りのキーを添えるとき、番号は 1 から）。
fn setting_row_label(path: &SettingPath) -> (String, String) {
    let Some((l, rest)) = setting_label(path) else {
        let key = path.segments.last().cloned().unwrap_or_default();
        return (key.clone(), key);
    };
    let rest: Vec<String> = rest
        .iter()
        .map(|k| {
            k.parse::<usize>()
                .map_or_else(|_| k.clone(), |i| (i + 1).to_string())
        })
        .collect();
    let with = |base: &str| {
        if rest.is_empty() {
            base.to_owned()
        } else {
            format!("{base} {}", rest.join(" "))
        }
    };
    (
        with(&l.label),
        with(l.label_en.as_deref().unwrap_or(&l.label)),
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
    if !nested {
        let label = setting_row_label(path);
        return vec![row(row_id, label, joined(s, &entries), target)];
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
            let label = setting_row_label(&path.child(&[child.as_str()]));
            row(&id, label, joined(s, &es), target)
        })
        .collect()
}

/// 設定の葉の表示を「 · 」でつなぐ（文字の値を先に。書かれていない葉は飛ばし、どれも無ければ None）。
fn joined(s: &Snapshot, entries: &[fune_model::design::SettingEntry]) -> Option<String> {
    let mut views: Vec<_> = entries
        .iter()
        .map(|e| crate::query::settings::setting_view(&s.design, e))
        .filter(|v| v.display.is_some())
        .collect();
    views.sort_by_key(|v| !matches!(v.value, crate::dto::ValueData::Text { .. }));
    let parts: Vec<String> = views.into_iter().filter_map(|v| v.display).collect();
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
        display_en: None,
        varies: None,
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
    let label = labels().case_field.get(field).map_or_else(
        || (field.to_owned(), field.to_owned()),
        |l| {
            (
                l.label.clone(),
                l.label_en.clone().unwrap_or_else(|| l.label.clone()),
            )
        },
    );
    row(field, label, display, target)
}
