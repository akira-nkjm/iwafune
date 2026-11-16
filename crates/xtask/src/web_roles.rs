//! Web が部品の役割（role）の名前を自分で見て決めていないかの検査。
//!
//! CONTRIBUTING.md の層の責務: `web/` は役割や属性の意味を知らない（API が構造で返す）。役割名の文字列と比べる書き方
//! （`p.role === "battery"`・`role !== "obc"`・`const X_ROLE: Role = "pcdu"` の定数）を違反にする。
//! 役割で何かを決めたいときは、その判定を `fune-service` が `PartRow` などに載せる（例: `PartRow.maneuver_thruster`）。
//!
//! テスト以外の `web/src/` のコード（生成物 `api/gen/` を除く）だけを見る。まだ API に寄せていないところは
//! [`ALLOWED`] に理由付きで置く（Issue の完了で空にする）。

use std::path::Path;

/// 役割名の比較を許すファイルと理由。
const ALLOWED: &[(&str, &str)] = &[
    ("web/src/features/diagram/power.ts", "tracked in #172"),
    ("web/src/features/diagram/model.ts", "tracked in #172"),
];

/// 識別子に使う文字。
fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '$')
}

/// 式の終わり（左側）の識別子。`part.role` なら `role`、`x?.Role` なら `Role`。
fn trailing_ident(s: &str) -> &str {
    let s = s.trim_end();
    let start = s.rfind(|c: char| !is_ident(c)).map_or(0, |i| i + 1);
    &s[start..]
}

/// 式の始まり（右側）の `a.b?.c` の最後の識別子。
fn last_segment(s: &str) -> &str {
    let s = s.trim_start();
    let end = s
        .find(|c: char| !(is_ident(c) || matches!(c, '.' | '?')))
        .unwrap_or(s.len());
    s[..end].rsplit(['.', '?']).next().unwrap_or("")
}

fn is_role_name(ident: &str) -> bool {
    ident.to_ascii_lowercase().ends_with("role")
}

fn starts_with_quote(s: &str) -> bool {
    s.trim_start().starts_with(['"', '\'', '`'])
}

fn ends_with_quote(s: &str) -> bool {
    s.trim_end().ends_with(['"', '\'', '`'])
}

/// 比較演算子（`===`・`!==`・`==`・`!=`）の位置と長さ。
fn comparisons(line: &str) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (i, _) in line.char_indices() {
        let rest = &line[i..];
        let len = if rest.starts_with("===") || rest.starts_with("!==") {
            3
        } else if rest.starts_with("==") || rest.starts_with("!=") {
            2
        } else {
            continue;
        };
        // `===` の 2 文字目から始まる `==` は数えない
        if out.last().is_none_or(|&(j, l)| i >= j + l) {
            out.push((i, len));
        }
    }
    out
}

/// 役割名の文字列との比較（`x.role === "…"`・`"…" === x.role`）。
fn role_comparison(line: &str) -> bool {
    comparisons(line).into_iter().any(|(i, len)| {
        let (left, right) = (&line[..i], &line[i + len..]);
        (is_role_name(trailing_ident(left)) && starts_with_quote(right))
            || (ends_with_quote(left) && is_role_name(last_segment(right)))
    })
}

/// 役割名の文字列の定数（`const X_ROLE: Role = "…"`・`const POWER_ROLE = "…"`）。JSX の `role="button"` は見ない。
fn role_constant(line: &str) -> bool {
    line.match_indices('=').any(|(i, _)| {
        let (left, right) = (&line[..i], &line[i + 1..]);
        let is_arrow_or_cmp = right.starts_with(['=', '>']) || left.ends_with(['=', '!', '<', '>']);
        if is_arrow_or_cmp || !starts_with_quote(right) {
            return false;
        }
        let left = left.trim_end();
        left.ends_with(": Role") || trailing_ident(left).ends_with("_ROLE")
    })
}

/// 行が役割名の文字列に頼っていれば、その種類。コメントの行は見ない。
fn role_string(line: &str) -> Option<&'static str> {
    let t = line.trim_start();
    if t.starts_with("//") || t.starts_with('*') || t.starts_with("/*") {
        return None;
    }
    if role_comparison(line) {
        Some("a comparison with a role name string")
    } else if role_constant(line) {
        Some("a constant of a role name string")
    } else {
        None
    }
}

/// 違反を `errors` に積む。`files` はワークスペースのファイル一覧。
pub fn check(root: &Path, files: &[std::path::PathBuf], errors: &mut Vec<String>) {
    for file in files {
        let rel = file.strip_prefix(root).unwrap_or(file);
        let rel = rel.to_string_lossy().replace('\\', "/");
        if !crate::web_strings::is_web_production(&rel) || ALLOWED.iter().any(|(f, _)| *f == rel) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            if let Some(what) = role_string(line) {
                errors.push(format!(
                    "{rel}:{}: found {what}; judgements decided by the meaning of a role are returned by the API (service puts them in, like `PartRow.maneuver_thruster`; layer responsibilities in CONTRIBUTING.md)",
                    n + 1
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparisons_with_a_role_literal_are_found() {
        for line in [
            "  .filter((p) => p.role === \"propulsion_thruster\" || p.role === \"thruster\")",
            "if (part?.role == 'battery') {",
            "return role !== \"obc\";",
            "if (\"pcdu\" === p.role) return;",
            "x.kind !== null && device.partRole != `rcs`",
        ] {
            assert!(role_string(line).is_some(), "{line}");
        }
    }

    #[test]
    fn role_constants_are_found() {
        assert_eq!(
            role_string("const BATTERY_ROLE: Role = \"battery\";"),
            Some("a constant of a role name string")
        );
        assert!(role_string("const POWER_HUB_ROLE = \"pcdu\";").is_some());
    }

    #[test]
    fn other_code_is_not_flagged() {
        for line in [
            "if (part.role !== part.kind) {",
            "const hub = ends.find((e) => parts.get(e.part)?.role === role);",
            "return p.role === null;",
            "<div role=\"button\" tabIndex={0}>",
            "export type PortRole = \"power_in\" | \"power_out\";",
            "const onClick = () => setRole(\"x\");",
            "  // p.role === \"battery\" は API が返す",
            "   * `role === \"x\"` の説明",
            "p.kind === \"battery\"",
            "const f = (role: Role) => role;",
            "const x = a >= \"b\";",
        ] {
            assert_eq!(role_string(line), None, "{line}");
        }
    }

    #[test]
    fn allow_list_entries_carry_a_reason() {
        assert!(
            ALLOWED
                .iter()
                .all(|(f, why)| f.starts_with("web/src/") && !why.is_empty())
        );
    }
}
