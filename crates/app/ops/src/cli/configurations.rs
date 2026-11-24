//! 形態の Op のコマンド（`fune configuration add|set|rm|rename|unset-pose`。旗は `fune_service` の `ConfigurationText` と 1 対 1）。
//! 形態の中への配置は `fune part place --configuration`（`commands::place`）。

use super::text::Cmd;
use crate::op::Op;

/// 形態の Op のコマンド。該当しなければ None。
pub(super) fn configuration_op(op: &Op) -> Option<Vec<String>> {
    Some(vec![match op {
        Op::CreateConfiguration {
            id,
            label,
            description,
        } => heading("add", id, label, description),
        Op::SetConfiguration {
            id,
            label,
            description,
        } => heading("set", id, label, description),
        Op::RemoveConfiguration { id } => Cmd::new(&["fune", "configuration", "rm", id]).done(),
        Op::RenameConfiguration { id, to } => {
            Cmd::new(&["fune", "configuration", "rename", id, to]).done()
        }
        Op::UnsetConfigurationPose {
            configuration,
            part,
        } => Cmd::new(&["fune", "configuration", "unset-pose", configuration, part]).done(),
        _ => return None,
    }])
}

/// `fune configuration add|set <ID> --label … --description …`
fn heading(verb: &str, id: &str, label: &Option<String>, description: &Option<String>) -> String {
    Cmd::new(&["fune", "configuration", verb, id])
        .flag("label", label.as_deref())
        .flag("description", description.as_deref())
        .done()
}
