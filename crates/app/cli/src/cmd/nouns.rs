//! 名詞のコマンドの引数 → Op（`fune part add` → `CreatePart` など）。綴りは `fune_ops::to_cli` と対になる。
//! 値の文字列の解釈が要るもの（ベクトル・量）は `fune_service::edit` の関数を使う。

use super::common::Built;
use crate::args::{EditOpts, ModeCmd, PartCmd, SettingCmd, ViewCmd};
use fune_service::Workspace;
use fune_service::dto::{ModeCell, Op, SettingValue, ViewNodePos};
use fune_service::edit::{self, SetText};
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

/// `fune part place` → `Place`（位置と姿勢の文字列は service が読む）。
fn place_op(p: &crate::args::PlaceArgs) -> anyhow::Result<Op> {
    let position = p
        .position
        .as_deref()
        .map(edit::vector_from_text)
        .transpose()?;
    let orientation = p
        .orientation
        .as_deref()
        .map(edit::quaternion_from_text)
        .transpose()?;
    Ok(Op::Place {
        part: p.id.clone(),
        on: p.on.clone(),
        point: p.point.clone(),
        position,
        orientation,
    })
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
            Op::SetModeStates {
                cells: vec![ModeCell {
                    mode: mode.clone(),
                    part: part.clone(),
                    state: Some(state.clone()),
                    duty: parse_duty(duty.as_deref())?,
                }],
            },
            edit,
        ),
        ModeCmd::Unset { mode, part, edit } => (
            Op::SetModeStates {
                cells: vec![ModeCell {
                    mode: mode.clone(),
                    part: part.clone(),
                    state: None,
                    duty: BTreeMap::new(),
                }],
            },
            edit,
        ),
        ModeCmd::Ls | ModeCmd::Table => return Ok(None),
    };
    Ok(Some((op, e)))
}

/// `--duty tx=0.1,rx=0.2` → 状態 → 割合。
fn parse_duty(text: Option<&str>) -> anyhow::Result<BTreeMap<String, f64>> {
    let mut out = BTreeMap::new();
    for pair in edit::list_from_text(text.unwrap_or_default()) {
        let (state, fraction) = pair
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("--duty は 状態=割合 の形です: {pair}"))?;
        out.insert(state.trim().to_owned(), fraction.trim().parse()?);
    }
    Ok(out)
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
            Op::SetSetting {
                path: key.clone(),
                value: None::<SettingValue>,
            },
            edit,
        ),
        SettingCmd::Ls => return Ok(None),
    };
    Ok(Some((op, e)))
}

/// `--pos 420,120` → (420, 120)。
fn pair(text: &str, what: &str) -> anyhow::Result<(i32, i32)> {
    let (a, b) = text
        .split_once(',')
        .ok_or_else(|| anyhow::anyhow!("{what} は X,Y の形です（例: 420,120）: {text}"))?;
    Ok((a.trim().parse()?, b.trim().parse()?))
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
            let (x, y) = pair(pos, "--pos")?;
            let size = size.as_deref().map(|s| pair(s, "--size")).transpose()?;
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
            let points = points
                .split_whitespace()
                .map(|p| pair(p, "--points").map(|(x, y)| [x, y]))
                .collect::<anyhow::Result<Vec<_>>>()?;
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
