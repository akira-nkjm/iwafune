//! 名詞のコマンドの引数 → Op（`fune part add` → `CreatePart` など）。綴りは `fune_ops::to_cli` と対になる。
//! 値の文字列の解釈が要るもの（配置・モードの時間割合・X,Y・設定の値）は `fune_service::edit` の関数を使う
//! （`fune part place`・`fune mode set|unset` は GUI と同じ打った引数の型にして渡す）。

use super::common::Built;
use crate::args::{EditOpts, ModeCmd, PartCmd, SettingCmd, ViewCmd};
use fune_service::Workspace;
use fune_service::dto::{ModeCellText, ModeSetText, Op, PlaceText, ViewNodePos};
use fune_service::edit::{self, SetText, UnsetText};
use std::collections::BTreeMap;

/// `fune part add|rm|mv|rename|rename-group|place`
pub fn part(c: &PartCmd) -> Built<'_> {
    let (op, e): (Op, &EditOpts) = match c {
        PartCmd::Add {
            id,
            catalog,
            kind,
            group,
            count,
            label,
            mode_state,
            edit,
        } => (
            Op::CreatePart {
                id: id.clone(),
                group: group.clone(),
                catalog: catalog.clone(),
                kind: kind.clone(),
                count: *count,
                label: label.clone(),
                mode_state: mode_state.clone(),
            },
            edit,
        ),
        PartCmd::Rm { id, edit } => (Op::RemovePart { id: id.clone() }, edit),
        PartCmd::Mv {
            id, group, edit, ..
        } => (
            Op::MovePart {
                id: id.clone(),
                to: group.clone(),
            },
            edit,
        ),
        PartCmd::Rename { id, to, edit } => (
            Op::RenamePart {
                id: id.clone(),
                to: to.clone(),
            },
            edit,
        ),
        PartCmd::RenameGroup { id, to, edit } => (
            Op::RenameGroup {
                id: id.clone(),
                to: to.clone(),
            },
            edit,
        ),
        PartCmd::Place(p) => (place_op(p)?, &p.edit),
        PartCmd::Point(p) => (super::common::point_op(p)?, &p.edit),
        PartCmd::Ls { .. } | PartCmd::Show { .. } => return Ok(None),
    };
    Ok(Some((op, e)))
}

/// `fune part place` → `Place`（位置と姿勢の文字列は GUI の配置の欄と同じ `edit::place_op` が読む）。
fn place_op(p: &crate::args::PlaceArgs) -> anyhow::Result<Op> {
    Ok(edit::place_op(&PlaceText {
        part: p.id.clone(),
        on: p.on.clone(),
        point: p.point.clone(),
        position: p.position.clone(),
        orientation: p.orientation.clone(),
    })?)
}

/// `fune mode set|unset` の 1 セル → `SetModeStates`（割合の読み方は GUI のモード表と同じ `edit::mode_set_op`）。
fn mode_cell_op(cell: ModeCellText) -> anyhow::Result<Op> {
    Ok(edit::mode_set_op(&ModeSetText { cells: vec![cell] })?)
}

/// `fune mode add|rm|rename|set|unset`
pub fn mode(c: &ModeCmd) -> Built<'_> {
    let (op, e) = match c {
        ModeCmd::Add {
            id,
            label,
            description,
            from,
            edit,
        } => (
            Op::CreateMode {
                id: id.clone(),
                label: label.clone(),
                description: description.clone(),
                copy_from: from.clone(),
            },
            edit,
        ),
        ModeCmd::Rm { id, edit } => (Op::RemoveMode { id: id.clone() }, edit),
        ModeCmd::Rename { id, to, edit } => (
            Op::RenameMode {
                id: id.clone(),
                to: to.clone(),
            },
            edit,
        ),
        ModeCmd::Set {
            mode,
            part,
            state,
            duty,
            edit,
        } => (
            mode_cell_op(ModeCellText {
                mode: mode.clone(),
                part: part.clone(),
                state: Some(state.clone()),
                duty: edit::duty_from_text(duty.as_deref().unwrap_or_default())?,
            })?,
            edit,
        ),
        ModeCmd::Unset { mode, part, edit } => (
            mode_cell_op(ModeCellText {
                mode: mode.clone(),
                part: part.clone(),
                state: None,
                duty: BTreeMap::new(),
            })?,
            edit,
        ),
        ModeCmd::Ls | ModeCmd::Table => return Ok(None),
    };
    Ok(Some((op, e)))
}

/// `fune setting set|unset`
pub fn setting<'a>(ws: &Workspace, c: &'a SettingCmd) -> Built<'a> {
    let (op, e) = match c {
        SettingCmd::Set {
            key,
            value,
            edit: e,
        } => {
            let text = SetText {
                path: key.clone(),
                value: value.clone(),
                ..Default::default()
            };
            (edit::set_op(ws, &text)?, e)
        }
        SettingCmd::Unset { key, edit } => (
            edit::unset_op(&UnsetText {
                item: None,
                path: key.clone(),
            })?,
            edit,
        ),
        SettingCmd::Ls => return Ok(None),
    };
    Ok(Some((op, e)))
}

/// `fune view set|rm`
pub fn view(c: &ViewCmd) -> Built<'_> {
    let (op, e) = match c {
        ViewCmd::Set {
            view,
            id,
            pos,
            size,
            edit,
        } => {
            // `--pos 420,120`・`--size 200,80` の読み方は service（`edit::xy_from_text`）
            let (x, y) = edit::xy_from_text(pos)?;
            let size = size.as_deref().map(edit::xy_from_text).transpose()?;
            let node = ViewNodePos {
                id: id.clone(),
                x,
                y,
                w: size.map(|s| s.0),
                h: size.map(|s| s.1),
            };
            (
                Op::SetViewLayout {
                    view: view.clone(),
                    nodes: vec![node],
                    remove: Vec::new(),
                },
                edit,
            )
        }
        ViewCmd::Rm { view, id, edit } => (
            Op::SetViewLayout {
                view: view.clone(),
                nodes: Vec::new(),
                remove: vec![id.clone()],
            },
            edit,
        ),
        ViewCmd::EdgeSet {
            view,
            key,
            points,
            source_side,
            target_side,
            source_offset,
            target_offset,
            edit,
        } => {
            let points = edit::points_from_text(points)?;
            (
                Op::SetViewEdge {
                    view: view.clone(),
                    key: key.clone(),
                    edge: fune_service::dto::ViewEdge {
                        points,
                        source_side: *source_side,
                        target_side: *target_side,
                        source_offset: *source_offset,
                        target_offset: *target_offset,
                    },
                },
                edit,
            )
        }
        ViewCmd::EdgeReset { view, key, edit } => (
            Op::UnsetViewEdge {
                view: view.clone(),
                key: key.clone(),
            },
            edit,
        ),
        ViewCmd::Reset { view, edit } => (Op::ResetView { view: view.clone() }, edit),
        ViewCmd::Ls { .. } => return Ok(None),
    };
    Ok(Some((op, e)))
}
