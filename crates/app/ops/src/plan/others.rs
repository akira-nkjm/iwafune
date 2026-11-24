//! モード・ネット・図・設定の Op の解釈（前提の確認 → store の編集）。

use super::check_keys::refuse_removing_checked_thermal_node;
use crate::error::OpError;
use crate::op::{ModeCell, ModePosition, Op, ViewNodePos};
use fune_model::Design;
use fune_model::design::{AnalysisSystem, Mode, ModeSegment, Net, StartRef, StateAssign, ViewNode};
use fune_model::id::{Id, clash, validate_entity_id};
use fune_model::path::{self, ParamPath, SettingPath, SettingScope};
use fune_model::value::SettingValue;
use fune_store::Edits;
use fune_store::layout;
use fune_store::write::{entities, project::set_mode_order, refs, sequences};

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::CreateMode {
            id,
            label,
            description,
            copy_from,
        } => create_mode(edits, design, id, label, description, copy_from.as_deref()),
        Op::RemoveMode { id } => {
            existing_mode(design, id)?;
            edits.remove(&layout::mode_file(id))?;
            if let Some(modes) = &design.project.modes {
                let order: Vec<_> = modes.order.iter().filter(|m| *m != id).cloned().collect();
                set_mode_order(edits, &order)?;
            }
            Ok(())
        }
        Op::RenameMode { id, to } => {
            existing_mode(design, id)?;
            new_flat_id(design.modes.keys().map(|k| k.as_str()), to, "Mode")?;
            refs::rename_mode(edits, design, id, to)?;
            if let Some(modes) = &design.project.modes {
                let order: Vec<_> = modes
                    .order
                    .iter()
                    .map(|m| if m == id { to.clone() } else { m.clone() })
                    .collect();
                set_mode_order(edits, &order)?;
            }
            Ok(())
        }
        Op::MoveMode { id, position } => move_mode(edits, design, id, position),
        Op::SetModeDuration { mode, duration } => {
            existing_mode(design, mode)?;
            Ok(entities::set_mode_duration(
                edits,
                &layout::mode_file(mode),
                duration.as_ref(),
            )?)
        }
        Op::SetModeTiming { mode, when, start } => {
            existing_mode(design, mode)?;
            Ok(sequences::set_mode_timing(
                edits,
                &layout::mode_file(mode),
                *when,
                start.as_ref(),
            )?)
        }
        Op::SetSequence {
            name,
            anchor,
            segments,
        } => set_sequence(edits, design, name, anchor.as_ref(), segments),
        Op::RemoveSequence { name } => remove_sequence(edits, design, name),
        Op::SetModeStates { cells } => cells.iter().try_for_each(|c| set_cell(edits, design, c)),
        Op::CreateNet { net } => {
            new_flat_id(
                design.nets.keys().map(|k| k.as_str()),
                net.id.as_str(),
                "Net",
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
                return Err(OpError::not_found(format!("Net `{net}`")));
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
        Op::SetSetting { path, value } => set_setting(edits, design, path, value.as_ref()),
        _ => Err(OpError::invalid("This op is not handled here")),
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
        return Err(OpError::invalid("The node id is empty"));
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

fn remove_net(edits: &mut Edits, design: &Design, id: &str) -> Result<(), OpError> {
    if !design.nets.contains_key(id) {
        return Err(OpError::not_found(format!("Net `{id}`")));
    }
    refs::forget_net(edits, design, id)?;
    edits.remove(&layout::net_file(id))?;
    Ok(())
}

fn set_setting(
    edits: &mut Edits,
    design: &Design,
    text: &str,
    value: Option<&SettingValue>,
) -> Result<(), OpError> {
    let ParamPath::Setting(setting) =
        path::parse(text).map_err(|e| OpError::invalid(e.to_string()))?
    else {
        return Err(OpError::invalid(format!(
            "`{text}` is not a setting path (it must start with conditions. / analysis. / budget.)"
        )));
    };
    if value.is_some()
        && let Some(moved) = fune_model::design::MovedSetting::of(&setting)
    {
        return Err(OpError::invalid(moved.refusal(&setting)));
    }
    require_station_for_setting(design, &setting)?;
    refuse_removing_checked_thermal_node(design, &setting, value.is_none())?;
    let (file, keys) = layout::setting_location(&setting)?;
    Ok(entities::set_setting(edits, &file, &keys, value)?)
}

/// `conditions.ground.<局>.…` の書き込みは、局が既にあるときだけ（局は `CreateGroundStation` だけが作る。ID の検査を通らない
/// ファイルができるのを防ぐ）。
fn require_station_for_setting(design: &Design, setting: &SettingPath) -> Result<(), OpError> {
    let s = &setting.segments;
    if setting.scope != SettingScope::Conditions || s.first().map(String::as_str) != Some("ground")
    {
        return Ok(());
    }
    match s.get(1) {
        Some(id) if !design.conditions.ground.contains_key(id) => Err(OpError::invalid(format!(
            "Ground station `{id}` does not exist. Create it first with `fune ground add {id}`"
        ))),
        _ => Ok(()),
    }
}

/// 名前付きの並びを書く（名前は ID の規則。区間は 1 つ以上・モードは存在すること。長さの次元は検査が見る）。
fn set_sequence(
    edits: &mut Edits,
    design: &Design,
    name: &str,
    anchor: Option<&StartRef>,
    segments: &[ModeSegment],
) -> Result<(), OpError> {
    validate_entity_id(name).map_err(|e| OpError::invalid(e.to_string()))?;
    if segments.is_empty() {
        return Err(OpError::invalid(format!(
            "Sequence `{name}` needs at least one segment"
        )));
    }
    for s in segments {
        existing_mode(design, &s.mode)?;
    }
    Ok(sequences::set_sequence(
        edits,
        &layout::analysis_file(AnalysisSystem::Power),
        name,
        anchor,
        segments,
    )?)
}

/// 名前付きの並びを消す。無ければ拒む。
fn remove_sequence(edits: &mut Edits, design: &Design, name: &str) -> Result<(), OpError> {
    let exists = design
        .analysis
        .power
        .as_ref()
        .is_some_and(|p| p.sequence(name).is_some());
    if !exists {
        return Err(OpError::not_found(format!("Sequence `{name}`")));
    }
    sequences::remove_sequence(edits, &layout::analysis_file(AnalysisSystem::Power), name)?;
    Ok(())
}

fn existing_mode(design: &Design, id: &str) -> Result<(), OpError> {
    if design.modes.contains_key(id) {
        Ok(())
    } else {
        Err(OpError::not_found(format!("Mode `{id}`")))
    }
}

fn existing_net(design: &Design, net: &Net) -> Result<(), OpError> {
    if design.nets.contains_key(net.id.as_str()) {
        Ok(())
    } else {
        Err(OpError::not_found(format!("Net `{}`", net.id)))
    }
}

/// 平らなディレクトリ（modes/・nets/）の新しい ID として使えるか（ID の規則と、大文字小文字を無視した一意性 `id::clash`）。
pub(super) fn new_flat_id<'a>(
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
    new_flat_id(design.modes.keys().map(|k| k.as_str()), id, "Mode")?;
    let states = match copy_from {
        Some(src) => design
            .modes
            .get(src)
            .map(|m| m.states.clone())
            .ok_or_else(|| OpError::not_found(format!("Source mode `{src}`")))?,
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
        when: copy_from
            .and_then(|src| design.modes.get(src))
            .and_then(|m| m.when),
        start: copy_from
            .and_then(|src| design.modes.get(src))
            .and_then(|m| m.start.clone()),
        eclipse: copy_from
            .and_then(|src| design.modes.get(src))
            .map(|m| m.eclipse.clone())
            .unwrap_or_default(),
    };
    edits.create(&layout::mode_file(id), entities::new_mode_text(&mode))?;
    if let Some(modes) = &design.project.modes {
        let mut order = modes.order.clone();
        order.retain(|m| m != id);
        order.push(id.to_owned());
        set_mode_order(edits, &order)?;
    }
    Ok(())
}

fn set_cell(edits: &mut Edits, design: &Design, cell: &ModeCell) -> Result<(), OpError> {
    existing_mode(design, &cell.mode)?;
    if !design.parts.contains_key(cell.part.as_str()) {
        return Err(OpError::not_found(format!("Part `{}`", cell.part)));
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
        return Err(OpError::invalid("The edge key is empty"));
    }
    Ok(())
}

/// いまの表示順から対象を抜き、移動先へ挿入する。
fn move_mode(
    edits: &mut Edits,
    design: &Design,
    id: &str,
    position: &ModePosition,
) -> Result<(), OpError> {
    existing_mode(design, id)?;
    if let ModePosition::Before(anchor) | ModePosition::After(anchor) = position {
        existing_mode(design, anchor)?;
        if anchor == id {
            return Err(OpError::invalid("The move target must be a different mode"));
        }
    }
    let mut order: Vec<String> = design
        .ordered_modes()
        .iter()
        .filter(|m| m.id.as_str() != id)
        .map(|m| m.id.to_string())
        .collect();
    let at = match position {
        ModePosition::First => 0,
        ModePosition::Last => order.len(),
        ModePosition::Before(anchor) | ModePosition::After(anchor) => {
            let index = order
                .iter()
                .position(|m| m == anchor)
                .ok_or_else(|| OpError::not_found(format!("Mode `{anchor}`")))?;
            index + usize::from(matches!(position, ModePosition::After(_)))
        }
    };
    order.insert(at, id.to_owned());
    // 手書きの欠けた参照は保存し、check の警告を維持する。
    order.extend(
        design
            .project
            .modes
            .iter()
            .flat_map(|m| &m.order)
            .filter(|m| !design.modes.contains_key(m.as_str()))
            .cloned(),
    );
    Ok(set_mode_order(edits, &order)?)
}
