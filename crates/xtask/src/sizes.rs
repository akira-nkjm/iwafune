//! 大きさの警告（CONTRIBUTING.md の目安: 1 ファイル 400 行・1 関数 60 行）。エラーにはせず、`warning:` として出すだけ。
//!
//! - ファイル: 400 行を超える Rust / TS（生成物と係数表は除く）。
//! - 関数: 60 行を超える Rust の関数（テスト以外のコード。データの表と NRLMSISE の移植は [`LONG_FN_EXEMPT`] で除く）。

use crate::scan::{function_lengths, production_text};
use std::path::{Path, PathBuf};

/// ファイルの行数の目安。
const MAX_LINES: usize = 400;

/// 関数の行数の目安。
const MAX_FN_LINES: usize = 60;

/// 行数の目安から外すデータの表（ロジックを持たない係数表）。
const LONG_DATA_FILES: &[&str] = &["crates/lib/astro/src/atmosphere/nrlmsise00/coeffs.rs"];

/// 関数の長さの目安から外すパスの接頭辞と理由。
const LONG_FN_EXEMPT: &[(&str, &str)] = &[
    (
        "crates/lib/astro/src/atmosphere/nrlmsise00",
        "a port of the reference NRLMSISE-00 model; it keeps the structure of the original so that it can be compared line by line",
    ),
    (
        "crates/lib/units/src/unit/named.rs",
        "the table of named units (one entry per unit)",
    ),
    (
        "crates/app/report/src/labels.rs",
        "the table of bilingual report labels (one entry per label)",
    ),
    (
        "crates/xtask/src/single_home/rules",
        "data tables of the single-home rules (one entry per rule)",
    ),
];

/// 警告の文を出力する。`files` はワークスペースのファイル一覧。
pub fn warn(root: &Path, files: &[PathBuf]) {
    for file in files {
        let rel = file.trim_prefix(root).to_string_lossy().replace('\\', "/");
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        for message in long_file_warning(&rel, &text)
            .into_iter()
            .chain(long_function_warnings(&rel, &text))
        {
            println!("warning: {message}");
        }
    }
}

/// 行数が目安を超えるコードのファイルの警告。
fn long_file_warning(rel: &str, text: &str) -> Option<String> {
    let is_code = rel.ends_with(".rs") || rel.ends_with(".ts") || rel.ends_with(".tsx");
    let generated = rel.split('/').any(|c| c == "gen");
    let data = LONG_DATA_FILES.iter().any(|d| rel.ends_with(d));
    if !is_code || generated || data {
        return None;
    }
    let lines = text.lines().count();
    (lines > MAX_LINES).then(|| format!("{rel} has {lines} lines (guideline: {MAX_LINES})"))
}

/// 本番のコードにある、目安より長い関数の警告。
fn long_function_warnings(rel: &str, text: &str) -> Vec<String> {
    if !rel.ends_with(".rs") || LONG_FN_EXEMPT.iter().any(|(p, _)| rel.starts_with(p)) {
        return Vec::new();
    }
    let Some(prod) = production_text(rel, text) else {
        return Vec::new();
    };
    function_lengths(&prod)
        .into_iter()
        .filter(|f| f.len > MAX_FN_LINES)
        .map(|f| {
            format!(
                "{rel}:{} fn `{}` has {} lines (guideline: {MAX_FN_LINES})",
                f.line, f.name, f.len
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(lines: usize) -> String {
        format!("fn long() {{\n{}}}\n", "    work();\n".repeat(lines))
    }

    #[test]
    fn functions_over_the_guideline_are_warned_with_their_line() {
        let text = format!("fn short() {{}}\n{}", body(70));
        let warnings = long_function_warnings("crates/lib/x/src/a.rs", &text);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("a.rs:2 fn `long` has 72 lines"));
        assert!(long_function_warnings("crates/lib/x/src/a.rs", &body(58)).is_empty());
        assert!(!long_function_warnings("crates/lib/x/src/a.rs", &body(59)).is_empty());
    }

    #[test]
    fn tests_exempt_ports_and_other_languages_are_not_warned() {
        let in_test = format!("#[cfg(test)]\nmod tests {{\n{}}}\n", body(70));
        assert!(long_function_warnings("crates/lib/x/src/a.rs", &in_test).is_empty());
        assert!(long_function_warnings("crates/lib/x/tests/a.rs", &body(70)).is_empty());
        let port = "crates/lib/astro/src/atmosphere/nrlmsise00/gts7.rs";
        assert!(long_function_warnings(port, &body(70)).is_empty());
        assert!(long_function_warnings("web/src/a.ts", &body(70)).is_empty());
    }

    #[test]
    fn long_files_are_warned_except_generated_and_data() {
        let text = "x\n".repeat(401);
        assert!(long_file_warning("crates/a/src/b.rs", &text).is_some());
        assert!(long_file_warning("crates/a/src/b.rs", &"x\n".repeat(400)).is_none());
        assert!(long_file_warning("web/src/api/gen/T.ts", &text).is_none());
        assert!(long_file_warning("docs/a.md", &text).is_none());
        let coeffs = "crates/lib/astro/src/atmosphere/nrlmsise00/coeffs.rs";
        assert!(long_file_warning(coeffs, &text).is_none());
    }
}
