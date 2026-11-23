//! `fune link add|rm|rename`・`fune project set|unset` の引数 → Op。綴りは `fune_ops::to_cli` と対になる。
//! 打った引数の読み方は GUI と同じ `fune_service::edit::{link_add_op, project_field_op}`。

use crate::args::{EditOpts, LinkCmd, ProjectCmd};
use fune_service::dto::{LinkAddText, Op, ProjectSetText};
use fune_service::edit;

/// `fune link add|rm|rename`
pub fn link(c: &LinkCmd) -> anyhow::Result<(Op, &EditOpts)> {
    Ok(match c {
        LinkCmd::Add(a) => (
            edit::link_add_op(&LinkAddText {
                id: a.id.clone(),
                direction: a.direction.clone(),
                part: a.part.clone(),
                band: a.band.clone(),
            })?,
            &a.edit,
        ),
        LinkCmd::Rm { id, edit } => (Op::RemoveCommLink { id: id.clone() }, edit),
        LinkCmd::Rename { id, to, edit } => (
            Op::RenameCommLink {
                id: id.clone(),
                to: to.clone(),
            },
            edit,
        ),
    })
}

/// `fune project set|unset`
pub fn project(c: &ProjectCmd) -> anyhow::Result<(Op, &EditOpts)> {
    let (field, value, opts) = match c {
        ProjectCmd::Set { field, value, edit } => (field, Some(value.clone()), edit),
        ProjectCmd::Unset { field, edit } => (field, None, edit),
    };
    let text = ProjectSetText {
        field: field.clone(),
        value,
    };
    Ok((edit::project_field_op(&text)?, opts))
}
