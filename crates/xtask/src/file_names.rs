//! ファイル名の大文字・小文字: 同じディレクトリで、最後の拡張子を除いた名前が大文字・小文字だけ違うファイルを拒む。
//!
//! 大文字と小文字を区別しないファイルシステム（macOS の既定）では、拡張子なしの import（`@/kit/tree/Tree`）が
//! 別のファイル（`tree.ts`）に当たってしまう。CI（Linux）では区別されるので気づけない。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `files` のうち、同じディレクトリで名前が大文字・小文字だけ違う組を `errors` に足す。
pub fn check(root: &Path, files: &[PathBuf], errors: &mut Vec<String>) {
    let mut seen: BTreeMap<(PathBuf, String), Vec<String>> = BTreeMap::new();
    for file in files {
        let rel = file.strip_prefix(root).unwrap_or(file);
        let (Some(dir), Some(stem)) = (rel.parent(), rel.file_stem().and_then(|s| s.to_str()))
        else {
            continue;
        };
        let names = seen
            .entry((dir.to_path_buf(), stem.to_lowercase()))
            .or_default();
        if !names.iter().any(|n| n == stem) {
            names.push(stem.to_owned());
        }
    }
    for ((dir, _), names) in seen {
        if names.len() > 1 {
            errors.push(format!(
                "{} has names that differ only in case: {} (extensionless imports are mixed up on macOS)",
                dir.display(),
                names.join(", ")
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(paths: &[&str]) -> Vec<String> {
        let root = Path::new("/repo");
        let files: Vec<PathBuf> = paths.iter().map(|p| root.join(p)).collect();
        let mut errors = Vec::new();
        check(root, &files, &mut errors);
        errors
    }

    #[test]
    fn rejects_names_that_differ_only_in_case() {
        let errors = run(&["web/src/kit/tree/tree.ts", "web/src/kit/tree/Tree.tsx"]);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("tree") && errors[0].contains("Tree"));
    }

    #[test]
    fn allows_same_name_with_other_extensions_and_other_directories() {
        assert!(
            run(&[
                "Cargo.toml",
                "Cargo.lock",
                "crates/app/model/src/builtin/labels.rs",
                "crates/app/model/src/builtin/labels.toml",
                "web/src/a/tree.ts",
                "web/src/b/Tree.tsx",
                "web/src/kit/tree/tree-nodes.ts",
                "web/src/kit/tree/Tree.tsx",
            ])
            .is_empty()
        );
    }
}
