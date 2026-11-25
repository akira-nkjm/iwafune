//! 設計書の書き出し（`fune-service` の `report`）が、解析の出力を JSON の欄の名前で読んでいないかの検査。
//!
//! 書き出しは解析の型付きの出力（`Session` の `typed_run`）から表と図を組み立てる。欄の名前（`"modes"`・`"temperature"` など）で
//! `serde_json::Value` を引くと、解析の欄の名前が変わったときに表が黙って空になる。型で読めば、欄が無くなった時点でコンパイルが通らない。
//! そのため `report.rs` と `report/` の本番のコードでは、`serde_json` の使用・`.get("…")`・`.pointer("…")`・`値["…"]` を違反にする。
//! どうしても JSON のまま読む表（解析自身の表の記述で組み立てる汎用の表）は [`ALLOWED`] に理由付きで置く
//! （そのファイルは、読めない欄を空として流さず、欠けた欄をエラーにすること）。許可したファイルに違反の書き方が無いときと、
//! 見るファイルが 1 つも無いときも違反にする（規則が何も守っていない）。

use crate::scan::production_text;
use std::path::{Path, PathBuf};

/// JSON の欄の名前での読み取りを許すファイルと理由（いまは無い）。
const ALLOWED: &[(&str, &str)] = &[];

/// 書き出しのモジュール（`report.rs` とそのサブモジュール）か。
fn is_report_module(rel: &str) -> bool {
    rel == "crates/app/service/src/report.rs" || rel.starts_with("crates/app/service/src/report/")
}

/// 違反を `errors` に積む。`files` はワークスペースのファイル一覧。
pub fn check(root: &Path, files: &[PathBuf], errors: &mut Vec<String>) {
    let mut seen = 0;
    let mut allowed_hits: Vec<&str> = Vec::new();
    for file in files {
        let rel = file.trim_prefix(root).to_string_lossy().replace('\\', "/");
        if !rel.ends_with(".rs") || !is_report_module(&rel) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        let Some(prod) = production_text(&rel, &text) else {
            continue;
        };
        seen += 1;
        let found = violations(&rel, &prod);
        if let Some((path, _)) = ALLOWED.iter().find(|(p, _)| *p == rel) {
            if found.is_empty() {
                errors.push(format!(
                    "{rel}: is allowed to read JSON by key in crates/xtask/src/report_json.rs but does not; remove it from ALLOWED"
                ));
            }
            allowed_hits.push(path);
        } else {
            errors.extend(found);
        }
    }
    if seen == 0 {
        errors.push(
            "report-json rule matches no file under crates/app/service/src/report*: the module was renamed or moved; fix the rule in crates/xtask/src/report_json.rs"
                .to_owned(),
        );
    }
    for (path, _) in ALLOWED {
        if !allowed_hits.contains(path) {
            errors.push(format!(
                "{path}: listed in ALLOWED of crates/xtask/src/report_json.rs but not found"
            ));
        }
    }
}

/// 1 つのファイル（本番のコード）の違反の文。コメントの行と行末のコメントは見ない。
fn violations(rel: &str, prod: &str) -> Vec<String> {
    prod.lines()
        .enumerate()
        .filter_map(|(n, line)| {
            let code = line.split("//").next().unwrap_or(line);
            reads_json_by_key(code).then(|| {
                format!(
                    "{rel}:{}: reads JSON by key (`serde_json`, `.get(\"…\")`, `.pointer(\"…\")` or `x[\"…\"]`); \
                     the report export reads the typed analysis outputs through `Run` (report/common.rs), so a renamed field fails to compile instead of leaving a table empty",
                    n + 1
                )
            })
        })
        .collect()
}

/// JSON を欄の名前で読む書き方か。
fn reads_json_by_key(code: &str) -> bool {
    if code.contains("serde_json") {
        return true;
    }
    if [".get(\"", ".get_mut(\"", ".pointer(\"", ".pointer_mut(\""]
        .iter()
        .any(|p| code.contains(p))
    {
        return true;
    }
    // `x["key"]`（直前が識別子・`)`・`]` の添字。配列の書き出し `= ["a", "b"]` は添字ではない）
    code.match_indices("[\"").any(|(i, _)| {
        code[..i]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | ')' | ']'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REL: &str = "crates/app/service/src/report/power.rs";

    #[test]
    fn json_reads_by_key_are_found() {
        for line in [
            "use serde_json::Value;",
            "let v = output.get(\"modes\");",
            "let v = out.pointer(\"/modes/0\");",
            "let v = output[\"modes\"];",
            "let v = f(x)[\"modes\"];",
            "let n = serde_json::to_value(k)?;",
        ] {
            assert_eq!(violations(REL, line).len(), 1, "{line}");
        }
    }

    #[test]
    fn typed_reads_arrays_and_comments_are_not_found() {
        for line in [
            "let v = output.modes.iter().map(|m| m.total.value());",
            "let both = [\"generation\", \"load\"];",
            "foo([\"a\"]);",
            "// `.get(\"modes\")` と書かない",
            "let x = 1; // serde_json は使わない",
            "let name = map.get(&key);",
        ] {
            assert!(violations(REL, line).is_empty(), "{line}");
        }
    }

    #[test]
    fn the_report_module_paths_are_recognised() {
        assert!(is_report_module("crates/app/service/src/report.rs"));
        assert!(is_report_module("crates/app/service/src/report/history.rs"));
        assert!(!is_report_module("crates/app/service/src/reports.rs"));
        assert!(!is_report_module("crates/app/service/src/budget.rs"));
    }

    #[test]
    fn a_missing_module_and_dead_allow_entries_are_errors() {
        let mut errors = Vec::new();
        check(Path::new("/nonexistent-root"), &[], &mut errors);
        assert_eq!(errors.len(), 1 + ALLOWED.len());
        assert!(errors[0].contains("matches no file"));
    }

    #[test]
    fn the_real_report_module_is_clean() {
        // 実際のソース（このリポジトリの report）に違反が無いこと（規則が現在の木で通る）
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dir = root.join("crates/app/service/src");
        let mut files = vec![dir.join("report.rs")];
        if let Ok(entries) = std::fs::read_dir(dir.join("report")) {
            files.extend(entries.flatten().map(|e| e.path()));
        }
        let mut errors = Vec::new();
        check(&root, &files, &mut errors);
        assert!(errors.is_empty(), "{errors:?}");
    }
}
