//! `fune self-update [--check] [--version <版>]`

use crate::output::Out;
use fune_service::self_update::{self, UpdateCheck};

/// 実行中の `fune` を置き換える（プロジェクトが無くても使える）。
pub fn run(check_only: bool, version: Option<&str>, out: &Out) -> anyhow::Result<u8> {
    if check_only {
        let c = self_update::check(version)?;
        if out.json {
            out.json(&c)?;
        } else {
            show_check(&c, out);
        }
        return Ok(0);
    }
    let outcome = self_update::update(version)?;
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
