//! `fune.toml` の `[project]` の Op（`SetProjectField`）の解釈。`name` は空にできない。`schema` と `toml` は書き換えない。

use crate::error::OpError;
use crate::op::Op;
use fune_model::design::ProjectField;
use fune_store::Edits;
use fune_store::write::project::set_project_field;

pub(super) fn interpret(edits: &mut Edits, op: &Op) -> Result<(), OpError> {
    let Op::SetProjectField { field } = op else {
        return Err(OpError::invalid("Not a project op"));
    };
    match field {
        ProjectField::Name(name) => check_text("The project name", name)?,
        ProjectField::Label(Some(label)) => check_text("The display name", label)?,
        ProjectField::Label(None) => {}
    }
    Ok(set_project_field(edits, field)?)
}

/// 空でなく、改行などの制御文字を含まない（1 行の文字列）。
fn check_text(what: &str, text: &str) -> Result<(), OpError> {
    if text.trim().is_empty() {
        return Err(OpError::invalid(format!("{what} is empty")));
    }
    if text.chars().any(char::is_control) {
        return Err(OpError::invalid(format!(
            "{what} must be a single line without control characters"
        )));
    }
    Ok(())
}
