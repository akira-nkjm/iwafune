//! 解析ケース・出典・ハーネス・マヌーバ・取付点の引数 → Op（`fune case set` → `SetCase` など）。綴りは `fune_ops::to_cli` と対になる。

use super::common::{Built, list};
use crate::args::{
    CaseArgs, CaseCmd, CatalogCmd, HarnessArgs, HarnessCmd, ManeuverArgs, ManeuverCmd, SourceArgs,
    SourceCmd,
};
use fune_service::dto::{Case, Harness, ManeuverText, Op, Source};
use fune_service::edit;

/// serde の綴り（snake_case）から列挙を読む。
fn enum_of<T: std::str::FromStr>(text: &str, what: &str) -> anyhow::Result<T> {
    text.parse()
        .map_err(|_| anyhow::anyhow!("{what} として読めません: {text}"))
}

/// `fune case set|rm`
pub fn case(c: &CaseCmd) -> Built<'_> {
    Ok(match c {
        CaseCmd::Set(a) => Some((Op::SetCase { case: case_of(a)? }, &a.edit)),
        CaseCmd::Rm { id, edit } => Some((Op::RemoveCase { id: id.clone() }, edit)),
        CaseCmd::Ls | CaseCmd::FromEclipse { .. } => None,
    })
}

fn case_of(a: &CaseArgs) -> anyhow::Result<Case> {
    let value = serde_json::json!({
        "label": a.label,
        "description": a.description,
        "mode": a.mode,
        "side": a.side,
        "thermal_case": a.thermal_case,
        "orbits": a.orbits,
    });
    let mut case: Case = serde_json::from_value(value)
        .map_err(|e| anyhow::anyhow!("解析ケースを作れません: {e}"))?;
    case.id = a.id.as_str().into();
    case.beta = a
        .beta
        .as_deref()
        .map(edit::quantity_from_text)
        .transpose()?;
    Ok(case)
}

/// `fune source set|rm`
pub fn source(c: &SourceCmd) -> Built<'_> {
    Ok(match c {
        SourceCmd::Set(a) => Some((
            Op::SetSource {
                source: source_of(a)?,
            },
            &a.edit,
        )),
        SourceCmd::Rm { id, edit } => Some((Op::RemoveSource { id: id.clone() }, edit)),
        SourceCmd::Ls => None,
    })
}

fn source_of(a: &SourceArgs) -> anyhow::Result<Source> {
    Ok(Source {
        id: a.id.as_str().into(),
        kind: enum_of(
            &a.kind,
            "出典の種類（placeholder / judgement / analysis / datasheet / measured）",
        )?,
        label: a.label.clone(),
        reference: a.reference.clone(),
        revision: a.revision.clone(),
        retrieved: a.retrieved.clone(),
        sha256: a.sha256.clone(),
        distribution: a
            .distribution
            .as_deref()
            .map(|d| enum_of(d, "配布の区分（public / on_request / restricted）"))
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

fn maneuver_text(a: &ManeuverArgs) -> ManeuverText {
    ManeuverText {
        id: a.id.clone(),
        category: a.category.clone(),
        method: a.method.clone(),
        label: a.label.clone(),
        thruster: a.thruster.clone(),
        after: a.after.clone(),
        via_altitude: a.via_altitude.clone(),
        altitude: a.altitude.clone(),
        perigee_altitude: a.perigee_altitude.clone(),
        apogee_altitude: a.apogee_altitude.clone(),
        inclination: a.inclination.clone(),
        sunlit_only: a.sunlit_only.clone(),
        mode: a.mode.clone(),
    }
}

/// `fune catalog add|set|unset|rm|rename`
pub fn catalog<'a>(ws: &fune_service::Workspace, c: &'a CatalogCmd) -> Built<'a> {
    Ok(match c {
        CatalogCmd::Add {
            id,
            kind,
            label,
            label_en,
            maturity,
            from_part,
            edit: opts,
        } => Some((
            Op::CreateCatalogItem {
                id: id.clone(),
                kind: kind.clone(),
                label: label.clone(),
                label_en: label_en.clone(),
                maturity: maturity.as_deref().map(edit::maturity).transpose()?,
                copy_from_part: from_part.clone(),
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
        CatalogCmd::Ls | CatalogCmd::Show { .. } => None,
    })
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
        anyhow::bail!("catalog set ではありません");
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
