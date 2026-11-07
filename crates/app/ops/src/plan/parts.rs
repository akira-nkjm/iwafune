//! 部品の Op の解釈（前提の確認 → store の編集）。

use super::attr::{AttrTarget, unset_attr, write_attr};
use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::design::{Mount, NodeRef, Part, Pose, StateAssign};
use fune_model::id::{Id, fold, validate_local_name, validate_node_id};
use fune_model::value::Value;
use fune_store::Edits;
use fune_store::layout::{self, dirs_for_parent, part_path};
use fune_store::write::{entities, part as wpart, refs};

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::CreatePart { .. } => create_part_op(edits, design, op),
        Op::RemovePart { id } => remove_part(edits, design, id),
        Op::RenamePart { id, to } => {
            existing_file(design, id)?;
            new_node_id(design, to)?;
            Ok(refs::rename_part(edits, design, id, to)?)
        }
        Op::RenameGroup { id, to } => {
            if !design.groups.keys().any(|g| g.as_str() == id) {
                return Err(OpError::not_found(format!("グループ `{id}`")));
            }
            new_node_id(design, to)?;
            Ok(refs::rename_group(edits, design, id, to)?)
        }
        Op::MovePart { id, to } => move_part(edits, design, id, to.as_deref()),
        Op::SetValue {
            part,
            attr,
            state,
            value,
            from,
        } => set_value(
            edits,
            design,
            part,
            attr,
            state.as_deref(),
            value,
            from.clone(),
        ),
        Op::UnsetValue { part, attr, state } => {
            let file = existing_file(design, part)?;
            unset_attr(edits, &file, part, attr, state.as_deref())
        }
        Op::SetField { part, field } => {
            let file = existing_file(design, part)?;
            Ok(wpart::set_field(edits, &file, field)?)
        }
        Op::Place { .. } => place_op(edits, design, op),
        Op::SetPort { part, port, net } => {
            let file = existing_file(design, part)?;
            validate_local_name(port).map_err(|e| OpError::invalid(e.to_string()))?;
            Ok(wpart::set_port(edits, &file, port, net.as_deref())?)
        }
        Op::SetMountPoint { part, point, pose } => {
            let file = existing_file(design, part)?;
            validate_local_name(point).map_err(|e| OpError::invalid(e.to_string()))?;
            Ok(wpart::set_mount_point(edits, &file, point, pose.as_ref())?)
        }
        _ => Err(OpError::invalid("部品の Op ではありません")),
    }
}

/// `CreatePart` の Op から部品を組み立てて足す。
fn create_part_op(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    let Op::CreatePart {
        id,
        group,
        catalog,
        kind,
        count,
        label,
        mode_state,
    } = op
    else {
        return Err(OpError::invalid("CreatePart ではありません"));
    };
    let mut part = Part::new(id.as_str());
    part.catalog = catalog.clone();
    part.kind = kind.as_deref().map(Id::new);
    part.count = count.unwrap_or(1);
    part.label = label.clone();
    create_part(edits, design, part, group.as_deref(), mode_state.as_deref())
}

/// `Place` の Op（取付と姿勢を丸ごと置き換える）。
fn place_op(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    let Op::Place {
        part,
        on,
        point,
        position,
        orientation,
    } = op
    else {
        return Err(OpError::invalid("Place ではありません"));
    };
    let file = existing_file(design, part)?;
    let mount = on.as_ref().map(|on| Mount {
        on: Id::new(on),
        point: point.clone(),
    });
    let pose = (position.is_some() || orientation.is_some()).then(|| Pose {
        position: position.clone(),
        orientation: orientation.clone(),
    });
    Ok(wpart::place(edits, &file, mount.as_ref(), pose.as_ref())?)
}

/// 部品のファイル（無ければ NotFound）。
pub(super) fn existing_file(design: &Design, id: &str) -> Result<String, OpError> {
    layout::part_file(design, id).ok_or_else(|| OpError::not_found(format!("部品 `{id}`")))
}

/// 新しいノード（部品・グループ）の ID として使えるか（規則と、大文字小文字を無視した一意性）。
fn new_node_id(design: &Design, id: &str) -> Result<(), OpError> {
    validate_node_id(id).map_err(|e| OpError::invalid(e.to_string()))?;
    let taken = design
        .parts
        .keys()
        .map(|k| k.as_str())
        .chain(design.groups.keys().map(|k| k.as_str()))
        .any(|k| fold(k) == fold(id));
    if taken {
        return Err(OpError::exists(format!("ID `{id}`")));
    }
    Ok(())
}

/// 親の指定 → (親のノード, `parts/` の下のディレクトリの列)。無いグループは最上位に作る。
fn parent_dirs(
    design: &Design,
    parent: Option<&str>,
) -> Result<(Option<NodeRef>, Vec<String>), OpError> {
    let Some(name) = parent else {
        return Ok((None, Vec::new()));
    };
    match design.node(name) {
        Some(node) => {
            let dirs = dirs_for_parent(design, Some(&node));
            Ok((Some(node), dirs))
        }
        None => {
            new_node_id(design, name)?;
            Ok((None, vec![name.to_string()]))
        }
    }
}

fn create_part(
    edits: &mut Edits,
    design: &Design,
    part: Part,
    group: Option<&str>,
    mode_state: Option<&str>,
) -> Result<(), OpError> {
    new_node_id(design, part.id.as_str())?;
    if part.catalog.is_none() && part.kind.is_none() {
        return Err(OpError::invalid("部品には catalog か kind が要ります"));
    }
    let (_, dirs) = parent_dirs(design, group)?;
    let file = part_path(&dirs, part.id.as_str());
    edits.create(&file, wpart::new_part_text(&part))?;
    let kind = design.part_kind_id(&part).map(str::to_string);
    let states = kind
        .as_deref()
        .map(|k| design.kind_states(k))
        .unwrap_or(&[]);
    let state = match mode_state {
        Some(s) => Some(s.to_string()),
        None if states.iter().any(|s| s == "off") => Some("off".to_string()),
        None => states.first().cloned(),
    };
    if let Some(state) = state {
        for mode in design.modes.keys() {
            let assign = StateAssign::new(state.as_str());
            entities::set_mode_row(
                edits,
                &layout::mode_file(mode.as_str()),
                part.id.as_str(),
                Some(&assign),
            )?;
        }
    }
    Ok(())
}

fn remove_part(edits: &mut Edits, design: &Design, id: &str) -> Result<(), OpError> {
    let file = existing_file(design, id)?;
    let children: Vec<&str> = design
        .parts
        .values()
        .filter(|p| matches!(&p.parent, Some(NodeRef::Part(parent)) if parent.as_str() == id))
        .map(|p| p.id.as_str())
        .collect();
    if !children.is_empty() {
        return Err(OpError::invalid(format!(
            "部品 `{id}` は子部品（{}）を持つ組立品なので消せません。先に子部品を消すか移してください",
            children.join(", ")
        )));
    }
    edits.remove(&file)?;
    Ok(refs::forget_part(edits, design, id)?)
}

fn move_part(
    edits: &mut Edits,
    design: &Design,
    id: &str,
    to: Option<&str>,
) -> Result<(), OpError> {
    let old_file = existing_file(design, id)?;
    if let Some(target) = to {
        let node = NodeRef::Part(Id::new(id));
        let into_self = target == id
            || design
                .node(target)
                .is_some_and(|t| design.ancestors(&t).contains(&node));
        if into_self {
            return Err(OpError::invalid(format!(
                "部品 `{id}` を自分の下へは移せません"
            )));
        }
    }
    let (_, dirs) = parent_dirs(design, to)?;
    let new_file = part_path(&dirs, id);
    if new_file == old_file {
        return Ok(());
    }
    edits.rename(&old_file, &new_file)?;
    let old_dir = old_file.trim_end_matches(".toml");
    let new_dir = new_file.trim_end_matches(".toml");
    Ok(edits.rename_dir(old_dir, new_dir)?)
}

fn set_value(
    edits: &mut Edits,
    design: &Design,
    part_id: &str,
    attr: &str,
    state: Option<&str>,
    value: &Value,
    from: Option<fune_model::value::Provenance>,
) -> Result<(), OpError> {
    let file = existing_file(design, part_id)?;
    let part = design
        .parts
        .get(part_id)
        .ok_or_else(|| OpError::not_found(format!("部品 `{part_id}`")))?;
    let target = AttrTarget {
        file: &file,
        current: part.attr.get(attr).map(|a| &a.value),
        states: design.part_states(part),
        owner: part_id,
    };
    write_attr(edits, design, target, attr, state, value, from)
}
