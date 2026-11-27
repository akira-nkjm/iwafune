//! バイナリに同梱したデモのプロジェクト（`fixtures/demo-6u` の全ファイル。`fune init --demo`）。
//! 同梱の表は `build.rs` が作る（パスの昇順・`.fune/` は含めない）。実行時にリポジトリは要らない。

use crate::error::StoreError;
use crate::layout::PROJECT_FILE;
use crate::write::{FileChange, commit};
use std::path::Path;

/// 同梱したファイル（ルートからの相対パス（`/` 区切り）, 本文）。パスの昇順。
pub const FILES: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/demo_files.rs"));

/// `dir` にデモのプロジェクトを書く。作ったファイルのパス（昇順）を返す。
///
/// `fune.toml` が既にあれば誤り。デモが書くファイルのどれかが既にあっても誤り（何も書かない。上書きしない）。
pub fn init(dir: &Path) -> Result<Vec<String>, StoreError> {
    if dir.join(PROJECT_FILE).exists() {
        return Err(StoreError::AlreadyExists {
            path: dir.join(PROJECT_FILE).display().to_string(),
        });
    }
    if let Some((path, _)) = FILES.iter().find(|(path, _)| dir.join(path).exists()) {
        return Err(StoreError::AlreadyExists {
            path: dir.join(path).display().to_string(),
        });
    }
    let changes: Vec<FileChange> = FILES
        .iter()
        .map(|(path, text)| FileChange {
            path: (*path).to_owned(),
            before: None,
            after: Some((*text).to_owned()),
        })
        .collect();
    commit(dir, &changes)?;
    Ok(changes.into_iter().map(|c| c.path).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// 走査したファイル（`.fune/` と `.DS_Store` を除く）→ 本文。
    fn read_tree(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if name == ".fune" || name == ".DS_Store" {
                continue;
            }
            if path.is_dir() {
                read_tree(root, &path, out);
            } else {
                let rel = path.strip_prefix(root).unwrap();
                let rel: Vec<_> = rel.iter().map(|p| p.to_string_lossy()).collect();
                out.insert(rel.join("/"), std::fs::read(&path).unwrap());
            }
        }
    }

    fn fixture_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")
    }

    /// 同梱の中身は fixtures/demo-6u と、パスの集合もバイト列も同じ（ずれたら失敗する）。
    #[test]
    fn bundle_equals_the_fixture() {
        let mut fixture = BTreeMap::new();
        read_tree(&fixture_dir(), &fixture_dir(), &mut fixture);
        let bundled: BTreeMap<String, Vec<u8>> = FILES
            .iter()
            .map(|(p, t)| ((*p).to_owned(), t.as_bytes().to_vec()))
            .collect();
        assert_eq!(bundled.len(), FILES.len(), "duplicate paths");
        assert_eq!(
            bundled.keys().collect::<Vec<_>>(),
            fixture.keys().collect::<Vec<_>>()
        );
        for (path, bytes) in &fixture {
            assert!(bundled[path] == *bytes, "{path} differs");
        }
        assert!(FILES.windows(2).all(|w| w[0].0 < w[1].0), "not sorted");
        assert!(FILES.iter().any(|(p, _)| *p == PROJECT_FILE));
    }

    #[test]
    fn writes_every_file() {
        let dir = tempfile::tempdir().unwrap();
        let created = init(dir.path()).unwrap();
        assert_eq!(created.len(), FILES.len());
        let mut written = BTreeMap::new();
        read_tree(dir.path(), dir.path(), &mut written);
        for (path, text) in FILES {
            assert_eq!(written[*path], text.as_bytes(), "{path}");
        }
        assert_eq!(written.len(), FILES.len(), "extra files were written");
    }

    #[test]
    fn refuses_an_existing_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(PROJECT_FILE), "x").unwrap();
        let err = init(dir.path()).unwrap_err();
        assert!(matches!(err, StoreError::AlreadyExists { .. }), "{err}");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    /// 衝突するファイルが 1 つでもあれば、何も書かず、そのファイルも変えない。
    #[test]
    fn refuses_a_conflicting_file_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let (conflict, _) = FILES.iter().find(|(p, _)| p.contains('/')).unwrap();
        let path = dir.path().join(conflict);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "mine").unwrap();
        let err = init(dir.path()).unwrap_err();
        assert!(matches!(err, StoreError::AlreadyExists { .. }), "{err}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "mine");
        assert!(!dir.path().join(PROJECT_FILE).exists());
    }
}
