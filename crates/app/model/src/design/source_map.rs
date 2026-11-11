//! エンティティ → ファイル、ファイル内のキー → 行の対応（診断の位置を示すため）。
//!
//! store が読み込みのときに作る。model は位置を引くだけで、ファイルの形式は知らない。

pub use crate::diagnostic::Loc;
use crate::path::SettingPath;
use std::collections::BTreeMap;

/// 位置を引くためのエンティティの名前。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntityKey {
    Project,
    Allocation,
    Policy,
    Attribute(String),
    Kind(String),
    Catalog(String),
    Part(String),
    Mode(String),
    Net(String),
    Harness(String),
    Case(String),
    Source(String),
    /// 設定ファイル（`mission.orbit`・`mission.ground.<局の ID>`・`analysis.power` のパス）
    Setting(SettingPath),
    /// 図の配置（`view/<名前>.toml`）
    View(String),
}

/// 位置の索引。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SourceMap {
    files: BTreeMap<EntityKey, String>,
    keys: BTreeMap<String, BTreeMap<String, (u32, u32)>>,
}

impl SourceMap {
    /// エンティティが書かれているファイルを登録する。
    pub fn insert_file(&mut self, entity: EntityKey, file: impl Into<String>) {
        self.files.insert(entity, file.into());
    }

    /// ファイル内のキー（ドット区切り）の位置を登録する。
    pub fn insert_key(&mut self, file: &str, key: impl Into<String>, line: u32, column: u32) {
        self.keys
            .entry(file.to_string())
            .or_default()
            .insert(key.into(), (line, column));
    }

    /// エンティティが書かれているファイル。
    pub fn file_of(&self, entity: &EntityKey) -> Option<&str> {
        self.files.get(entity).map(String::as_str)
    }

    /// エンティティの全ファイル（エンティティ → ファイル）。
    pub fn entities(&self) -> impl Iterator<Item = (&EntityKey, &str)> {
        self.files.iter().map(|(k, v)| (k, v.as_str()))
    }

    /// ファイル内のキーの位置。キーの列の最長の接頭辞で見つかった位置を返す。
    pub fn locate_in_file(&self, file: &str, keys: &[&str]) -> Loc {
        let index = self.keys.get(file);
        let mut found = None;
        for n in (1..=keys.len()).rev() {
            let dotted = keys[..n].join(".");
            if let Some(pos) = index.and_then(|i| i.get(&dotted)) {
                found = Some(*pos);
                break;
            }
        }
        Loc {
            file: file.to_string(),
            line: found.map(|p| p.0),
            column: found.map(|p| p.1),
        }
    }

    /// エンティティのキーの位置（エンティティのファイルが分からなければ None）。
    pub fn locate(&self, entity: &EntityKey, keys: &[&str]) -> Option<Loc> {
        self.file_of(entity)
            .map(|file| self.locate_in_file(file, keys))
    }
}
