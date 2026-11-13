//! 出典の台帳（`sources/<ID>.toml`。docs/model.md §7.8）。
//!
//! serde の形がそのままファイルの形（`id` はファイル名から決まるので書かない）。欄の形（日付・sha256）の検査は
//! `crate::check` の `sources` が持つ。

use crate::enum_text::enum_text;
use crate::id::Id;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

enum_text! {
    /// 出典の種類（根拠の強さの順: placeholder が最も弱い）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
    pub enum SourceKind: "出典の種類" {
        Placeholder = "placeholder",
        Judgement = "judgement",
        Analysis = "analysis",
        Datasheet = "datasheet",
        Measured = "measured",
    }
}

enum_text! {
    /// 資料の配布の区分（誰が資料を手に入れられるか）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
    pub enum Distribution: "配布の区分" {
        /// 誰でも取れる（公開の Web ページなど）
        Public = "public",
        /// ベンダーに依頼して配られる
        OnRequest = "on_request",
        /// 公開できない（NDA など）。共有カタログの品の値の出典にしない
        Restricted = "restricted",
    }
}

/// 出典の台帳（`sources/<ID>.toml`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(rename = "EvidenceSource")]
pub struct Source {
    #[serde(default)]
    pub id: Id<Source>,
    pub kind: SourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// 参照先（URL・リポジトリ内のパス・文書番号など）。データシートの場所はここだけに書く
    /// （カタログ品の `datasheet` はこの出典の ID を指す）
    #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// 資料の版（`Rev. C`・`v1.4` など。書かれたまま）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// 資料を取得した日（`2026-10-15` の形）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieved: Option<String>,
    /// 取得したファイル（PDF など）の SHA-256（64 桁の 16 進数）。版番号を変えずに差し替えられたことに気づくため
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// 配布の区分
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distribution: Option<Distribution>,
    /// 日付（試験・解析・判断をした日。`2026-10-15` の形）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// 記入者
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Source {
    /// 種類だけの出典（ほかの欄は空）。
    pub fn new(id: impl Into<Id<Source>>, kind: SourceKind) -> Self {
        Self {
            id: id.into(),
            kind,
            label: None,
            reference: None,
            revision: None,
            retrieved: None,
            sha256: None,
            distribution: None,
            date: None,
            by: None,
            note: None,
        }
    }

    /// 参照先が Web の URL（`http://`・`https://`）なら、その URL（GUI がリンクにする）。
    /// **参照先を URL とみなすかの判定はここだけ。**
    pub fn web_link(&self) -> Option<&str> {
        self.reference
            .as_deref()
            .filter(|r| r.starts_with("https://") || r.starts_with("http://"))
    }

    /// 公開できない（`distribution = "restricted"`）資料か。
    pub fn is_restricted(&self) -> bool {
        self.distribution == Some(Distribution::Restricted)
    }
}
