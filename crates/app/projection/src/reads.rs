//! 射影が読んだものの記録（部品の属性・設定・条件・モード・解析ケース）。**記録するのは `Context` の読み出しの入口だけ**
//! （`Context::attr`・`Context::note` など。context.rs）。service が解析ごとの「読んだ値」を集め、値 → 解析のつながりを作る。
//! 記録は入力の組み立てを変えない（読んだ対象を足すだけ）。

use fune_model::Target;
use std::collections::BTreeSet;
use std::sync::{Mutex, PoisonError};

/// 読んだ対象の集まり（重複なし・順序付き）。`Context::with_reads` で文脈に渡すと、その文脈の読み出しを記録する。
#[derive(Debug, Default)]
pub struct ReadLog(Mutex<BTreeSet<Target>>);

impl ReadLog {
    /// 空の記録。
    pub fn new() -> Self {
        Self::default()
    }

    /// 1 件記録する。
    pub(crate) fn push(&self, target: Target) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(target);
    }

    /// 記録した対象（順序付き）。
    pub fn into_targets(self) -> BTreeSet<Target> {
        self.0.into_inner().unwrap_or_else(PoisonError::into_inner)
    }
}
