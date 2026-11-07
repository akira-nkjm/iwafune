//! 読み書き・revision の共有ファイル集合。
use std::collections::BTreeMap;
/// 相対パス（`/` 区切り）→ 本文。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileSet {
    files: BTreeMap<String, String>,
}

impl FileSet {
    /// 空の集合。
    pub fn new() -> Self {
        Self::default()
    }

    /// 本文。
    pub fn get(&self, path: &str) -> Option<&str> {
        self.files.get(path).map(String::as_str)
    }

    /// あるか。
    pub fn contains(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    /// 書く（あれば置き換える）。
    pub fn insert(&mut self, path: impl Into<String>, contents: impl Into<String>) {
        self.files.insert(path.into(), contents.into());
    }

    /// 消す。
    pub fn remove(&mut self, path: &str) -> Option<String> {
        self.files.remove(path)
    }

    /// (相対パス, 本文) をパスの順に列挙する。
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.files.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// 相対パスをパスの順に列挙する。
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    /// ファイルの数。
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// 空か。
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}
