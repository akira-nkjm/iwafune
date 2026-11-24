//! `fune configuration …`: 引数 → Op（`add|set|rm|rename|unset-pose`。打った値を読むのは service の `edit::configuration_op`）と、
//! 読み取り（`ls`・`show`。HTTP の `GET /configurations` と同じ関数）。形態の中の姿勢は `fune part ls|show --configuration`。

use super::common::{Built, Handled};
use crate::args::{ConfigurationArgs, ConfigurationCmd};
use crate::output::{C, Out};
use fune_service::dto::{ConfigurationText, ConfigurationView, Op};
use fune_service::{Workspace, edit, query};

/// `fune configuration add|set|rm|rename|unset-pose`
pub fn configuration(c: &ConfigurationCmd) -> Built<'_> {
    Ok(match c {
        ConfigurationCmd::Add(a) => Some((edit::configuration_op(&text(a), true), &a.edit)),
        ConfigurationCmd::Set(a) => Some((edit::configuration_op(&text(a), false), &a.edit)),
        ConfigurationCmd::Rm { id, edit } => {
            Some((Op::RemoveConfiguration { id: id.clone() }, edit))
        }
        ConfigurationCmd::Rename { id, to, edit } => Some((
            Op::RenameConfiguration {
                id: id.clone(),
                to: to.clone(),
            },
            edit,
        )),
        ConfigurationCmd::UnsetPose {
            configuration,
            part,
            edit: opts,
        } => Some((edit::unset_pose_op(configuration, part), opts)),
        ConfigurationCmd::Ls | ConfigurationCmd::Show { .. } => None,
    })
}

fn text(a: &ConfigurationArgs) -> ConfigurationText {
    ConfigurationText {
        id: a.id.clone(),
        label: a.label.clone(),
        label_en: a.label_en.clone(),
        description: a.description.clone(),
    }
}

/// `fune configuration ls|show`（読み取り）。
pub fn read(ws: &Workspace, cmd: &ConfigurationCmd, out: &Out) -> Handled {
    match cmd {
        ConfigurationCmd::Ls => ls(ws, out),
        ConfigurationCmd::Show { id } => show(ws, id, out),
        _ => Ok(None),
    }
}

fn ls(ws: &Workspace, out: &Out) -> Handled {
    let all = query::configurations(ws)?;
    if out.json {
        out.json(&all)?;
        return Ok(Some(0));
    }
    // 英語名は、どれかの形態が書いているときだけ列を足す（無ければ表の幅を変えない）
    let with_en = all.iter().any(|c| c.label_en.is_some());
    let mut head = vec!["ID", "Name"];
    if with_en {
        head.push("English name");
    }
    head.push("Overridden parts");
    let mut t = out.table(&head);
    for c in &all {
        let mut row = vec![C::Strong(c.id.clone()), C::Text(c.label.clone())];
        if with_en {
            row.push(C::Text(c.label_en.clone().unwrap_or_default()));
        }
        row.push(C::Dim(c.overridden.join(" ")));
        t.row(row);
    }
    t.print();
    Ok(Some(0))
}

fn show(ws: &Workspace, id: &str, out: &Out) -> Handled {
    let all = query::configurations(ws)?;
    let Some(c) = all.iter().find(|c| c.id == id) else {
        anyhow::bail!("Configuration `{id}` not found");
    };
    if out.json {
        out.json(c)?;
        return Ok(Some(0));
    }
    print_configuration(c, out);
    Ok(Some(0))
}

fn print_configuration(c: &ConfigurationView, out: &Out) {
    out.heading(&format!("{} ({})", c.label, c.id));
    let line = |key: &str, value: String| {
        if !value.is_empty() {
            println!("  {:<12} {value}", out.dim(key));
        }
    };
    line("label_en", c.label_en.clone().unwrap_or_default());
    line("description", c.description.clone().unwrap_or_default());
    line("overridden", c.overridden.join(", "));
    line("file", c.file.clone());
}
