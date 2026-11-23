//! 解析ケース・出典・ハーネス・マヌーバ・取付点の引数 → Op（`fune case set` → `SetCase` など）。綴りは `fune_ops::to_cli` と対になる。

use super::common::{Built, list};
use crate::args::{
    CaseArgs, CaseCmd, CatalogCmd, CatalogSourceCmd, GroundCmd, HarnessArgs, HarnessCmd,
    ManeuverArgs, ManeuverCmd, SourceArgs, SourceCmd,
};
use fune_service::dto::{
    Case, CatalogSourceAddText, GroundAddText, Harness, ManeuverText, Op, Source,
};
use fune_service::edit;

/// serde の綴り（snake_case）から列挙を読む。
fn enum_of<T: std::str::FromStr>(text: &str, what: &str) -> anyhow::Result<T> {
    text.parse()
        .map_err(|_| anyhow::anyhow!("cannot read as {what}: {text}"))
}

/// `fune case set|rm`
pub fn case(c: &CaseCmd) -> Built<'_> {
    Ok(match c {
        CaseCmd::Set(a) => Some((Op::SetCase { case: case_of(a)? }, &a.edit)),
        CaseCmd::SetPhase { id, phase, edit } => Some((edit::case_phase_op(id, Some(phase)), edit)),
        CaseCmd::UnsetPhase { id, edit } => Some((edit::case_phase_op(id, None), edit)),
        CaseCmd::Rm { id, edit } => Some((Op::RemoveCase { id: id.clone() }, edit)),
        CaseCmd::Ls | CaseCmd::FromEclipse { .. } | CaseCmd::FromSunDistance { .. } => None,
    })
}

fn case_of(a: &CaseArgs) -> anyhow::Result<Case> {
    let value = serde_json::json!({
        "label": a.label,
        "description": a.description,
        "mode": a.mode,
        "phase": a.phase,
        "side": a.side,
        "thermal_case": a.thermal_case,
        "orbits": a.orbits,
    });
    let mut case: Case = serde_json::from_value(value)
        .map_err(|e| anyhow::anyhow!("Cannot create the analysis case: {e}"))?;
    case.id = a.id.as_str().into();
    case.beta = a
        .beta
        .as_deref()
        .map(edit::quantity_from_text)
        .transpose()?;
    case.sun_distance = a
        .sun_distance
        .as_deref()
        .map(edit::quantity_from_text)
        .transpose()?;
    Ok(case)
}

/// `fune source set|rm|mv`
pub fn source(c: &SourceCmd) -> Built<'_> {
    Ok(match c {
        SourceCmd::Set(a) => Some((
            Op::SetSource {
                source: source_of(a)?,
                dir: a.subdir.clone(),
            },
            &a.edit,
        )),
        SourceCmd::Rm { id, edit } => Some((Op::RemoveSource { id: id.clone() }, edit)),
        SourceCmd::Mv {
            id,
            target,
            edit: opts,
        } => Some((
            Op::MoveSource {
                id: id.clone(),
                dir: target.clone(),
            },
            opts,
        )),
        SourceCmd::Ls => None,
    })
}

fn source_of(a: &SourceArgs) -> anyhow::Result<Source> {
    Ok(Source {
        id: a.id.as_str().into(),
        kind: enum_of(
            &a.kind,
            "source kind (placeholder / judgement / analysis / datasheet / measured)",
        )?,
        label: a.label.clone(),
        reference: a.reference.clone(),
        revision: a.revision.clone(),
        retrieved: a.retrieved.clone(),
        sha256: a.sha256.clone(),
        distribution: a
            .distribution
            .as_deref()
            .map(|d| enum_of(d, "distribution class (public / on_request / restricted)"))
            .transpose()?,
        date: a.date.clone(),
        by: a.by.clone(),
        note: a.note.clone(),
    })
}

/// `fune harness set|rm`
pub fn harness(c: &HarnessCmd) -> Built<'_> {
    Ok(match c {
        HarnessCmd::Set(a) => Some((
            Op::SetHarness {
                harness: harness_of(a)?,
            },
            &a.edit,
        )),
        HarnessCmd::Rm { id, edit } => Some((Op::RemoveHarness { id: id.clone() }, edit)),
        HarnessCmd::Ls => None,
    })
}

fn harness_of(a: &HarnessArgs) -> anyhow::Result<Harness> {
    Ok(Harness {
        id: a.id.as_str().into(),
        label: a.label.clone(),
        from: a.from.parse()?,
        to: a.to.parse()?,
        nets: list(&a.nets)
            .into_iter()
            .map(|n| n.as_str().into())
            .collect(),
        wire: a.wire.clone(),
        cores: a.cores,
        length: a
            .length
            .as_deref()
            .map(edit::quantity_from_text)
            .transpose()?,
    })
}

/// `fune maneuver add|set|rm`（打った値を読むのは service の `edit::maneuver_op`）
pub fn maneuver(c: &ManeuverCmd) -> Built<'_> {
    Ok(match c {
        ManeuverCmd::Add(a) => Some((edit::maneuver_op(&maneuver_text(a), true)?, &a.edit)),
        ManeuverCmd::Set(a) => Some((edit::maneuver_op(&maneuver_text(a), false)?, &a.edit)),
        ManeuverCmd::Rm { id, edit } => Some((Op::RemoveManeuver { id: id.clone() }, edit)),
        ManeuverCmd::Ls => None,
    })
}

/// `fune ground add|rm|rename` → `CreateGroundStation`（引数の読み方は service）・`RemoveGroundStation`・`RenameGroundStation`。
pub fn ground(c: &GroundCmd) -> anyhow::Result<(Op, &crate::args::EditOpts)> {
    Ok(match c {
        GroundCmd::Add(a) => (
            edit::ground_add_op(&GroundAddText {
                id: a.id.clone(),
                label: a.label.clone(),
                latitude: a.latitude.clone(),
                longitude: a.longitude.clone(),
                altitude: a.altitude.clone(),
                min_elevation: a.min_elevation.clone(),
            })?,
            &a.edit,
        ),
        GroundCmd::Rm { id, edit } => (Op::RemoveGroundStation { id: id.clone() }, edit),
        GroundCmd::Rename { id, to, edit } => (
            Op::RenameGroundStation {
                id: id.clone(),
                to: to.clone(),
            },
            edit,
        ),
    })
}

fn maneuver_text(a: &ManeuverArgs) -> ManeuverText {
    ManeuverText {
        id: a.id.clone(),
        category: a.category.clone(),
        method: a.method.clone(),
        label: a.label.clone(),
        thruster: a.thruster.clone(),
        after: a.after.clone(),
        via_altitude: a.via_altitude.clone(),
        body: a.body.clone(),
        altitude: a.altitude.clone(),
        perigee_altitude: a.perigee_altitude.clone(),
        apogee_altitude: a.apogee_altitude.clone(),
        inclination: a.inclination.clone(),
        sunlit_only: a.sunlit_only.clone(),
        mode: a.mode.clone(),
        duty: a.duty.clone(),
        steering: a.steering.clone(),
    }
}

/// `fune catalog add|set|unset|rm|rename|mv|source add|source rm`
pub fn catalog<'a>(ws: &fune_service::Workspace, c: &'a CatalogCmd) -> Built<'a> {
    Ok(match c {
        CatalogCmd::Add {
            id,
            kind,
            label,
            label_en,
            maturity,
            from_part,
            subdir,
            edit: opts,
        } => Some((
            Op::CreateCatalogItem {
                id: id.clone(),
                kind: kind.clone(),
                label: label.clone(),
                label_en: label_en.clone(),
                maturity: maturity.as_deref().map(edit::maturity).transpose()?,
                copy_from_part: from_part.clone(),
                dir: subdir.clone(),
            },
            opts,
        )),
        CatalogCmd::Set { .. } => Some(catalog_set(ws, c)?),
        CatalogCmd::Unset {
            id,
            path,
            edit: opts,
        } => Some((
            edit::unset_op(&edit::UnsetText {
                item: Some(id.clone()),
                path: path.clone(),
            })?,
            opts,
        )),
        CatalogCmd::Rm { id, edit: opts } => Some((Op::RemoveCatalogItem { id: id.clone() }, opts)),
        CatalogCmd::Rename {
            from,
            to,
            edit: opts,
        } => Some((
            Op::RenameCatalogItem {
                from: from.clone(),
                to: to.clone(),
            },
            opts,
        )),
        CatalogCmd::Mv {
            id,
            target,
            edit: opts,
        } => Some((
            Op::MoveCatalogItem {
                item: id.clone(),
                dir: target.clone(),
            },
            opts,
        )),
        CatalogCmd::Source(c) => Some(catalog_source(c)),
        CatalogCmd::Ls
        | CatalogCmd::Show { .. }
        | CatalogCmd::Fetch { .. }
        | CatalogCmd::Update { .. } => None,
    })
}

/// `fune catalog source add|rm` → `AddCatalogSource`（引数の読み方は service）・`RemoveCatalogSource`。
fn catalog_source(c: &CatalogSourceCmd) -> (Op, &crate::args::EditOpts) {
    match c {
        CatalogSourceCmd::Add {
            name,
            git,
            rev,
            edit: opts,
        } => (
            edit::catalog_source_add_op(&CatalogSourceAddText {
                name: name.clone(),
                git: git.clone(),
                rev: rev.clone(),
            }),
            opts,
        ),
        CatalogSourceCmd::Rm { name, edit: opts } => {
            (Op::RemoveCatalogSource { name: name.clone() }, opts)
        }
    }
}

/// `fune catalog set <品> <パス> <値>` → `SetCatalogValue` か `SetCatalogField`。
fn catalog_set<'a>(
    ws: &fune_service::Workspace,
    c: &'a CatalogCmd,
) -> anyhow::Result<(Op, &'a crate::args::EditOpts)> {
    let CatalogCmd::Set {
        id,
        path,
        value,
        source,
        at,
        note,
        min,
        max,
        edit: opts,
    } = c
    else {
        anyhow::bail!("not a catalog set");
    };
    let text = edit::SetText {
        item: Some(id.clone()),
        path: path.clone(),
        value: value.clone(),
        source: source.clone(),
        at: at.clone(),
        note: note.clone(),
        min: min.clone(),
        max: max.clone(),
        ops: Vec::new(),
    };
    Ok((edit::set_op(ws, &text)?, opts))
}
