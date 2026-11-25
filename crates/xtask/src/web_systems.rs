//! Web が系（サブシステム）の名前を自分で見て決めていないかの検査。
//!
//! CONTRIBUTING.md の層の責務: `web/` は系の名前の意味を知らない。`system === "power"`・`p.system !== "structure"` のように、
//! 系を表す識別子（`system`・`p.system`・`activeSystem` など、名前が `system` で終わるもの）を文字列と比べる書き方を違反にする。
//! 系で何かを決めたいときは、判定を `fune-service` が返す（例: `PartRow.structural`・`DefsView.mode_levels`）か、
//! 系ごとの登録は登録表（`features/systems/dedicated.tsx` の `SYSTEM_REGISTRY`）に系をキーとして足す（キーは比較ではない）。
//!
//! もう 1 つ、系をキーにした表（`Record<System, …>`・`Partial<Record<System, …>>`）を書いてよいファイルも [`TABLES`] に限る。
//! 系ごとの登録は `features/systems/dedicated.tsx`（一部の系だけが持つもの）と `lib/systems.ts`（全系に必要な色とアイコン）の 2 か所だけで、
//! ほかの所に系の表を足さない。[`TABLES`] の各ファイルに表が 1 つも無ければ、規則が何にも当たっていないとして誤りにする。
//!
//! テスト以外の `web/src/` のコード（生成物 `api/gen/` を除く）だけを見る。比較を許すファイルは [`ALLOWED`] に理由付きで置く。
//! 役割名の比較は [`crate::web_roles`]。

use crate::web_roles::{
    comparisons, ends_with_quote, last_segment, starts_with_quote, trailing_ident,
};
use std::path::Path;

/// 系の名前の比較を許すファイルと理由。登録表は系をキーに持つだけで比較は書かないので、いまは空。
const ALLOWED: &[(&str, &str)] = &[];

/// 系をキーにした表を書いてよいファイルと理由。
const TABLES: &[(&str, &str)] = &[
    (
        "web/src/features/systems/dedicated.tsx",
        "the typed registration table of per-system views (tabs, actions, extras, chunks)",
    ),
    (
        "web/src/lib/systems.ts",
        "the colour and icon every system needs (shared by the nav, the diagram and the 3D, which `features/` cannot be imported from)",
    ),
];

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

/// 系をキーにした型の表（`Record<System,`・`Record<System ` の書き出し）か。
fn system_table(line: &str) -> bool {
    let t = line.trim_start();
    !(t.starts_with("//") || t.starts_with('*') || t.starts_with("/*"))
        && (line.contains("Record<System,") || line.contains("Record<System "))
}

/// コメントの行は見ない。
fn flagged(line: &str) -> bool {
    let t = line.trim_start();
    !(t.starts_with("//") || t.starts_with('*') || t.starts_with("/*")) && system_comparison(line)
}

/// 違反を `errors` に積む。`files` はワークスペースのファイル一覧。
pub fn check(root: &Path, files: &[std::path::PathBuf], errors: &mut Vec<String>) {
    check_tables_in_use(root, errors);
    for file in files {
        let rel = file.trim_prefix(root);
        let rel = rel.to_string_lossy().replace('\\', "/");
        if !crate::web_strings::is_web_production(&rel) || ALLOWED.iter().any(|(f, _)| *f == rel) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        let table_ok = TABLES.iter().any(|(f, _)| *f == rel);
        for (n, line) in text.lines().enumerate() {
            if !table_ok && system_table(line) {
                errors.push(format!(
                    "{rel}:{}: found a table keyed by subsystem (`Record<System, …>`); per-system registrations live only in `features/systems/dedicated.tsx` and `lib/systems.ts`",
                    n + 1
                ));
            }
            if flagged(line) {
                errors.push(format!(
                    "{rel}:{}: found a comparison with a subsystem name string; the API returns what depends on the subsystem (a flag on the DTO, like `PartRow.structural`), or register a dedicated view in `features/systems/dedicated.tsx` (layer responsibilities in CONTRIBUTING.md)",
                    n + 1
                ));
            }
        }
    }
}

/// [`TABLES`] の各ファイルに系の表が実際にあるか（規則が何にも当たっていない状態を誤りにする）。
fn check_tables_in_use(root: &Path, errors: &mut Vec<String>) {
    for (rel, _) in TABLES {
        let found = std::fs::read_to_string(root.join(rel))
            .map(|text| text.lines().any(system_table))
            .unwrap_or(false);
        if !found {
            errors.push(format!(
                "{rel}: listed in `web_systems::TABLES` but has no table keyed by subsystem (`Record<System, …>`); remove it from the list"
            ));
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
    fn tables_keyed_by_subsystem_are_found() {
        for line in [
            "const tones: Record<System, Tone> = {",
            "} as const satisfies Partial<Record<System, SystemRegistration>>;",
            "const x: Record<System , number> = {};",
        ] {
            assert!(system_table(line), "{line}");
        }
        for line in [
            "// Record<System, Tone> の説明",
            "   * `Record<System, X>`",
            "const x: Record<SystemTab, number> = {};",
            "type T = Record<string, System>;",
        ] {
            assert!(!system_table(line), "{line}");
        }
    }

    #[test]
    fn registered_table_files_exist_and_carry_a_reason() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut errors = Vec::new();
        check_tables_in_use(&root, &mut errors);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(
            TABLES
                .iter()
                .all(|(f, why)| f.starts_with("web/src/") && !why.is_empty())
        );
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
