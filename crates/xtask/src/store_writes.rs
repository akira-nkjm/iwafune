//! 設計ファイルの書き込みの入口の検査（CONTRIBUTING.md の「構成のルール」5）。
//!
//! 設計データへの編集は Op（`fune-ops` の `apply`）だけを通す。`fune_store::write::commit` を呼んでよいのは
//! `fune-store` と `fune-ops` の中だけで、そのほかの crate のテスト以外のコードが呼んだら違反にする。
//! Op にしない書き込みは [`EXCEPTIONS`] に理由付きで挙げたファイルだけが持つ（足すときは registry.md と CONTRIBUTING.md の例外の文も直す）。

use crate::single_home::production_text;
use std::path::{Path, PathBuf};

/// `commit` を呼んでよい crate のディレクトリ（store 自身と、Op の適用・取り消し・やり直し）。
const HOMES: &[&str] = &["crates/app/store/", "crates/app/ops/", "crates/xtask/"];

/// Op を通さずに書いてよいファイルと理由。
const EXCEPTIONS: &[(&str, &str)] = &[
    (
        "crates/app/service/src/format.rs",
        "`fune fmt`: rewrites every file into canonical form without changing the design (not an edit)",
    ),
    (
        "crates/app/service/src/shared_catalog.rs",
        "`fune catalog fetch`: writes only the derived `catalog.lock` decided by fetching the shared catalogs (not a design edit)",
    ),
];

/// 違反を `errors` に積む。
pub fn check(root: &Path, files: &[PathBuf], errors: &mut Vec<String>) {
    for file in files {
        let rel = file.trim_prefix(root).to_string_lossy().replace('\\', "/");
        if !rel.starts_with("crates/") || !rel.ends_with(".rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        if let Some(message) = violation(&rel, &text) {
            errors.push(message);
        }
    }
}

/// そのファイルが Op を通さない書き込みの入口を持っていれば、その説明。
fn violation(rel: &str, text: &str) -> Option<String> {
    if HOMES.iter().any(|h| rel.starts_with(h)) || EXCEPTIONS.iter().any(|(p, _)| *p == rel) {
        return None;
    }
    let prod = production_text(rel, text)?;
    calls_commit(prod).then(|| {
        format!(
            "{rel}: calls `fune_store::write::commit`. Design edits go through an Op (apply in fune-ops). \
             Writes that are not Ops are listed with a reason in EXCEPTIONS of crates/xtask/src/store_writes.rs"
        )
    })
}

/// `write::commit`・`fune_store::commit` の呼び出しか、`use fune_store::…commit…` の取り込みがあるか。
fn calls_commit(text: &str) -> bool {
    if text.contains("write::commit") || text.contains("fune_store::commit") {
        return true;
    }
    text.split(';').any(|statement| {
        let statement = statement.trim_start();
        statement.starts_with("use ")
            && statement.contains("fune_store")
            && statement
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .any(|word| word == "commit")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CALL: &str = "fn f() { fune_store::write::commit(root, &changes); }";

    #[test]
    fn commit_outside_store_and_ops_is_rejected() {
        assert!(violation("crates/app/server/src/routes.rs", CALL).is_some());
        assert!(violation("crates/app/service/src/edit.rs", CALL).is_some());
        assert!(
            violation(
                "crates/app/service/src/edit.rs",
                "use fune_store::{commit, load};"
            )
            .is_some()
        );
        assert!(violation("crates/app/service/src/edit.rs", "fune_store::commit(a, b)").is_some());
    }

    #[test]
    fn store_ops_exceptions_and_tests_are_allowed() {
        assert!(violation("crates/app/ops/src/apply.rs", CALL).is_none());
        assert!(violation("crates/app/store/src/project.rs", CALL).is_none());
        for (path, _) in EXCEPTIONS {
            assert!(violation(path, CALL).is_none());
        }
        assert!(violation("crates/app/service/tests/x.rs", CALL).is_none());
        let in_tests = format!("fn f() {{}}\n#[cfg(test)]\nmod tests {{ {CALL} }}");
        assert!(violation("crates/app/service/src/edit.rs", &in_tests).is_none());
    }

    #[test]
    fn a_commit_variable_is_not_a_call() {
        let text = "use fune_store::load;\nlet commit = mirror.resolve(rev)?;";
        assert!(violation("crates/app/service/src/shared_catalog/git.rs", text).is_none());
    }
}
