//! Web が文字列を分解して ID やパスを読んでいないかの検査（CONTRIBUTING.md の層の責務: `web/` は
//! 「文字列の分解による ID やパスの解釈」をしない。API が構造で返す）。
//!
//! テスト以外の `web/src/` のコード（生成物 `api/gen/` を除く）で、パスや ID の区切り（`/`・`.`）で
//! 分ける書き方と、`.` か `/` で終わる接頭辞で比べる書き方（`startsWith("mission.")`）を違反にする。
//! カンマ区切りの一覧（`split(",")`）や改行（`split("\n")`）は区切りが違うので見ない。

use std::path::Path;

/// パスや ID の区切りで分ける書き方。
const SPLITTERS: &[&str] = &[
    ".split(\"/\")",
    ".split('/')",
    ".split(\".\")",
    ".split('.')",
    ".split(/[\\\\/]/)",
    ".indexOf(\".\")",
    ".indexOf(\"/\")",
    ".lastIndexOf(\".\")",
    ".lastIndexOf(\"/\")",
];

/// `.` か `/` で終わる文字列の接頭辞で比べるところ（`startsWith("mission.")`）を探す。
fn prefix_match(line: &str) -> Option<&str> {
    for (i, _) in line.match_indices(".startsWith(") {
        let rest = &line[i + ".startsWith(".len()..];
        // 文字列の直書きでなければ（`startsWith(prefix)`）次の出現へ
        let Some(quote) = rest
            .chars()
            .next()
            .filter(|c| matches!(c, '"' | '\'' | '`'))
        else {
            continue;
        };
        let Some(end) = rest[1..].find(quote) else {
            continue;
        };
        let literal = &rest[1..1 + end];
        if literal.ends_with('.') || literal.ends_with('/') {
            return Some(literal);
        }
    }
    None
}

/// 違反を `errors` に積む。`files` はワークスペースのファイル一覧。
pub fn check(root: &Path, files: &[std::path::PathBuf], errors: &mut Vec<String>) {
    for file in files {
        let rel = file.strip_prefix(root).unwrap_or(file);
        let rel = rel.to_string_lossy().replace('\\', "/");
        if !is_web_production(&rel) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            let found = SPLITTERS
                .iter()
                .find(|s| line.contains(*s))
                .map(|s| (*s).to_owned())
                .or_else(|| prefix_match(line).map(|p| format!(".startsWith(\"{p}\")")));
            if let Some(what) = found {
                errors.push(format!(
                    "{rel}:{}: `{what}` で文字列を分解しています。ID やパスは API の構造（PortRef・PartRow.dir・SettingView.scope など）を使います（CONTRIBUTING.md）",
                    n + 1
                ));
            }
        }
    }
}

/// `web/src/` のテスト以外の TS（生成物とテストの補助を除く）。
fn is_web_production(rel: &str) -> bool {
    let code = rel.ends_with(".ts") || rel.ends_with(".tsx");
    code && rel.starts_with("web/src/")
        && !rel.contains("/gen/")
        && !rel.starts_with("web/src/test/")
        && !rel.contains(".test.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_splitting_and_dotted_prefixes_are_found() {
        assert!(SPLITTERS.iter().any(|s| "a.split(\"/\")".contains(s)));
        assert!(SPLITTERS.iter().any(|s| "f.split(/[\\\\/]/)".contains(s)));
        assert_eq!(
            prefix_match("s.key.startsWith(\"mission.\")"),
            Some("mission.")
        );
        assert_eq!(prefix_match("p.startsWith('parts/')"), Some("parts/"));
        assert_eq!(prefix_match("node.startsWith(\"junction:\")"), None);
        assert_eq!(prefix_match("lang.startsWith(\"en\")"), None);
        assert_eq!(
            prefix_match("a.startsWith(p) || b.startsWith(\"budget.\")"),
            Some("budget.")
        );
        assert!(!SPLITTERS.iter().any(|s| "s.split(\",\")".contains(s)));
        assert!(is_web_production("web/src/kit/tree.ts"));
        assert!(!is_web_production("web/src/kit/tree.test.ts"));
        assert!(!is_web_production("web/src/api/gen/PartRow.ts"));
    }
}
