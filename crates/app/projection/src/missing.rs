//! 不足入力。**解析に必要なのに設計データに無いものを、対象（`fune_model::Target`）で名指しする。**
//! 文字列のパスは対象の `Display` から作るだけで、読み直さない。

use fune_model::Target;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 対象を名指しできない不足（入力の組み立ての内部の誤り）の表示。
const INTERNAL: &str = "(内部)";

/// 足りない入力 1 件。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
pub struct MissingInput {
    /// 部品のファイルの中の入力ならその部品 ID（[`Target::part`]）。設定・解析ケース・「どれかの部品」は None。
    pub part: Option<String>,
    /// 表示用のパス（対象の `Display`。例: `"RW_X.max_momentum"`、`"conditions.orbit.altitude"`、`"<部品>.power"`）
    pub path: String,
    /// 足りない入力の対象（構造）。None は対象を名指しできない内部の不足
    pub target: Option<Target>,
    /// 何に使うか・何が要るか（例: `"RW の角運動量の上限"`）
    pub reason: String,
}

impl MissingInput {
    fn new(target: Option<Target>, reason: String) -> Self {
        Self {
            part: target.as_ref().and_then(Target::part).map(str::to_owned),
            path: target
                .as_ref()
                .map_or_else(|| INTERNAL.to_owned(), Target::to_string),
            target,
            reason,
        }
    }
}

/// 足りない入力の集まり。射影の途中で見つけたものをすべて集めてから返す（最初の 1 件で止めない）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MissingInputs(Vec<MissingInput>);

impl MissingInputs {
    /// 空の集まり。
    pub fn new() -> Self {
        Self::default()
    }

    /// 1 件足す。
    pub fn push(&mut self, target: impl Into<Target>, reason: impl Into<String>) {
        self.0
            .push(MissingInput::new(Some(target.into()), reason.into()));
    }

    /// `Option` の値を取り出し、無ければ不足として記録して `None` を返す。
    pub fn need<T>(
        &mut self,
        value: Option<T>,
        target: impl Into<Target>,
        reason: impl Into<String>,
    ) -> Option<T> {
        if value.is_none() {
            self.push(target, reason);
        }
        value
    }

    /// 別の集まりを足す。
    pub fn extend(&mut self, other: MissingInputs) {
        self.0.extend(other.0);
    }

    /// 不足が無いか。
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 不足があればエラー、無ければ `value` を返す。
    pub fn finish<T>(self, value: impl FnOnce() -> Option<T>) -> Result<T, MissingInputs> {
        if !self.is_empty() {
            return Err(self);
        }
        match value() {
            Some(v) => Ok(v),
            None => {
                let mut m = self;
                let reason = "入力を組み立てられませんでした".to_owned();
                m.0.push(MissingInput::new(None, reason));
                Err(m)
            }
        }
    }

    /// 並べ替えて重複を除いた一覧。
    pub fn into_vec(mut self) -> Vec<MissingInput> {
        self.0.sort();
        self.0.dedup();
        self.0
    }
}

impl std::fmt::Display for MissingInputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let paths: Vec<&str> = self.0.iter().map(|m| m.path.as_str()).collect();
        write!(f, "不足入力: {}", paths.join(", "))
    }
}
