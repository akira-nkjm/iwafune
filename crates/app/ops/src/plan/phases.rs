//! フェーズ（`phases/<ID>.toml`）の Op の解釈。足す・書く・消す・名前を変える・軌道と姿勢則の値を書く・ケースのフェーズを書く。
//! 参照（モード・ケースのフェーズ）の実在は適用後の `check` が見る（消したフェーズをケースが指していれば拒まれる）。

use super::others::new_flat_id;
use super::records::{remove, set_id};
use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::design::{Case, Phase};
use fune_model::value::SettingValue;
use fune_store::Edits;
use fune_store::layout;
use fune_store::write::records::{PHASE_SECTIONS, write_record, write_record_with_sections};
use fune_store::write::{entities, refs};

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::CreatePhase { phase } => {
            let id = phase.id.as_str();
            if design.phases.contains_key(id) {
                return Err(OpError::exists(format!("Phase `{id}`")));
            }
            set_phase(edits, design, phase)
        }
        Op::SetPhase { phase } => set_phase(edits, design, phase),
        Op::RemovePhase { id } => remove(
            edits,
            design.phases.contains_key(id.as_str()),
            &layout::phase_file(id),
            "Phase",
            id,
        ),
        Op::RenamePhase { id, to } => {
            existing_phase(design, id)?;
            new_flat_id(design.phases.keys().map(|k| k.as_str()), to, "Phase")?;
            Ok(refs::rename_phase(edits, design, id, to)?)
        }
        Op::SetPhaseField { phase, path, value } => {
            set_phase_field(edits, design, phase, path, value.as_ref())
        }
        Op::SetCasePhase { case, phase } => set_case_phase(edits, design, case, phase.as_deref()),
        _ => Err(OpError::invalid("This op is not handled here")),
    }
}

fn existing_phase(design: &Design, id: &str) -> Result<(), OpError> {
    if design.phases.contains_key(id) {
        Ok(())
    } else {
        Err(OpError::not_found(format!("Phase `{id}`")))
    }
}

/// フェーズを書く。軌道（`orbit`）と姿勢則（`law`）が空なら、今のフェーズのものを残す（見出しの欄だけを書き換える
/// `fune phase set` と GUI の編集がこれを使う。軌道と姿勢則の値は `SetPhaseField` で書く）。
fn set_phase(edits: &mut Edits, design: &Design, p: &Phase) -> Result<(), OpError> {
    let id = p.id.as_str();
    set_id(design.phases.keys().map(|k| k.as_str()), id, "Phase")?;
    let current = design.phases.get(id);
    let merged = Phase {
        orbit: p
            .orbit
            .clone()
            .or_else(|| current.and_then(|c| c.orbit.clone())),
        law: if p.law.is_empty() {
            current.map(|c| c.law.clone()).unwrap_or_default()
        } else {
            p.law.clone()
        },
        ..p.clone()
    };
    Ok(write_record_with_sections(
        edits,
        &layout::phase_file(id),
        &merged,
        PHASE_SECTIONS,
    )?)
}

/// `orbit.<キー>` か `law.<モード>.<キー>` の値を 1 つ書く（先頭がそれ以外なら拒む）。
fn set_phase_field(
    edits: &mut Edits,
    design: &Design,
    phase: &str,
    path: &str,
    value: Option<&SettingValue>,
) -> Result<(), OpError> {
    existing_phase(design, phase)?;
    let keys: Vec<String> = path.split('.').map(str::to_owned).collect();
    let min = match keys.first().map(String::as_str) {
        Some("orbit") => 2,
        Some("law") => 3,
        _ => {
            return Err(OpError::invalid(format!(
                "`{path}` is not a phase field path (it must start with orbit. or law.<mode>.)"
            )));
        }
    };
    if keys.len() < min || keys.iter().any(String::is_empty) {
        return Err(OpError::invalid(format!(
            "`{path}` is not a phase field path (orbit.<key> or law.<mode>.<key>)"
        )));
    }
    Ok(entities::set_setting(
        edits,
        &layout::phase_file(phase),
        &keys,
        value,
    )?)
}

/// ケースの `phase` だけを書き換える（ケースのファイルを値に合わせる。ほかの欄は今のまま）。
fn set_case_phase(
    edits: &mut Edits,
    design: &Design,
    case: &str,
    phase: Option<&str>,
) -> Result<(), OpError> {
    let Some(current) = design.cases.get(case) else {
        return Err(OpError::not_found(format!("Analysis case `{case}`")));
    };
    let updated = Case {
        phase: phase.map(Into::into),
        ..current.clone()
    };
    Ok(write_record(edits, &layout::case_file(case), &updated)?)
}
