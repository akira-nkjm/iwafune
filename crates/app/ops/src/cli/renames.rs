//! 名前の変更・通信の回線・プロジェクト・ローカルのカタログの Op のコマンド（#330）。
//! 綴りは `fune_cli` の引数の定義（`net rename`・`link add|rm|rename`・`project set|unset`・`catalog source add-local`）と 1 対 1。

use super::text::Cmd;
use crate::op::Op;
use fune_model::design::ProjectField;

/// 該当しなければ None。
pub(super) fn rename_op(op: &Op) -> Option<Vec<String>> {
    Some(vec![match op {
        Op::RenameNet { id, to } => Cmd::new(&["fune", "net", "rename", id, to]).done(),
        Op::RenameHarness { id, to } => Cmd::new(&["fune", "harness", "rename", id, to]).done(),
        Op::RenameCase { id, to } => Cmd::new(&["fune", "case", "rename", id, to]).done(),
        Op::RenameSource { id, to } => Cmd::new(&["fune", "source", "rename", id, to]).done(),
        Op::RenameManeuver { id, to } => Cmd::new(&["fune", "maneuver", "rename", id, to]).done(),
        Op::CreateCommLink {
            id,
            direction,
            part,
            band,
        } => Cmd::new(&["fune", "link", "add", id])
            .flag("direction", Some(direction.as_str()))
            .flag("part", Some(part))
            .flag("band", Some(band))
            .done(),
        Op::RemoveCommLink { id } => Cmd::new(&["fune", "link", "rm", id]).done(),
        Op::RenameCommLink { id, to } => Cmd::new(&["fune", "link", "rename", id, to]).done(),
        Op::SetProjectField { field } => match field {
            ProjectField::Name(v) | ProjectField::Label(Some(v)) => {
                Cmd::new(&["fune", "project", "set", field.name()])
                    .arg(v)
                    .done()
            }
            ProjectField::Label(None) => {
                Cmd::new(&["fune", "project", "unset", field.name()]).done()
            }
        },
        Op::AddLocalCatalog { name, path } => {
            Cmd::new(&["fune", "catalog", "source", "add-local", name])
                .flag("path", Some(path))
                .done()
        }
        _ => return None,
    }])
}
