//! フェーズの Op のコマンド（`fune phase add|set|rm|rename|set-field|unset-field|copy-orbit|unset-orbit|unset-law`・`fune case set-phase|unset-phase`）。

use super::text::Cmd;
use crate::op::Op;
use fune_model::design::Phase;
use fune_model::design::settings_view::entries_under;
use fune_model::path::SettingPath;
use fune_model::value::{list_to_text, quantity_to_text, setting_to_text};

/// フェーズの Op のコマンド（1 行以上）。該当しなければ None。
pub(super) fn phase_op(op: &Op) -> Option<Vec<String>> {
    Some(match op {
        Op::CreatePhase { phase } => with_fields("add", phase),
        Op::SetPhase { phase } => with_fields("set", phase),
        Op::RemovePhase { id } => vec![Cmd::new(&["fune", "phase", "rm", id]).done()],
        Op::RenamePhase { id, to } => vec![Cmd::new(&["fune", "phase", "rename", id, to]).done()],
        Op::SetPhaseField { phase, path, value } => vec![match value {
            Some(v) => Cmd::new(&["fune", "phase", "set-field", phase, path])
                .arg(&setting_to_text(v))
                .done(),
            None => Cmd::new(&["fune", "phase", "unset-field", phase, path]).done(),
        }],
        Op::CopyPhaseOrbit { phase } => {
            vec![Cmd::new(&["fune", "phase", "copy-orbit", phase]).done()]
        }
        Op::UnsetPhaseOrbit { phase } => {
            vec![Cmd::new(&["fune", "phase", "unset-orbit", phase]).done()]
        }
        Op::UnsetPhaseLaw { phase, mode } => {
            vec![Cmd::new(&["fune", "phase", "unset-law", phase, mode]).done()]
        }
        Op::SetCasePhase { case, phase } => vec![match phase {
            Some(p) => Cmd::new(&["fune", "case", "set-phase", case, p]).done(),
            None => Cmd::new(&["fune", "case", "unset-phase", case]).done(),
        }],
        _ => return None,
    })
}

/// 見出しの欄の `phase add|set` と、軌道・姿勢則の葉ごとの `phase set-field`（フェーズの `set` は軌道と姿勢則を今のまま残すので、
/// 葉を足して書く）。
fn with_fields(verb: &str, p: &Phase) -> Vec<String> {
    let mut lines = vec![phase_cmd(verb, p)];
    let leaves = |root: &str, json: serde_json::Value| {
        let root = SettingPath::conditions(&[root]);
        entries_under(&root, &json)
            .into_iter()
            .filter_map(|e| Some((e.path.segments.join("."), e.value?)))
            .collect::<Vec<_>>()
    };
    let orbit = serde_json::to_value(&p.orbit).unwrap_or_default();
    let law = serde_json::to_value(&p.law).unwrap_or_default();
    let all = leaves("orbit", orbit).into_iter().chain(leaves("law", law));
    for (path, value) in all {
        lines.push(
            Cmd::new(&["fune", "phase", "set-field", p.id.as_str(), &path])
                .arg(&setting_to_text(&value))
                .done(),
        );
    }
    lines
}

/// `fune phase add|set <ID> --label … --order 1 --start … --end … --duration "30 d" --modes SAFE,NOMINAL`
/// （旗は `fune_service` の `PhaseText` と 1 対 1）。
fn phase_cmd(verb: &str, p: &Phase) -> String {
    let order = (p.order != 0).then(|| p.order.to_string());
    let duration = p.duration.as_ref().map(quantity_to_text);
    let modes = (!p.modes.is_empty()).then(|| list_to_text(p.modes.iter().map(|m| m.as_str())));
    Cmd::new(&["fune", "phase", verb, p.id.as_str()])
        .flag("label", p.label.as_deref())
        .flag("order", order.as_deref())
        .flag("start", p.start.as_deref())
        .flag("end", p.end.as_deref())
        .flag("duration", duration.as_deref())
        .flag("modes", modes.as_deref())
        .flag(
            "configuration",
            p.configuration.as_ref().map(|id| id.as_str()),
        )
        .done()
}
