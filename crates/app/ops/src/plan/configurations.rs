//! 形態（`configurations/<ID>.toml`）の Op の解釈。足す・見出しを書く・消す・名前を変える・部品の姿勢の上書きを消す。
//! 形態の中への配置は部品の配置と同じ `Op::Place`（`configuration` つき）で、`parts::place_op` が [`place_in`] を呼ぶ。
//! 部品の存在・`[pose.<部品>]` のキーが部品であることは適用後の `check`（`ref.part`）が見る。

use super::others::new_flat_id;
use super::records::{remove, set_id};
use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::design::{Configuration, Pose};
use fune_model::value::{Quaternion, Vector3};
use fune_store::Edits;
use fune_store::layout;
use fune_store::write::records::{CONFIGURATION_SECTIONS, write_record_with_sections};
use fune_store::write::refs;

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::CreateConfiguration {
            id,
            label,
            description,
        } => {
            if design.configurations.contains_key(id.as_str()) {
                return Err(OpError::exists(format!("Configuration `{id}`")));
            }
            set_heading(edits, design, id, label, description)
        }
        Op::SetConfiguration {
            id,
            label,
            description,
        } => set_heading(edits, design, id, label, description),
        Op::RemoveConfiguration { id } => remove(
            edits,
            design.configurations.contains_key(id.as_str()),
            &layout::configuration_file(id),
            "Configuration",
            id,
        ),
        Op::RenameConfiguration { id, to } => {
            existing(design, id)?;
            new_flat_id(
                design.configurations.keys().map(|k| k.as_str()),
                to,
                "Configuration",
            )?;
            Ok(refs::rename_configuration(edits, design, id, to)?)
        }
        Op::UnsetConfigurationPose {
            configuration,
            part,
        } => unset_pose(edits, design, configuration, part),
        _ => Err(OpError::invalid("This op is not handled here")),
    }
}

fn existing<'a>(design: &'a Design, id: &str) -> Result<&'a Configuration, OpError> {
    design
        .configurations
        .get(id)
        .ok_or_else(|| OpError::not_found(format!("Configuration `{id}`")))
}

/// 形態のファイルを今のまま（部品ごとの姿勢の表も含めて）書き直す。
fn rewrite(edits: &mut Edits, c: &Configuration) -> Result<(), OpError> {
    Ok(write_record_with_sections(
        edits,
        &layout::configuration_file(c.id.as_str()),
        c,
        CONFIGURATION_SECTIONS,
    )?)
}

/// 見出し（`label`・`description`）を書く。あれば今の `[pose.*]` を残し、無ければ作る。
fn set_heading(
    edits: &mut Edits,
    design: &Design,
    id: &str,
    label: &Option<String>,
    description: &Option<String>,
) -> Result<(), OpError> {
    set_id(
        design.configurations.keys().map(|k| k.as_str()),
        id,
        "Configuration",
    )?;
    let mut c = design
        .configurations
        .get(id)
        .cloned()
        .unwrap_or_else(|| Configuration {
            id: id.into(),
            ..Configuration::default()
        });
    c.label = label.clone();
    c.description = description.clone();
    rewrite(edits, &c)
}

/// 形態の中への配置の中身（`Op::Place` の `configuration` つきの欄）。
pub(super) struct Placement<'a> {
    pub part: &'a str,
    pub on: Option<&'a str>,
    pub point: Option<&'a str>,
    pub position: &'a Option<Vector3>,
    pub orientation: &'a Option<Quaternion>,
}

/// 形態の `[pose.<部品>]` へ姿勢を書く（部品の `[pose]` は変えない）。取付先・取付点は部品の取付のまま変えられない
/// （書かれていれば部品の取付と同じであること）。取付の無い部品（根）の姿勢は機体座標での姿勢で、これも書ける。
pub(super) fn place_in(
    edits: &mut Edits,
    design: &Design,
    configuration: &str,
    p: &Placement,
) -> Result<(), OpError> {
    let current = existing(design, configuration)?;
    let part = design
        .parts
        .get(p.part)
        .ok_or_else(|| OpError::not_found(format!("Part `{}`", p.part)))?;
    let base_on = part.mount.as_ref().map(|m| m.on.as_str());
    let base_point = part.mount.as_ref().and_then(|m| m.point.as_deref());
    let differs = |given: Option<&str>, base: Option<&str>| given.is_some_and(|g| Some(g) != base);
    if differs(p.on, base_on) || differs(p.point, base_point) {
        let base = match (base_on, base_point) {
            (Some(on), Some(point)) => format!("on `{on}`, point `{point}`"),
            (Some(on), None) => format!("on `{on}`"),
            (None, _) => "none, a root of the vehicle".to_owned(),
        };
        return Err(OpError::invalid(format!(
            "A configuration overrides only the pose of part `{}`, not its mount (base mount: {base}). Change the mount without --configuration",
            p.part
        )));
    }
    if p.position.is_none() && p.orientation.is_none() {
        return Err(OpError::invalid(
            "Give a position or an orientation for the pose in the configuration (to go back to the base pose, remove the override)",
        ));
    }
    let mut c = current.clone();
    c.pose.insert(
        p.part.into(),
        Pose {
            position: p.position.clone(),
            orientation: p.orientation.clone(),
        },
    );
    rewrite(edits, &c)
}

/// 形態での部品の姿勢の上書きを消す（無ければ拒む）。
fn unset_pose(
    edits: &mut Edits,
    design: &Design,
    configuration: &str,
    part: &str,
) -> Result<(), OpError> {
    let mut c = existing(design, configuration)?.clone();
    if c.pose.remove(part).is_none() {
        return Err(OpError::not_found(format!(
            "Pose override of part `{part}` in configuration `{configuration}`"
        )));
    }
    rewrite(edits, &c)
}
