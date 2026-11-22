//! `fune skill show` / `fune skill install [--skills-dir <PATH>] [--force]`

use crate::args::SkillCmd;
use crate::output::Out;
use fune_service::skill::{self, InstallStatus};
use std::path::Path;

/// skill の本文の表示と、置き場への書き込み（プロジェクトが無くても使える）。
pub fn run(cmd: &SkillCmd, project_dir: &Path, out: &Out) -> anyhow::Result<u8> {
    match cmd {
        SkillCmd::Show => {
            if out.json {
                out.json(
                    &serde_json::json!({ "version": skill::VERSION, "text": skill::render() }),
                )?;
            } else {
                print!("{}", skill::render());
            }
        }
        SkillCmd::Install { skills_dir, force } => {
            let dir = skills_dir
                .clone()
                .unwrap_or_else(|| skill::default_skills_dir(project_dir));
            let o = skill::install(&dir, *force)?;
            if out.json {
                out.json(&o)?;
            } else {
                show_install(&o, out);
            }
        }
    }
    Ok(0)
}

/// 入れた結果の 1 行。
fn show_install(o: &skill::InstallOutcome, out: &Out) {
    let v = &o.version;
    let path = &o.path;
    match (o.status, &o.previous_version) {
        (InstallStatus::Created, _) => out.ok(format!("Installed the fune skill {v}: {path}")),
        (InstallStatus::UpToDate, _) => {
            out.ok(format!("The fune skill is up to date ({v}): {path}"))
        }
        (InstallStatus::Updated, Some(old)) => {
            out.ok(format!("Updated the fune skill {old} -> {v}: {path}"));
        }
        (InstallStatus::Updated, None) => out.ok(format!("Updated the fune skill to {v}: {path}")),
        (InstallStatus::Overwritten, _) => {
            out.ok(format!("Overwrote the edited fune skill with {v}: {path}"));
        }
    }
}

#[cfg(test)]
mod drift;
