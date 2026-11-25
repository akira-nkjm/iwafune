//! 実行の記憶（`Session`）。**1 回の読み込み（1 つの revision の設計）に対して、解析・バジェットを回す入れ物。**
//! 設計を読んで検査するのは開くとき 1 回だけ（[`Session::open`]）で、同じ解析・同じケースの結果は 1 回だけ計算して型のまま持ち、
//! 使い回す。要求適合表・設計書の書き出し・電源のモードの表示のように 1 つの依頼で解析をいくつも回す呼び出し側は、
//! ここを開いて渡す。途中で設計が編集されても、1 つの出力に違う revision が混ざらない。
//!
//! - 寿命は 1 つの依頼・1 つのコマンドだけ（依頼をまたぐキャッシュではない。ディスクのキャッシュ（外部データ）には触れない）。
//! - 下書き（保存していない編集を当てた設計）は [`Session::draft`] で、読み込み済みの設計から作る（読み直さない）。同じ編集は同じ記憶を返す。
//! - 何を記憶するかは呼び出し側の module が決める（[`Session::cached`] の鍵と値の型）。解析の実行は `analysis`、バジェットは
//!   `budget` が持ち、この module はそれらに依存しない葉（循環を避ける）。
//! - 単一スレッド（`Rc`・`RefCell`）。解析を並列に回す箇所は無い。

use crate::error::{Result, ServiceError};
use crate::preview::{draft_of, snapshot_with};
use crate::workspace::{Snapshot, Workspace};
use fune_ops::Op;
use serde::Serialize;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

/// 1 つの設計（ある revision）と、その上で回した解析・バジェットの結果の記憶。
pub struct Session {
    ws: Workspace,
    snapshot: Snapshot,
    /// (値の型, 鍵) → 値。鍵は呼び出し側が決める（値の型ごとに名前空間が分かれる）
    memo: RefCell<HashMap<(TypeId, String), Rc<dyn Any>>>,
    /// 保存していない編集（Op の列を JSON にしたもの） → その編集を当てた設計の記憶
    drafts: RefCell<BTreeMap<String, Rc<Session>>>,
}

impl Session {
    /// 設計を読んで開く（`ops` があれば書き込まずに当てたもの。空なら保存済みの設計）。
    /// 編集が通らなければ保存したときと同じエラー。
    pub fn open(ws: &Workspace, ops: &[Op]) -> Result<Rc<Self>> {
        Ok(Self::of(ws, snapshot_with(ws, ops)?))
    }

    /// 読み込み済みのスナップショットで開く。
    pub fn of(ws: &Workspace, snapshot: Snapshot) -> Rc<Self> {
        Rc::new(Self {
            ws: ws.clone(),
            snapshot,
            memo: RefCell::default(),
            drafts: RefCell::default(),
        })
    }

    /// この記憶が持つ設計（読み込みと検査は済んでいる）。
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    /// この設計に `ops` を当てた下書きの記憶（ディスクを読み直さない）。`ops` が空ならこの記憶そのもの。
    /// 同じ `ops` には同じ記憶を返すので、下書きの上の解析も使い回される。
    pub fn draft(self: &Rc<Self>, ops: &[Op]) -> Result<Rc<Self>> {
        if ops.is_empty() {
            return Ok(Rc::clone(self));
        }
        let key = serde_json::to_string(ops).map_err(|e| ServiceError::Internal(e.to_string()))?;
        if let Some(found) = self.drafts.borrow().get(&key) {
            return Ok(Rc::clone(found));
        }
        let draft = Self::of(&self.ws, draft_of(&self.ws, &self.snapshot, ops)?);
        self.drafts.borrow_mut().insert(key, Rc::clone(&draft));
        Ok(draft)
    }

    /// 鍵 `key` の値 `T` を 1 回だけ計算して持つ（2 回目からは同じ値）。鍵は値の型ごとに分かれる。
    /// `compute` の中でこの記憶をもう一度使ってよい（計算中は借りていない）。
    pub fn cached<T: 'static, K: Serialize>(&self, key: &K, compute: impl FnOnce() -> T) -> Rc<T> {
        // 鍵が JSON に書けないときは記憶せずに計算する（鍵は文字列と ID だけなので起きない）
        let Ok(key) = serde_json::to_string(key) else {
            return Rc::new(compute());
        };
        let slot = (TypeId::of::<T>(), key);
        let found = self.memo.borrow().get(&slot).cloned();
        if let Some(any) = found
            && let Ok(value) = any.downcast::<T>()
        {
            return value;
        }
        let value = Rc::new(compute());
        self.memo.borrow_mut().insert(slot, value.clone());
        value
    }

    /// [`Session::cached`] の、失敗しうる計算の版（失敗も記憶する。エラーは複製して返す）。
    pub fn cached_result<T: 'static, E: Clone + 'static, K: Serialize>(
        &self,
        key: &K,
        compute: impl FnOnce() -> Result<T, E>,
    ) -> Result<Rc<T>, E> {
        let stored = self.cached(key, || compute().map(Rc::new));
        Result::clone(&*stored)
    }
}

#[cfg(test)]
mod tests;
