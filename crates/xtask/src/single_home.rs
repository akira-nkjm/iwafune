//! 「1 つの機能は 1 か所だけ」の機械的な検査（docs/architecture/registry.md §2 の一部を強制する）。
//!
//! 定義の書き出し（`fn rk4` や定数の値など）が、台帳で決めたファイル以外に現れたら違反にする。
//! 使う側（呼び出し）は検査しない。台帳に行を足したら、規則の表（[`rules`]）にも規則を足す。

use crate::scan::production_text;
use std::path::Path;

mod rules;
use rules::{PROD_RULES, RUST_RULES, SHARED, WEB_RULES};

/// 違反を `errors` に積む。`files` はワークスペースのファイル一覧。色の直書きは [`crate::web_colors`] が見る。
pub fn check(root: &Path, files: &[std::path::PathBuf], errors: &mut Vec<String>) {
    let mut sources: Vec<(String, String)> = Vec::new();
    for file in files {
        let rel = file.trim_prefix(root);
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        let is_code = rel_s.ends_with(".rs") || rel_s.ends_with(".ts") || rel_s.ends_with(".tsx");
        if !is_code || rel_s.starts_with("crates/xtask/") || rel_s.contains("/gen/") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        let web = rel_s.starts_with("web/");
        sources.push((rel_s.clone(), text.clone()));
        let shared = SHARED.iter().flat_map(|rules| rules.iter());
        let rules = shared.chain(if web { WEB_RULES } else { RUST_RULES });
        for rule in rules {
            check_rule(&rel_s, &text, rule, errors);
        }
        if let Some(prod) = production_text(&rel_s, &text) {
            for rule in PROD_RULES {
                check_rule(&rel_s, &prod, rule, errors);
            }
        }
    }
    errors.extend(dead_rules(&sources));
}

/// 定義が 1 つも無いことを前提に「この名前の関数を書かない」と決めた規則の探す文字列（死んだ規則の検査から外す）。
const BANNED_NAMES: &[&str] = &["fn enum_str"];

/// 許したパスのどこにも探す文字列が現れない規則（対象の関数や定数が消えたり名前が変わったのに残っている規則）。
/// 規則は定義の所在を決めるものなので、許したパスに 1 つも現れなければ何も守っていない。
fn dead_rules(sources: &[(String, String)]) -> Vec<String> {
    let all = SHARED
        .iter()
        .flat_map(|rules| rules.iter())
        .chain(RUST_RULES)
        .chain(WEB_RULES)
        .chain(PROD_RULES);
    let mut out = Vec::new();
    for (needle, allowed, what) in all {
        // 値の綴りの別表記や「ここには書かない」禁止の規則は、現れなくてよい。定義の書き出し（`fn `・`struct `）だけ見る。
        let definition = needle.starts_with("fn ") || needle.starts_with("struct ");
        if !definition || BANNED_NAMES.contains(needle) {
            continue;
        }
        let alive = sources
            .iter()
            .any(|(rel, text)| allowed.iter().any(|a| is_allowed(rel, a)) && text.contains(needle));
        if !alive {
            out.push(format!(
                "single-home rule for {what} (`{needle}`) matches nothing under {}: \
                 the definition was renamed or removed; fix or delete the rule in crates/xtask/src/single_home/rules",
                allowed.join(", ")
            ));
        }
    }
    out
}

fn check_rule(
    rel: &str,
    text: &str,
    (needle, allowed, what): &(&str, &[&str], &str),
    errors: &mut Vec<String>,
) {
    if text.contains(needle) && !allowed.iter().any(|a| is_allowed(rel, a)) {
        errors.push(format!(
            "{rel}: found `{needle}`; {what} is defined only in {} (registry.md)",
            allowed.join(", ")
        ));
    }
}

/// `allowed` がファイル（`foo.rs`）なら、そのサブモジュールのディレクトリ（`foo/`）も許す。
fn is_allowed(path: &str, allowed: &str) -> bool {
    if path == allowed || path.starts_with(allowed) {
        return true;
    }
    allowed
        .strip_suffix(".rs")
        .is_some_and(|stem| path.starts_with(&format!("{stem}/")))
}

#[cfg(test)]
mod tests;
