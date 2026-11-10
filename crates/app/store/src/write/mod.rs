//! 書き込み: 変更をファイル単位で組み立て（[`Edits`]）、まとめて反映する（[`commit`]）。
//!
//! | モジュール | 中身 |
//! |---|---|
//! | `doc` | toml_edit の小さな道具（装飾を保った値の置き換え・揃えた挿入・表の確保・キーの名前の変更・配列の要素の置き換え） |
//! | `catalog` | カタログ品のファイルの正規形とフィールド |
//! | `part` | 部品ファイルの編集（属性・フィールド・取付・ポート）と新しい部品ファイル |
//! | `entities` | モード・ネット・定義・設定のファイルの編集と新しいファイル |
//! | `project` | プロジェクトのファイル（`fune.toml`・`budget/`）の新しい本文（`fune init` の雛形も） |
//! | `records` | ケース・出典・ハーネス（serde の形のファイル）の書き込み |
//! | `refs` | 名前の変更・削除に伴う参照の書き換え（参照の所在は model の `Design::references`） |
//! | `commit` | tmp ＋ rename、失敗したら戻す |
//!
//! 編集は既存の行の装飾（空白・コメント）を保ち、変えた値の行だけが変わるようにする。
//! 計画した変更の列を（ディスクに書かずに）ファイルの集合に当てるのは [`apply_changes`]。

pub mod catalog;
mod commit;
pub mod doc;
pub mod entities;
pub mod part;
pub mod project;
pub mod records;
pub mod refs;

pub use commit::commit;

use crate::error::StoreError;
use crate::fileset::FileSet;
use serde::{Deserialize, Serialize};
use toml_edit::DocumentMut;
use ts_rs::TS;

/// ファイル 1 つの変更。`before` が None なら新規、`after` が None なら削除。移動は削除＋新規。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct FileChange {
    /// ルートからの相対パス（`/` 区切り）
    pub path: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// 変更の列をファイルの集合に当てる（ディスクには書かない）。**計画の変更を集合に当てるのはここだけ**
/// （下書きを当てたスナップショット・保存の診断の抜粋）。削除は集合から消す。
pub fn apply_changes(files: &mut FileSet, changes: &[FileChange]) {
    for change in changes {
        match &change.after {
            Some(text) => files.insert(change.path.clone(), text.clone()),
            None => {
                files.remove(&change.path);
            }
        }
    }
}

/// 編集中のファイルの集合（元の集合と、変更後の集合）。ディスクには書かない。
#[derive(Debug, Clone)]
pub struct Edits {
    base: FileSet,
    current: FileSet,
}

impl Edits {
    /// 元の集合から始める。
    pub fn new(base: &FileSet) -> Self {
        Self {
            base: base.clone(),
            current: base.clone(),
        }
    }

    /// 変更後の集合。
    pub fn files(&self) -> &FileSet {
        &self.current
    }

    /// 変更後にあるか。
    pub fn exists(&self, path: &str) -> bool {
        self.current.contains(path)
    }

    /// 新しいファイルを作る（既にあれば誤り）。
    pub fn create(&mut self, path: &str, text: String) -> Result<(), StoreError> {
        if self.current.contains(path) {
            return Err(StoreError::AlreadyExists { path: path.into() });
        }
        self.current.insert(path, text);
        Ok(())
    }

    /// ファイルを消す。
    pub fn remove(&mut self, path: &str) -> Result<String, StoreError> {
        self.current
            .remove(path)
            .ok_or_else(|| StoreError::Missing { path: path.into() })
    }

    /// ファイルを移す（git では rename になる）。
    pub fn rename(&mut self, from: &str, to: &str) -> Result<(), StoreError> {
        if from == to {
            return Ok(());
        }
        if self.current.contains(to) {
            return Err(StoreError::AlreadyExists { path: to.into() });
        }
        let text = self.remove(from)?;
        self.current.insert(to, text);
        Ok(())
    }

    /// ディレクトリの中身をまとめて移す（組立品の子部品など）。
    pub fn rename_dir(&mut self, from: &str, to: &str) -> Result<(), StoreError> {
        let prefix = format!("{from}/");
        let moved: Vec<String> = self
            .current
            .paths()
            .filter(|p| p.starts_with(&prefix))
            .map(str::to_string)
            .collect();
        for path in moved {
            let target = format!("{to}/{}", &path[prefix.len()..]);
            self.rename(&path, &target)?;
        }
        Ok(())
    }

    /// TOML のファイルを toml_edit で編集する。
    pub fn edit<T>(
        &mut self,
        path: &str,
        f: impl FnOnce(&mut DocumentMut) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        let text = self
            .current
            .get(path)
            .ok_or_else(|| StoreError::Missing { path: path.into() })?;
        let mut doc = doc::parse(path, text)?;
        let out = f(&mut doc)?;
        self.current.insert(path, doc.to_string());
        Ok(out)
    }

    /// ファイルがあれば編集し、無ければ空の文書から作る。
    pub fn edit_or_create<T>(
        &mut self,
        path: &str,
        f: impl FnOnce(&mut DocumentMut) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        if !self.current.contains(path) {
            self.current.insert(path, String::new());
        }
        self.edit(path, f)
    }

    /// 元の集合との差分（パスの順）。
    pub fn changes(&self) -> Vec<FileChange> {
        let mut out = Vec::new();
        for (path, before) in self.base.iter() {
            match self.current.get(path) {
                Some(after) if after == before => {}
                after => out.push(FileChange {
                    path: path.to_string(),
                    before: Some(before.to_string()),
                    after: after.map(str::to_string),
                }),
            }
        }
        for (path, after) in self.current.iter() {
            if !self.base.contains(path) {
                out.push(FileChange {
                    path: path.to_string(),
                    before: None,
                    after: Some(after.to_string()),
                });
            }
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }
}
