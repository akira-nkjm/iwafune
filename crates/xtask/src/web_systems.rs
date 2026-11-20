//! Web が系（サブシステム）の名前を自分で見て決めていないかの検査。
//!
//! CONTRIBUTING.md の層の責務: `web/` は系の名前の意味を知らない。`system === "power"`・`p.system !== "structure"` のように、
//! 系を表す識別子（`system`・`p.system`・`activeSystem` など、名前が `system` で終わるもの）を文字列と比べる書き方を違反にする。
//! 系で何かを決めたいときは、判定を `fune-service` が返す（例: `PartRow.structural`・`DefsView.mode_levels`）か、
//! 専用の表示は登録表（`features/systems/dedicated.tsx` の `DEDICATED_VIEWS`）に系をキーとして足す（キーは比較ではない）。
//!
//! テスト以外の `web/src/` のコード（生成物 `api/gen/` を除く）だけを見る。比較を許すファイルは [`ALLOWED`] に理由付きで置く。
//! 役割名の比較は [`crate::web_roles`]。

use crate::web_roles::{
    comparisons, ends_with_quote, last_segment, starts_with_quote, trailing_ident,
};
use std::path::Path;

/// 系の名前の比較を許すファイルと理由。登録表は系をキーに持つだけで比較は書かないので、いまは空。
const ALLOWED: &[(&str, &str)] = &[];

fn is_system_name(ident: &str) -> bool {
    ident.to_ascii_lowercase().ends_with("system")
}

/// 系を表す識別子と文字列の比較（`system === "…"`・`"…" !== p.system`）。
fn system_comparison(line: &str) -> bool {
    comparisons(line).into_iter().any(|(i, len)| {
        let (left, right) = (&line[..i], &line[i + len..]);
        (is_system_name(trailing_ident(left)) && starts_with_quote(right))
            || (ends_with_quote(left) && is_system_name(last_segment(right)))
    })
}

/// コメントの行は見ない。
fn flagged(line: &str) -> bool {
    let t = line.trim_start();
    !(t.starts_with("//") || t.starts_with('*') || t.starts_with("/*")) && system_comparison(line)
}

/// 違反を `errors` に積む。`files` はワークスペースのファイル一覧。
pub fn check(root: &Path, files: &[std::path::PathBuf], errors: &mut Vec<String>) {
    for file in files {
        let rel = file.trim_prefix(root);
        let rel = rel.to_string_lossy().replace('\\', "/");
        if !crate::web_strings::is_web_production(&rel) || ALLOWED.iter().any(|(f, _)| *f == rel) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            if flagged(line) {
                errors.push(format!(
                    "{rel}:{}: found a comparison with a subsystem name string; the API returns what depends on the subsystem (a flag on the DTO, like `PartRow.structural`), or register a dedicated view in `features/systems/dedicated.tsx` (layer responsibilities in CONTRIBUTING.md)",
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
    fn comparisons_with_a_subsystem_literal_are_found() {
        for line in [
            "  if (system === \"power\" && tab === \"series\") return x;",
            "structure: p.system === \"structure\",",
            "return tab === \"modes\" && system !== 'aocs';",
            "const x = \"thermal\" == part?.system;",
            "{!system || activeSystem === `power` ? a : b}",
        ] {
            assert!(flagged(line), "{line}");
        }
    }

    #[test]
    fn other_code_is_not_flagged() {
        for line in [
            "p.system === system",
            "x.system === null",
            "n.kind === \"power\"",
            "if (tab === \"modes\") return;",
            "  // system === \"power\" は API が返す",
            "   * `p.system === \"structure\"` の説明",
            "const t = { system: \"aocs\" };",
            "const s: System = \"power\";",
            "view === \"power\" ? a : b",
            "const f = (system: System) => system;",
            "x.systems === \"a\"",
        ] {
            assert!(!flagged(line), "{line}");
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
