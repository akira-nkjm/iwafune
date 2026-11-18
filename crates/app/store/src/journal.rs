//! 編集の履歴（`.fune/history.json`）。取り消し・やり直しのために、編集ごとに「変えたファイルの前後の中身」を控える。
//!
//! 履歴は作業ディレクトリの下にあり（git の管理外）、CLI と GUI が同じものを使う。ここは控えの保存と、
//! 上限（[`MAX_ENTRIES`]・[`MAX_BYTES`]）・取り消しの位置（[`Journal::head`]）の管理だけを持つ。
//! 控えを書き戻す適用（revision の確認を含む）は fune-ops の `history`。
//!
//! 読み書きはプロジェクトのロック（[`crate::lock::ProjectLock`]）を持った状態で行う。
//! 履歴は補助のデータなので、壊れていたり版が違ったりすれば空として読む（編集は止めない）。

use crate::error::StoreError;
use crate::layout::{HISTORY_FILE, WORK_DIR};
use crate::write::FileChange;
use fune_model::design::Revision;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// 控えておく編集の数の上限。超えたら古いものから捨てる。
pub const MAX_ENTRIES: usize = 100;
/// 控えの本文の合計の上限（バイト）。超えたら古いものから捨てる（最新の 1 件は残す）。
pub const MAX_BYTES: usize = 32 * 1024 * 1024;

/// ファイルの形の版。違う版のファイルは空として読む。
const FORMAT_VERSION: u32 = 1;

/// 編集 1 回分の控え。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// 同じ操作をする CLI のコマンド（表示用）
    pub cli: Vec<String>,
    /// 適用した時刻（UNIX 秒）
    pub at: u64,
    /// 適用の前の revision
    pub before: Revision,
    /// 適用の後の revision
    pub after: Revision,
    /// 変えたファイル（前後の中身つき。作った・消した・移したものも含む）
    pub changes: Vec<FileChange>,
}

impl Entry {
    /// 控えている本文の大きさ（バイト）。
    fn size(&self) -> usize {
        self.changes
            .iter()
            .map(|c| {
                c.path.len()
                    + c.before.as_deref().map_or(0, str::len)
                    + c.after.as_deref().map_or(0, str::len)
            })
            .sum()
    }
}

/// 現在の時刻（UNIX 秒）。
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// 編集の履歴。`entries[..head]` が適用済み（取り消せる）、`entries[head..]` が取り消し済み（やり直せる）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    version: u32,
    /// 古い順
    pub entries: Vec<Entry>,
    /// 適用済みの数
    pub head: usize,
}

impl Journal {
    /// 読む。無い・壊れている・版が違うときは空。
    pub fn load(root: &Path) -> Self {
        let parsed = fs::read_to_string(root.join(HISTORY_FILE))
            .ok()
            .and_then(|text| serde_json::from_str::<Journal>(&text).ok())
            .filter(|j| j.version == FORMAT_VERSION && j.head <= j.entries.len());
        parsed.unwrap_or_default()
    }

    /// 書く（tmp に書いて rename。ロックを持って呼ぶ）。
    pub fn save(&self, root: &Path) -> Result<(), StoreError> {
        let dir = root.join(WORK_DIR);
        fs::create_dir_all(&dir).map_err(|e| StoreError::io(&dir, e))?;
        let target = root.join(HISTORY_FILE);
        let tmp = dir.join("history.json.tmp");
        let body = Journal {
            version: FORMAT_VERSION,
            ..self.clone()
        };
        let text = serde_json::to_string(&body).map_err(|e| StoreError::Commit {
            message: format!("Cannot serialize the edit history: {e}"),
        })?;
        fs::write(&tmp, text).map_err(|e| StoreError::io(&tmp, e))?;
        fs::rename(&tmp, &target).map_err(|e| StoreError::io(&target, e))
    }

    /// 新しい編集を控える。取り消し済みの分（やり直し）は捨て、上限を超えた古い分も捨てる。
    pub fn record(&mut self, entry: Entry) {
        self.entries.truncate(self.head);
        self.entries.push(entry);
        let mut total: usize = self.entries.iter().map(Entry::size).sum();
        let mut dropped = 0;
        while self.entries.len() - dropped > 1
            && (self.entries.len() - dropped > MAX_ENTRIES || total > MAX_BYTES)
        {
            total -= self.entries[dropped].size();
            dropped += 1;
        }
        self.entries.drain(..dropped);
        self.head = self.entries.len();
    }

    /// 次に取り消す編集（無ければ None）。
    pub fn undoable(&self) -> Option<&Entry> {
        self.head.checked_sub(1).and_then(|i| self.entries.get(i))
    }

    /// 次にやり直す編集（無ければ None）。
    pub fn redoable(&self) -> Option<&Entry> {
        self.entries.get(self.head)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(n: usize, body: &str) -> Entry {
        Entry {
            cli: vec![format!("fune set {n}")],
            at: 0,
            before: Revision(format!("r{n}")),
            after: Revision(format!("r{}", n + 1)),
            changes: vec![FileChange {
                path: "a.toml".into(),
                before: Some(body.into()),
                after: None,
            }],
        }
    }

    #[test]
    fn new_edit_drops_the_redo_side() {
        let mut j = Journal::default();
        j.record(entry(0, "x"));
        j.record(entry(1, "x"));
        j.head = 1;
        j.record(entry(5, "x"));
        assert_eq!(j.entries.len(), 2);
        assert_eq!(j.head, 2);
        assert!(j.redoable().is_none());
        assert_eq!(j.undoable().map(|e| e.before.0.as_str()), Some("r5"));
    }

    #[test]
    fn entries_are_bounded() {
        let mut j = Journal::default();
        for n in 0..MAX_ENTRIES + 10 {
            j.record(entry(n, "x"));
        }
        assert_eq!(j.entries.len(), MAX_ENTRIES);
        assert_eq!(j.entries[0].before.0, "r10");
    }

    #[test]
    fn entries_are_bounded_by_size_but_keep_the_newest() {
        let mut j = Journal::default();
        let big = "x".repeat(MAX_BYTES / 2 + 1);
        j.record(entry(0, &big));
        j.record(entry(1, &big));
        assert_eq!(j.entries.len(), 1);
        assert_eq!(j.entries[0].before.0, "r1");
    }

    #[test]
    fn round_trips_and_reads_garbage_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let mut j = Journal::default();
        j.record(entry(0, "x"));
        j.save(dir.path()).unwrap();
        let back = Journal::load(dir.path());
        assert_eq!(back.entries, j.entries);
        assert_eq!(back.head, 1);
        fs::write(dir.path().join(HISTORY_FILE), "not json").unwrap();
        assert_eq!(Journal::load(dir.path()), Journal::default());
    }
}
