//! 「`fune-projection` にモデル化の既定値を書かない」の検査（CONTRIBUTING.md の層の責務）。
//!
//! 設計データから解析の入力への写し替えは、書かれた値をそのまま（書かれていなければ `None` のまま）渡し、
//! 既定値は解析（sat-analysis）が持つ。テスト以外の `crates/app/projection/src/` で次を違反にする。
//! - `const DEFAULT_…` の定数（[`ALLOWED`] に理由付きで置いたものを除く）
//! - 値の直書きの代わり（`unwrap_or(` / `map_or(` の最初の引数が数・`true`・`false`・`Vec3::`・`Quat::`）
//!
//!   慣例の既定値を sat-astro / sat-analysis から呼ぶもの（`unwrap_or(DEFAULT_ECCENTRICITY)`・`unwrap_or_default()`）は見ない。

use crate::single_home::production_text;
use std::path::Path;

/// 検査する範囲。
const SCOPE: &str = "crates/app/projection/src/";

/// 許す `const DEFAULT_…`（(ファイルの接尾辞, 定数名, 理由)）。モデルの値でないものだけをここに置く。
const ALLOWED: &[(&str, &str, &str)] = &[(
    "case_policy.rs",
    "DEFAULT_CASE_ANALYSES",
    "IDs of analyses that need a default case (a declaration of how cases are chosen, not an analysis value)",
)];

/// 値の直書きとみなす `unwrap_or(` / `map_or(` の最初の引数の書き出し。
const LITERAL_STARTS: &[&str] = &["true", "false", "Vec3::", "Quat::", "-"];

/// 違反を `errors` に積む。`files` はワークスペースのファイル一覧。
pub fn check(root: &Path, files: &[std::path::PathBuf], errors: &mut Vec<String>) {
    for file in files {
        let rel = file.trim_prefix(root);
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        if !rel_s.starts_with(SCOPE) || !rel_s.ends_with(".rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        if let Some(prod) = production_text(&rel_s, &text) {
            errors.extend(violations(&rel_s, prod));
        }
    }
}

/// テスト以外のコード `code` の違反の文（1 件 1 行）。
fn violations(rel: &str, code: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (n, line) in code.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        if let Some(rest) = trimmed
            .split("const DEFAULT_")
            .nth(1)
            .filter(|_| trimmed.contains("const DEFAULT_"))
        {
            let name = format!(
                "DEFAULT_{}",
                rest.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .next()
                    .unwrap_or("")
            );
            if !ALLOWED
                .iter()
                .any(|(path, allowed, _)| rel.ends_with(path) && *allowed == name)
            {
                out.push(format!(
                    "{rel}:{}: found `const {name}`; modelling defaults belong to the analysis (sat-analysis), and projection passes on the written value (None if absent)",
                    n + 1
                ));
            }
        }
        for call in ["unwrap_or(", "map_or("] {
            for rest in trimmed.split(call).skip(1) {
                if starts_literal(rest) {
                    out.push(format!(
                        "{rel}:{}: found a hard-coded default in `{call}`; modelling defaults belong to the analysis (sat-analysis), and projection passes None on",
                        n + 1
                    ));
                }
            }
        }
    }
    out
}

/// 値の直書きの書き出しか（数・`true`・`false`・`Vec3::`・`Quat::`・負の数）。
fn starts_literal(rest: &str) -> bool {
    rest.chars().next().is_some_and(|c| c.is_ascii_digit())
        || LITERAL_STARTS.iter().any(|s| rest.starts_with(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "crates/app/projection/src/power.rs";

    #[test]
    fn default_constants_and_literal_fallbacks_are_rejected() {
        assert_eq!(
            violations(FILE, "pub const DEFAULT_ORBITS: u32 = 3;").len(),
            1
        );
        assert_eq!(violations(FILE, "x.unwrap_or(0.0)").len(), 1);
        assert_eq!(violations(FILE, "x.unwrap_or(false)").len(), 1);
        assert_eq!(violations(FILE, "x.map_or(1.0, f)").len(), 1);
        assert_eq!(violations(FILE, "x.unwrap_or(Vec3::ZERO)").len(), 1);
    }

    /// 慣例の既定値を呼ぶもの・構造の値・コメントは見ない。
    #[test]
    fn conventions_and_structure_are_accepted() {
        for ok in [
            "o.eccentricity.unwrap_or(DEFAULT_ECCENTRICITY)",
            "x.unwrap_or_default()",
            "x.map_or_else(|| a, f)",
            "x.unwrap_or(\"\")",
            "x.map_or(v, |q| q.rotate(v))",
            "// const DEFAULT_ORBITS: u32 = 3;",
            "const SENSOR_FOV_ORBITS: u32 = 1;",
        ] {
            assert!(violations(FILE, ok).is_empty(), "{ok}");
        }
    }

    /// 許可リストの定数だけは通る（別のファイルの同名は通らない）。
    #[test]
    fn allow_list_is_per_file_and_name() {
        let code = "const DEFAULT_CASE_ANALYSES: &[&str] = &[];";
        assert!(violations("crates/app/projection/src/case_policy.rs", code).is_empty());
        assert_eq!(violations(FILE, code).len(), 1);
    }
}
