//! `fune phase …`: 引数 → Op（`add|set|rm|rename|set-field|unset-field|copy-orbit|unset-orbit|unset-law`。打った値を読むのは service の `edit::phase_op`・
//! `edit::phase_field_op`）と、読み取り（`ls`・`show`。HTTP の `GET /phases` と同じ関数）。

use super::common::{Built, Handled};
use crate::args::{PhaseArgs, PhaseCmd};
use crate::output::{C, Out};
use fune_service::dto::{Op, PhaseText, PhaseView};
use fune_service::{Workspace, edit, query};

/// `fune phase add|set|rm|rename|set-field|unset-field` と `fune case set-phase|unset-phase`
pub fn phase(c: &PhaseCmd) -> Built<'_> {
    Ok(match c {
        PhaseCmd::Add(a) => Some((edit::phase_op(&text(a), true)?, &a.edit)),
        PhaseCmd::Set(a) => Some((edit::phase_op(&text(a), false)?, &a.edit)),
        PhaseCmd::Rm { id, edit } => Some((Op::RemovePhase { id: id.clone() }, edit)),
        PhaseCmd::Rename { id, to, edit } => Some((
            Op::RenamePhase {
                id: id.clone(),
                to: to.clone(),
            },
            edit,
        )),
        PhaseCmd::SetField {
            phase,
            path,
            value,
            edit: opts,
        } => Some((edit::phase_field_op(phase, path, Some(value))?, opts)),
        PhaseCmd::UnsetField {
            phase,
            path,
            edit: opts,
        } => Some((edit::phase_field_op(phase, path, None)?, opts)),
        PhaseCmd::CopyOrbit { phase, edit } => Some((edit::copy_orbit_op(phase), edit)),
        PhaseCmd::UnsetOrbit { phase, edit } => Some((edit::unset_orbit_op(phase), edit)),
        PhaseCmd::UnsetLaw { phase, mode, edit } => Some((edit::unset_law_op(phase, mode), edit)),
        PhaseCmd::Ls | PhaseCmd::Show { .. } => None,
    })
}

fn text(a: &PhaseArgs) -> PhaseText {
    PhaseText {
        id: a.id.clone(),
        label: a.label.clone(),
        order: a.order.clone(),
        start: a.start.clone(),
        end: a.end.clone(),
        duration: a.duration.clone(),
        modes: a.modes.clone(),
        configuration: a.configuration.clone(),
    }
}

/// `fune phase ls|show`（読み取り）。
pub fn read(ws: &Workspace, cmd: &PhaseCmd, out: &Out) -> Handled {
    match cmd {
        PhaseCmd::Ls => ls(ws, out),
        PhaseCmd::Show { id } => show(ws, id, out),
        _ => Ok(None),
    }
}

fn ls(ws: &Workspace, out: &Out) -> Handled {
    let phases = query::phases(ws)?;
    if out.json {
        out.json(&phases)?;
        return Ok(Some(0));
    }
    let mut t = out.table(&[
        "#", "ID", "Name", "Duration", "Modes", "Orbit", "Law", "Cases",
    ]);
    for p in &phases {
        t.row(vec![
            C::Num(p.position.to_string()),
            C::Strong(p.id.clone()),
            C::Text(p.label.clone()),
            C::Num(
                p.duration
                    .as_ref()
                    .map(|d| d.display.clone())
                    .unwrap_or_default(),
            ),
            C::Text(p.modes.join(" ")),
            C::Dim(if p.orbit_overridden { "override" } else { "" }.to_owned()),
            C::Dim(
                p.laws
                    .iter()
                    .map(|l| l.mode.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            C::Dim(p.cases.join(" ")),
        ]);
    }
    t.print();
    Ok(Some(0))
}

fn show(ws: &Workspace, id: &str, out: &Out) -> Handled {
    let phases = query::phases(ws)?;
    let Some(p) = phases.iter().find(|p| p.id == id) else {
        anyhow::bail!("Phase `{id}` not found");
    };
    if out.json {
        out.json(p)?;
        return Ok(Some(0));
    }
    print_phase(p, out);
    Ok(Some(0))
}

fn print_phase(p: &PhaseView, out: &Out) {
    out.heading(&format!("{} ({})", p.label, p.id));
    let line = |key: &str, value: String| {
        if !value.is_empty() {
            println!("  {:<10} {value}", out.dim(key));
        }
    };
    line("order", p.order.to_string());
    line("start", p.start.clone().unwrap_or_default());
    line("end", p.end.clone().unwrap_or_default());
    line(
        "duration",
        p.duration
            .as_ref()
            .map(|d| d.display.clone())
            .unwrap_or_default(),
    );
    line("modes", p.modes.join(", "));
    line("cases", p.cases.join(", "));
    line("file", p.file.clone());
    for f in p.orbit.iter().chain(p.laws.iter().flat_map(|l| &l.fields)) {
        println!("  {:<28} {}", f.path, f.display);
    }
}
