//! 取付点・解析ケース・出典・ハーネスの Op のコマンド。

use super::text::Cmd;
use crate::op::Op;
use fune_model::design::{Case, Harness, Pose, Source};
use fune_model::value::{list_to_text, quantity_to_text, quaternion_to_text, vector_to_text};

/// 取付点・ケース・出典・ハーネスの Op のコマンド。該当しなければ None。
pub(super) fn record_op(op: &Op) -> Option<Vec<String>> {
    Some(vec![match op {
        Op::SetMountPoint { part, point, pose } => mount_point(part, point, pose.as_ref()),
        Op::SetCase { case } => case_cmd(case),
        Op::RemoveCase { id } => Cmd::new(&["fune", "case", "rm", id]).done(),
        Op::SetSource { source } => source_cmd(source),
        Op::RemoveSource { id } => Cmd::new(&["fune", "source", "rm", id]).done(),
        Op::SetHarness { harness } => harness_cmd(harness),
        Op::RemoveHarness { id } => Cmd::new(&["fune", "harness", "rm", id]).done(),
        _ => return None,
    }])
}

fn mount_point(part: &str, point: &str, pose: Option<&Pose>) -> String {
    let cmd = Cmd::new(&["fune", "part", "point", part, point]);
    let Some(pose) = pose else {
        return cmd.switch("rm", true).done();
    };
    let pos = pose.position.as_ref().map(vector_to_text);
    let ori = pose.orientation.as_ref().map(quaternion_to_text);
    cmd.flag("position", pos.as_deref())
        .flag("orientation", ori.as_deref())
        .done()
}

fn case_cmd(case: &Case) -> String {
    let beta = case.beta.as_ref().map(quantity_to_text);
    let orbits = case.orbits.map(|o| o.to_string());
    Cmd::new(&["fune", "case", "set", case.id.as_str()])
        .flag("label", case.label.as_deref())
        .flag("description", case.description.as_deref())
        .flag("mode", case.mode.as_ref().map(|m| m.as_str()))
        .flag("side", case.side.map(|s| s.as_str()))
        .flag("thermal-case", case.thermal_case.as_deref())
        .flag("beta", beta.as_deref())
        .flag("orbits", orbits.as_deref())
        .done()
}

fn source_cmd(source: &Source) -> String {
    Cmd::new(&["fune", "source", "set", source.id.as_str()])
        .flag("kind", Some(source.kind.as_str()))
        .flag("label", source.label.as_deref())
        .flag("ref", source.reference.as_deref())
        .flag("date", source.date.as_deref())
        .flag("by", source.by.as_deref())
        .flag("note", source.note.as_deref())
        .done()
}

fn harness_cmd(h: &Harness) -> String {
    let nets = (!h.nets.is_empty()).then(|| list_to_text(h.nets.iter().map(|n| n.as_str())));
    let cores = h.cores.map(|c| c.to_string());
    let length = h.length.as_ref().map(quantity_to_text);
    Cmd::new(&["fune", "harness", "set", h.id.as_str()])
        .flag("from", Some(&h.from.to_string()))
        .flag("to", Some(&h.to.to_string()))
        .flag("nets", nets.as_deref())
        .flag("wire", h.wire.as_deref())
        .flag("cores", cores.as_deref())
        .flag("length", length.as_deref())
        .flag("label", h.label.as_deref())
        .done()
}
