//! 共有カタログ（`[[catalog]] git = …`）の固定（`catalog.lock`）と、読み込みの状態（docs/model.md §12）。
//!
//! lock の型の serde の形がそのままファイルの形（読み書きは fune-store）。fune.toml・lock・キャッシュの突き合わせ
//! （状態の判定）は [`Design::shared_catalog_statuses`] だけ（検査 `check::catalogs` と service の表示が使う）。

use super::{CatalogSource, Design};
use crate::enum_text::enum_text;
use crate::path::split_catalog_ref;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `catalog.lock` 全体（`fune catalog fetch`・`fune catalog update` が書く）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CatalogLock {
    /// `[[catalog]]`（fune.toml の `[[catalog]]` の順）
    #[serde(default, rename = "catalog")]
    pub catalogs: Vec<LockedCatalog>,
}

/// 固定した共有カタログ 1 つ（`catalog.lock` の `[[catalog]]`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct LockedCatalog {
    /// カタログ名（fune.toml の `[[catalog]]` の `name`）
    pub name: String,
    /// 取得したときの `git`（fune.toml と違えば lock が古い）
    pub git: String,
    /// 取得したときの `rev`（ブランチ・タグ・commit。省略は既定のブランチ）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    /// `rev` を解決した commit（16 進数の完全な形）
    pub commit: String,
}

impl CatalogLock {
    /// 名前で引く。
    pub fn get(&self, name: &str) -> Option<&LockedCatalog> {
        self.catalogs.iter().find(|c| c.name == name)
    }
}

impl LockedCatalog {
    /// fune.toml の `[[catalog]]` と `git`・`rev` が同じか（違えば取得し直す）。
    pub fn matches(&self, source: &CatalogSource) -> bool {
        source.git.as_deref() == Some(self.git.as_str()) && source.rev == self.rev
    }
}

impl CatalogSource {
    /// 共有カタログ（書き込めない `git = …` の出所）なら、その取得元。書き込めるかの判定は [`CatalogSource::local_dir`]。
    pub fn shared_git(&self) -> Option<&str> {
        if self.local_dir().is_some() {
            return None;
        }
        self.git.as_deref()
    }
}

enum_text! {
    /// 共有カタログの読み込みの状態。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
    pub enum SharedCatalogState: "shared catalog state" {
        /// lock の commit のキャッシュを読んだ
        Ready = "ready",
        /// `catalog.lock` に記録が無い（`fune catalog fetch` が要る）
        NotLocked = "not_locked",
        /// lock の `git`・`rev` が fune.toml と違う（`fune catalog fetch` か `fune catalog update` が要る）
        LockMismatch = "lock_mismatch",
        /// lock の commit がキャッシュ（`.fune/`）に無い（`fune catalog fetch` が要る）
        NotFetched = "not_fetched",
    }
}

/// 共有カタログ 1 つの状態（fune.toml の `[[catalog]] git = …` 1 つ）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SharedCatalogStatus {
    /// カタログ名
    pub name: String,
    /// fune.toml の `git`
    pub git: String,
    /// fune.toml の `rev`
    pub rev: Option<String>,
    /// lock の commit（lock に無ければ None）
    pub commit: Option<String>,
    pub state: SharedCatalogState,
}

impl Design {
    /// 共有カタログ（fune.toml の順）の lock とキャッシュの状態。**突き合わせの判定はここだけ。**
    pub fn shared_catalog_statuses(&self) -> Vec<SharedCatalogStatus> {
        self.project
            .catalogs
            .iter()
            .filter_map(|c| {
                let git = c.shared_git()?;
                let locked = self.catalog_lock.as_ref().and_then(|l| l.get(&c.name));
                let state = match locked {
                    None => SharedCatalogState::NotLocked,
                    Some(l) if !l.matches(c) => SharedCatalogState::LockMismatch,
                    Some(_) if !self.fetched_catalogs.contains(&c.name) => {
                        SharedCatalogState::NotFetched
                    }
                    Some(_) => SharedCatalogState::Ready,
                };
                Some(SharedCatalogStatus {
                    name: c.name.clone(),
                    git: git.to_owned(),
                    rev: c.rev.clone(),
                    commit: locked.map(|l| l.commit.clone()),
                    state,
                })
            })
            .collect()
    }

    /// `catalog.lock` にあるが、fune.toml の共有カタログに無い記録（`fune catalog fetch` で lock から外す）。
    pub fn unused_lock_entries(&self) -> Vec<&LockedCatalog> {
        let Some(lock) = &self.catalog_lock else {
            return Vec::new();
        };
        lock.catalogs
            .iter()
            .filter(|l| {
                !self
                    .project
                    .catalogs
                    .iter()
                    .any(|c| c.name == l.name && c.shared_git().is_some())
            })
            .collect()
    }

    /// 出所付きの参照（カタログ品の `lab/rw_18mNms`・出典の `lab/ds_rw`）が共有カタログのものなら、そのカタログ。
    /// このプロジェクトのカタログ（`path = …`）の品・プロジェクトの出典（`/` を含まない ID）は None。
    pub fn shared_catalog_of(&self, reference: &str) -> Option<&CatalogSource> {
        let (name, _) = split_catalog_ref(reference)?;
        self.project
            .catalogs
            .iter()
            .find(|c| c.name == name && c.shared_git().is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shared(name: &str, rev: Option<&str>) -> CatalogSource {
        CatalogSource {
            name: name.into(),
            path: None,
            git: Some("https://example.com/catalog.git".into()),
            rev: rev.map(str::to_owned),
        }
    }

    fn locked(name: &str, rev: Option<&str>) -> LockedCatalog {
        LockedCatalog {
            name: name.into(),
            git: "https://example.com/catalog.git".into(),
            rev: rev.map(str::to_owned),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
        }
    }

    /// 状態は lock の有無 → git・rev の一致 → キャッシュの有無の順に決まる。書き込めるカタログは並ばない。
    #[test]
    fn statuses_compare_project_lock_and_cache() {
        let mut design = Design::default();
        design.project.catalogs = vec![
            shared("a", Some("v1")),
            shared("b", Some("v2")),
            shared("c", None),
            shared("d", None),
            CatalogSource {
                name: "local".into(),
                path: Some("catalog".into()),
                git: None,
                rev: None,
            },
        ];
        design.catalog_lock = Some(CatalogLock {
            catalogs: vec![
                locked("b", Some("v1")),
                locked("c", None),
                locked("d", None),
                locked("old", None),
            ],
        });
        design.fetched_catalogs.insert("d".into());
        let states: Vec<(String, SharedCatalogState)> = design
            .shared_catalog_statuses()
            .into_iter()
            .map(|s| (s.name, s.state))
            .collect();
        use SharedCatalogState::*;
        assert_eq!(
            states,
            [
                ("a".into(), NotLocked),
                ("b".into(), LockMismatch),
                ("c".into(), NotFetched),
                ("d".into(), Ready),
            ]
        );
        let unused: Vec<&str> = design
            .unused_lock_entries()
            .iter()
            .map(|l| l.name.as_str())
            .collect();
        assert_eq!(unused, ["old"]);
        assert!(design.shared_catalog_of("d/ds_rw").is_some());
        assert!(design.shared_catalog_of("local/rw").is_none());
        assert!(design.shared_catalog_of("ds_rw").is_none());
    }
}
