//! フェーズ → 表示（並びの順。`Design::ordered_phases`）。軌道・姿勢則の上書きの葉は model の `entries_under`
//! （設定の葉の歩き方と同じ）で列挙し、値の表示は設定の表示（`query::settings`）と同じ。

use super::settings::value_data;
use super::values::{authored_quantity, display_of};
use super::views::file_of;
use crate::dto::{PhaseFieldView, PhaseLawView, PhaseText, PhaseView};
use fune_model::Target;
use fune_model::design::settings_view::entries_under;
use fune_model::design::{Design, EntityKey, Phase};
use fune_model::path::SettingPath;
use fune_model::value::{list_to_text, quantity_to_text, setting_to_text};
use std::collections::BTreeMap;

/// 編集の欄（`PhaseText` の欄）→ 対象のキー（`Target::phase` の `field`。検査が付ける対象と同じ）。
const FIELDS: &[(&str, &str)] = &[
    ("label", "label"),
    ("order", "order"),
    ("start", "start"),
    ("end", "end"),
    ("duration", "duration"),
    ("modes", "modes"),
    ("configuration", "configuration"),
];

/// フェーズの一覧（並びの順）。
pub(crate) fn phases(design: &Design) -> Vec<PhaseView> {
    design
        .ordered_phases()
        .into_iter()
        .zip(1u32..)
        .map(|(p, position)| view(design, p, position))
        .collect()
}

fn view(design: &Design, p: &Phase, position: u32) -> PhaseView {
    let id = p.id.as_str();
    PhaseView {
        id: id.to_owned(),
        position,
        order: p.order,
        label: p.label.clone().unwrap_or_else(|| id.to_owned()),
        start: p.start.clone(),
        end: p.end.clone(),
        duration: p
            .duration
            .as_ref()
            .map(|q| authored_quantity(q, Some("Time"))),
        modes: p.modes.iter().map(ToString::to_string).collect(),
        orbit_overridden: p.orbit.is_some(),
        orbit: fields("orbit", &serde_json::to_value(&p.orbit).unwrap_or_default()),
        laws: p
            .law
            .iter()
            .map(|(mode, setting)| PhaseLawView {
                mode: mode.clone(),
                fields: fields(
                    "law",
                    &serde_json::json!({ mode: serde_json::to_value(setting).unwrap_or_default() }),
                ),
                attitude: crate::attitude_view::phase_view(mode, setting),
            })
            .collect(),
        cases: design
            .cases
            .values()
            .filter(|c| c.phase.as_ref().is_some_and(|x| x.as_str() == id))
            .map(|c| c.id.to_string())
            .collect(),
        text: phase_text(p),
        field_paths: FIELDS
            .iter()
            .map(|(field, key)| {
                let target = Target::phase(id, Some(key));
                ((*field).to_owned(), target.to_string())
            })
            .collect::<BTreeMap<_, _>>(),
        file: file_of(design, EntityKey::Phase(id.to_owned())),
    }
}

/// 表（`orbit`・`law`）の書かれた葉（`orbit.altitude`・`law.NOMINAL.law`）。
fn fields(root: &str, json: &serde_json::Value) -> Vec<PhaseFieldView> {
    let base = SettingPath::conditions(&[root]);
    entries_under(&base, json)
        .into_iter()
        .filter_map(|e| {
            let value = e.value?;
            let data = value_data(&value);
            Some(PhaseFieldView {
                path: e.path.segments.join("."),
                key: e.path.segments.get(1..).unwrap_or_default().join("."),
                display: display_of(&data).unwrap_or_else(|| setting_to_text(&value)),
                value: data,
                text: setting_to_text(&value),
            })
        })
        .collect()
}

/// 今のフェーズ → 打った引数（`edit::phase_op` で読み直すと同じ見出しの欄になる。GUI の編集の欄の初期値）。
pub(crate) fn phase_text(p: &Phase) -> PhaseText {
    PhaseText {
        id: p.id.to_string(),
        label: p.label.clone(),
        order: (p.order != 0).then(|| p.order.to_string()),
        start: p.start.clone(),
        end: p.end.clone(),
        duration: p.duration.as_ref().map(quantity_to_text),
        modes: (!p.modes.is_empty()).then(|| list_to_text(p.modes.iter().map(|m| m.as_str()))),
        configuration: p.configuration.as_ref().map(ToString::to_string),
    }
}
