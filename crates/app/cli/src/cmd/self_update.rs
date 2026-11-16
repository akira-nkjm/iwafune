//! `fune self-update [--check] [--version <版>]`

use crate::output::Out;
use fune_service::self_update::{self, Step, UpdateCheck};

/// 実行中の `fune` を置き換える（プロジェクトが無くても使える）。
///
/// 進行は標準エラーに 1 行ずつ出す（`--json` のときは出さない）。進捗バーは標準エラーが端末のときだけ。
pub fn run(check_only: bool, version: Option<&str>, out: &Out) -> anyhow::Result<u8> {
    let mut report = |step: Step| {
        if !out.json {
            out.status(step_text(&step));
        }
    };
    if check_only {
        let c = self_update::check(version, &mut report)?;
        if out.json {
            out.json(&c)?;
        } else {
            show_check(&c, out);
        }
        return Ok(0);
    }
    let bar = !out.json && out.stderr_is_terminal();
    let outcome = self_update::update(version, bar, &mut report)?;
    if out.json {
        out.json(&outcome)?;
        return Ok(0);
    }
    match &outcome.replaced {
        Some(path) => {
            out.ok(format!(
                "Updated fune {} -> {}",
                outcome.check.current, outcome.check.latest
            ));
            println!("  {}", out.dim(format!("Replaced {path}")));
        }
        None => show_current(&outcome.check, out),
    }
    Ok(0)
}

/// 進行の行の文面。
fn step_text(step: &Step) -> String {
    match step {
        Step::CheckingLatest => "Checking the latest release...".to_owned(),
        Step::Downloading { asset, version } => format!("Downloading fune {version} ({asset})..."),
        Step::Verifying => "Verifying the checksum...".to_owned(),
        Step::Unpacking => "Unpacking and checking the binary...".to_owned(),
        Step::Replacing { path } => format!("Replacing {path}..."),
    }
}

/// `--check` の表示。
fn show_check(c: &UpdateCheck, out: &Out) {
    if !c.available {
        return show_current(c, out);
    }
    if c.explicit {
        println!(
            "fune {} is installed; fune {} would be installed",
            c.current, c.latest
        );
    } else {
        println!("Update available: fune {} -> {}", c.current, c.latest);
    }
    println!("  {}", out.dim(format!("Asset: {}", c.url)));
    println!("  {}", out.dim("Run: fune self-update"));
}

/// 置き換える対象が無いときの表示。
fn show_current(c: &UpdateCheck, out: &Out) {
    if c.ahead {
        out.ok(format!(
            "fune {} is newer than the latest release ({}); nothing to do",
            c.current, c.latest
        ));
        println!(
            "  {}",
            out.dim("Install a specific version with: fune self-update --version <VERSION>")
        );
    } else {
        out.ok(format!("fune {} is already the latest version", c.current));
    }
}
