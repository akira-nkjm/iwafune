//! 台帳の検査: すべての crate が docs/architecture/registry.md の「1. Crates and dependencies」の表に載っているか。
//!
//! 本文の途中に crate 名が出てくるだけでは載っているとは見なさない。表の行の最初の欄（`| `name` |`）を見る。

use crate::meta::Package;
use anyhow::{Context, Result};
use std::path::Path;

/// 台帳に載せなくてよい crate（開発用の検査）。
const EXEMPT: &[&str] = &["xtask"];

/// crate の表がある節の見出し（この見出しから次の `## ` まで）。
const CRATES_HEADING: &str = "## 1. Crates and dependencies";

/// 違反を `errors` に積む。
pub fn check(root: &Path, packages: &[Package], errors: &mut Vec<String>) -> Result<()> {
    let path = root.join("docs/architecture/registry.md");
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read {}", path.display()))?;
    let names: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
    errors.extend(violations(&names, &text));
    Ok(())
}

/// 表に載っていない crate の文。
fn violations(names: &[&str], registry: &str) -> Vec<String> {
    let listed = listed_crates(registry);
    names
        .iter()
        .filter(|n| !EXEMPT.contains(n) && !listed.iter().any(|l| l == **n))
        .map(|n| {
            format!(
                "crate `{n}` is not listed in the crate table of registry.md (\"{CRATES_HEADING}\")"
            )
        })
        .collect()
}

/// crate の表の行の最初の欄にある名前（バッククォートで囲んだもの）。
fn listed_crates(registry: &str) -> Vec<String> {
    let mut in_section = false;
    let mut out = Vec::new();
    for line in registry.lines() {
        if line.starts_with("## ") {
            in_section = line.trim_end() == CRATES_HEADING;
            continue;
        }
        if !in_section {
            continue;
        }
        let first = line
            .strip_prefix('|')
            .and_then(|rest| rest.split('|').next())
            .map(str::trim);
        if let Some(name) = first
            .and_then(|cell| cell.strip_prefix('`'))
            .and_then(|cell| cell.strip_suffix('`'))
        {
            out.push(name.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "# Registry\n\nThe crate `fune-ghost` is mentioned here.\n\n\
        ## 1. Crates and dependencies\n\n| crate | Location |\n|---|---|\n\
        | `sat-units` | crates/lib/units |\n| `fune-model` | crates/app/model |\n\n\
        ## 2. Feature registry\n\n| Feature | Home |\n| `fune-store` | somewhere |\n";

    #[test]
    fn only_first_cells_of_the_crate_table_count() {
        assert_eq!(listed_crates(DOC), ["sat-units", "fune-model"]);
    }

    #[test]
    fn a_crate_mentioned_only_in_prose_or_other_sections_is_missing() {
        let errors = violations(&["sat-units", "fune-ghost", "fune-store", "xtask"], DOC);
        assert_eq!(errors.len(), 2);
        assert!(errors[0].contains("fune-ghost"));
        assert!(errors[1].contains("fune-store"));
    }

    #[test]
    fn every_listed_crate_passes() {
        assert!(violations(&["sat-units", "fune-model"], DOC).is_empty());
    }
}
