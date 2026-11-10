//! 「1 つの機能は 1 か所だけ」の機械的な検査（docs/architecture/registry.md §2 の一部を強制する）。
//!
//! 定義の書き出し（`fn rk4` や定数の値など）が、台帳で決めたファイル以外に現れたら違反にする。
//! 使う側（呼び出し）は検査しない。台帳に行を足したら、規則の表（[`rules`]）にも規則を足す。

use std::path::Path;

mod rules;
use rules::{PROD_RULES, RULES, WEB_RULES};

/// 違反を `errors` に積む。`files` はワークスペースのファイル一覧。色の直書きは [`crate::web_colors`] が見る。
pub fn check(root: &Path, files: &[std::path::PathBuf], errors: &mut Vec<String>) {
    for file in files {
        let rel = file.strip_prefix(root).unwrap_or(file);
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        let is_code = rel_s.ends_with(".rs") || rel_s.ends_with(".ts") || rel_s.ends_with(".tsx");
        if !is_code || rel_s.starts_with("crates/xtask/") || rel_s.contains("/gen/") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        let web = rel_s.starts_with("web/");
        let rules = if web {
            RULES.iter().chain(WEB_RULES)
        } else {
            RULES.iter().chain(&[][..])
        };
        for rule in rules {
            check_rule(&rel_s, &text, rule, errors);
        }
        if let Some(prod) = production_text(&rel_s, &text) {
            for rule in PROD_RULES {
                check_rule(&rel_s, prod, rule, errors);
            }
        }
    }
}

fn check_rule(
    rel: &str,
    text: &str,
    (needle, allowed, what): &(&str, &[&str], &str),
    errors: &mut Vec<String>,
) {
    if text.contains(needle) && !allowed.iter().any(|a| is_allowed(rel, a)) {
        errors.push(format!(
            "{rel}: `{needle}` が見つかりました。{what}は {} だけに置きます（registry.md）",
            allowed.join(", ")
        ));
    }
}

/// テスト以外の部分（テストのファイルは None、それ以外は最初の `#[cfg(test)]` より前）。
fn production_text<'a>(rel: &str, text: &'a str) -> Option<&'a str> {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    let test_file = rel.contains("/tests/")
        || name == "tests.rs"
        || name.ends_with("_tests.rs")
        || name == "testkit.rs"
        || name.contains(".test.");
    if test_file {
        return None;
    }
    Some(text.split("#[cfg(test)]").next().unwrap_or(text))
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
mod tests {
    use super::*;

    #[test]
    fn analysis_display_conversions_are_rejected_only_in_production() {
        for needle in ["/ 3600.0", "* 100.0", ".log10()"] {
            let rule = PROD_RULES.iter().find(|r| r.0 == needle).unwrap();
            let mut errors = Vec::new();
            check_rule(
                "crates/lib/analysis/src/check.rs",
                needle,
                rule,
                &mut errors,
            );
            assert_eq!(errors.len(), 1);
            errors.clear();
            check_rule("crates/lib/units/src/format.rs", needle, rule, &mut errors);
            assert!(errors.is_empty());
            let code = format!("fn run() {{}}\n#[cfg(test)]\nmod tests {{ {needle} }}");
            check_rule(
                "crates/lib/analysis/src/check.rs",
                production_text("check.rs", &code).unwrap(),
                rule,
                &mut errors,
            );
            assert!(errors.is_empty());
        }
    }
}
