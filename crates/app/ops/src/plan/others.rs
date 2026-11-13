//! 定義・モード・ネット・設定の Op の解釈（前提の確認 → store の編集）。

use crate::error::OpError;
use crate::op::{ModeCell, Op, ViewNodePos};
use fune_model::Design;
use fune_model::builtin;
use fune_model::check::attribute_def_problems;
use fune_model::design::{AttributeDef, KindDef, Mode, Net, StateAssign, ViewNode};
use fune_model::id::{Id, clash, validate_entity_id, validate_local_name};
use fune_model::path::{self, ParamPath};
use fune_model::value::SettingValue;
use fune_store::Edits;
use fune_store::layout;
use fune_store::write::{entities, refs};

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::CreateAttribute { def } => create_attribute(edits, design, def),
        Op::CreateKind { def } => create_kind(edits, design, def),
        Op::CreateMode {
            id,
            label,
            description,
            copy_from,
        } => create_mode(edits, design, id, label, description, copy_from.as_deref()),
        Op::RemoveMode { id } => {
            existing_mode(design, id)?;
            edits.remove(&layout::mode_file(id))?;
            Ok(())
        }
        Op::RenameMode { id, to } => {
            existing_mode(design, id)?;
            new_flat_id(design.modes.keys().map(|k| k.as_str()), to, "モード")?;
            Ok(refs::rename_mode(edits, design, id, to)?)
        }
        Op::SetModeDuration { mode, duration } => {
            existing_mode(design, mode)?;
            Ok(entities::set_mode_duration(
                edits,
                &layout::mode_file(mode),
                duration.as_ref(),
            )?)
        }
        Op::SetModeStates { cells } => cells.iter().try_for_each(|c| set_cell(edits, design, c)),
        Op::CreateNet { net } => {
            new_flat_id(
                design.nets.keys().map(|k| k.as_str()),
                net.id.as_str(),
                "ネット",
            )?;
            Ok(edits.create(
                &layout::net_file(net.id.as_str()),
                entities::new_net_text(net),
            )?)
        }
        Op::UpdateNet { net } => {
            existing_net(design, net)?;
            Ok(entities::write_net(
                edits,
                &layout::net_file(net.id.as_str()),
                net,
            )?)
        }
        Op::SetNetField { net, field } => {
            if !design.nets.contains_key(net.as_str()) {
                return Err(OpError::not_found(format!("ネット `{net}`")));
            }
            Ok(entities::set_net_field(
                edits,
                &layout::net_file(net),
                field,
            )?)
        }
        Op::RemoveNet { id } => remove_net(edits, design, id),
        Op::SetViewLayout {
            view,
            nodes,
            remove,
        } => set_view(edits, view, nodes, remove),
        Op::SetViewEdge { view, key, edge } => {
            validate_view_edge(view, key)?;
            Ok(entities::set_view_edge(
                edits,
                &layout::view_file(view),
                key,
                Some(edge),
            )?)
        }
        Op::UnsetViewEdge { view, key } => {
            validate_view_edge(view, key)?;
            Ok(entities::set_view_edge(
                edits,
                &layout::view_file(view),
                key,
                None,
            )?)
        }
        Op::ResetView { view } => {
            validate_entity_id(view).map_err(|e| OpError::invalid(e.to_string()))?;
            Ok(entities::reset_view(edits, &layout::view_file(view))?)
        }
        Op::SetSetting { path, value } => set_setting(edits, path, value.as_ref()),
        _ => Err(OpError::invalid("この Op はここでは扱いません")),
    }
}

fn set_view(
    edits: &mut Edits,
    view: &str,
    nodes: &[ViewNodePos],
    remove: &[String],
) -> Result<(), OpError> {
    validate_entity_id(view).map_err(|e| OpError::invalid(e.to_string()))?;
    if nodes
        .iter()
        .map(|n| n.id.as_str())
        .chain(remove.iter().map(String::as_str))
        .any(str::is_empty)
    {
        return Err(OpError::invalid("ノードの ID が空です"));
    }
    let rows: Vec<(String, ViewNode)> = nodes
        .iter()
        .map(|n| {
            (
                n.id.clone(),
                ViewNode {
                    x: n.x,
                    y: n.y,
                    w: n.w,
                    h: n.h,
                },
            )
        })
        .collect();
    Ok(entities::set_view_nodes(
        edits,
        &layout::view_file(view),
        &rows,
        remove,
    )?)
}

fn create_attribute(edits: &mut Edits, design: &Design, def: &AttributeDef) -> Result<(), OpError> {
    let id = def.id.as_str();
    if design.attribute(id).is_some() {
        return Err(OpError::exists(format!("属性 `{id}`")));
    }
    let problems = attribute_def_problems(def);
    if !problems.is_empty() {
        let text: Vec<String> = problems.into_iter().map(|(_, m)| m).collect();
        return Err(OpError::invalid(text.join(" / ")));
    }
    let file = layout::attribute_file(id);
    Ok(edits.create(&file, entities::new_attribute_text(&file, def)?)?)
}

fn create_kind(edits: &mut Edits, design: &Design, def: &KindDef) -> Result<(), OpError> {
    let id = def.id.as_str();
    validate_local_name(id).map_err(|e| OpError::invalid(e.to_string()))?;
    if design.kind(id).is_some() || builtin::is_builtin_kind(id) {
        return Err(OpError::exists(format!("種別 `{id}`")));
    }
    let file = layout::kind_file(id);
    Ok(edits.create(&file, entities::new_kind_text(&file, def)?)?)
}

fn remove_net(edits: &mut Edits, design: &Design, id: &str) -> Result<(), OpError> {
    if !design.nets.contains_key(id) {
        return Err(OpError::not_found(format!("ネット `{id}`")));
    }
    refs::forget_net(edits, design, id)?;
    edits.remove(&layout::net_file(id))?;
    Ok(())
}

fn set_setting(edits: &mut Edits, text: &str, value: Option<&SettingValue>) -> Result<(), OpError> {
    let ParamPath::Setting(setting) =
        path::parse(text).map_err(|e| OpError::invalid(e.to_string()))?
    else {
        return Err(OpError::invalid(format!(
            "`{text}` は設定のパスではありません（conditions. / analysis. / budget. で始めます）"
        )));
    };
    let (file, keys) = layout::setting_location(&setting)?;
    Ok(entities::set_setting(edits, &file, &keys, value)?)
}

fn existing_mode(design: &Design, id: &str) -> Result<(), OpError> {
    if design.modes.contains_key(id) {
        Ok(())
    } else {
        Err(OpError::not_found(format!("モード `{id}`")))
    }
}

fn existing_net(design: &Design, net: &Net) -> Result<(), OpError> {
    if design.nets.contains_key(net.id.as_str()) {
        Ok(())
    } else {
        Err(OpError::not_found(format!("ネット `{}`", net.id)))
    }
}

/// 平らなディレクトリ（modes/・nets/）の新しい ID として使えるか（ID の規則と、大文字小文字を無視した一意性 `id::clash`）。
fn new_flat_id<'a>(
    existing: impl Iterator<Item = &'a str>,
    id: &str,
    what: &str,
) -> Result<(), OpError> {
    validate_entity_id(id).map_err(|e| OpError::invalid(e.to_string()))?;
    if clash(existing, id).is_some() {
        return Err(OpError::exists(format!("{what} `{id}`")));
    }
    Ok(())
}

fn create_mode(
    edits: &mut Edits,
    design: &Design,
    id: &str,
    label: &Option<String>,
    description: &Option<String>,
    copy_from: Option<&str>,
) -> Result<(), OpError> {
    new_flat_id(design.modes.keys().map(|k| k.as_str()), id, "モード")?;
    let states = match copy_from {
        Some(src) => design
            .modes
            .get(src)
            .map(|m| m.states.clone())
            .ok_or_else(|| OpError::not_found(format!("写し元のモード `{src}`")))?,
        None => Default::default(),
    };
    let mode = Mode {
        id: Id::new(id),
        label: label.clone(),
        description: description.clone(),
        states,
        max_duration: copy_from
            .and_then(|src| design.modes.get(src))
            .and_then(|m| m.max_duration.clone()),
        eclipse: copy_from
            .and_then(|src| design.modes.get(src))
            .map(|m| m.eclipse.clone())
            .unwrap_or_default(),
    };
    Ok(edits.create(&layout::mode_file(id), entities::new_mode_text(&mode))?)
}

fn set_cell(edits: &mut Edits, design: &Design, cell: &ModeCell) -> Result<(), OpError> {
    existing_mode(design, &cell.mode)?;
    if !design.parts.contains_key(cell.part.as_str()) {
        return Err(OpError::not_found(format!("部品 `{}`", cell.part)));
    }
    let assign = cell.state.as_ref().map(|state| StateAssign {
        state: state.clone(),
        duty: cell.duty.clone(),
        active: cell.active,
        spare_state: cell.spare_state.clone(),
    });
    Ok(entities::set_mode_row(
        edits,
        &layout::mode_file(&cell.mode),
        &cell.part,
        cell.eclipse,
        assign.as_ref(),
    )?)
}

/// 図のファイル名と線のキー。経路の形（直交・端子の位置）は検査（`view.route`・`view.port_offset`）が見るので、
/// 形の悪い経路は計画の最後の検査で拒まれる。
fn validate_view_edge(view: &str, key: &str) -> Result<(), OpError> {
    validate_entity_id(view).map_err(|e| OpError::invalid(e.to_string()))?;
    if key.is_empty() {
        return Err(OpError::invalid("辺のキーが空です"));
    }
    Ok(())
}
