//! 部品の Op の解釈（前提の確認 → store の編集）。

use super::attr::{AttrTarget, unset_attr, write_attr};
use super::groups::{new_node_id, target_dirs};
use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::builtin::STATE_OFF;
use fune_model::design::{Mount, NodeRef, Part, Pose, StateAssign};
use fune_model::id::{Id, validate_local_name};
use fune_model::value::Value;
use fune_store::Edits;
use fune_store::layout::{self, part_path};
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
                return Err(OpError::not_found(format!("Group `{id}`")));
            }
            new_node_id(design, to)?;
            Ok(refs::rename_group(edits, design, id, to)?)
        }
        Op::MovePart { id, to, under } => move_part(edits, design, id, to.as_deref(), under),
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
        _ => Err(OpError::invalid("Not a part op")),
    }
}

/// `CreatePart` の Op から部品を組み立てて足す。
fn create_part_op(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    let Op::CreatePart {
        id,
        group,
        group_under,
        catalog,
        kind,
        count,
        label,
        mode_state,
    } = op
    else {
        return Err(OpError::invalid("Not a CreatePart op"));
    };
    let mut part = Part::new(id.as_str());
    part.catalog = catalog.clone();
    part.kind = kind.as_deref().map(Id::new);
    part.count = count.unwrap_or(1);
    part.label = label.clone();
    let target = (group.as_deref(), group_under.as_slice());
    create_part(edits, design, part, target, mode_state.as_deref())
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
        return Err(OpError::invalid("Not a Place op"));
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
    layout::part_file(design, id).ok_or_else(|| OpError::not_found(format!("Part `{id}`")))
}

fn create_part(
    edits: &mut Edits,
    design: &Design,
    part: Part,
    (group, group_under): (Option<&str>, &[String]),
    mode_state: Option<&str>,
) -> Result<(), OpError> {
    // `catalog` も `kind` も無い部品は検査（`part.no_definition`）の誤りになり、計画の最後に拒まれる
    new_node_id(design, part.id.as_str())?;
    let (_, dirs) = target_dirs(design, group, group_under)?;
    let file = part_path(&dirs, part.id.as_str());
    edits.create(&file, wpart::new_part_text(&part))?;
    let kind = design.part_kind_id(&part).map(str::to_string);
    let states = kind
        .as_deref()
        .map(|k| design.kind_states(k))
        .unwrap_or(&[]);
    let state = match mode_state {
        Some(s) => Some(s.to_string()),
        None if states.iter().any(|s| s == STATE_OFF) => Some(STATE_OFF.to_string()),
        None => states.first().cloned(),
    };
    if let Some(state) = state {
        for mode in design.modes.keys() {
            let assign = StateAssign::new(state.as_str());
            entities::set_mode_row(
                edits,
                &layout::mode_file(mode.as_str()),
                part.id.as_str(),
                false,
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
            "Part `{id}` is an assembly with child parts ({}) and cannot be deleted. Delete or move the children first",
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
    under: &[String],
) -> Result<(), OpError> {
    let old_file = existing_file(design, id)?;
    let (chain, dirs) = target_dirs(design, to, under)?;
    let node = NodeRef::Part(Id::new(id));
    let into_self = to == Some(id)
        || chain
            .iter()
            .any(|t| *t == node || design.ancestors(t).contains(&node));
    if into_self {
        return Err(OpError::invalid(format!(
            "Cannot move part `{id}` under itself"
        )));
    }
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
        .ok_or_else(|| OpError::not_found(format!("Part `{part_id}`")))?;
    let target = AttrTarget {
        file: &file,
        current: part.attr.get(attr).map(|a| &a.value),
        states: design.part_states(part),
        owner: part_id,
    };
    write_attr(edits, design, target, attr, state, value, from)
}
