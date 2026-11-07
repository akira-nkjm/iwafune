//! ネットの型（部品と運用エンティティが共有）。
use super::defs::NetKind;
use crate::{id::Id, value::Quantity};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
/// 電気的な接続（論理。`nets/<ID>.toml`）。所属する部品は部品側のポートに書く。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Net {
    #[serde(default)]
    pub id: Id<Net>,
    pub kind: NetKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// 電源の供給元のポート（`"PCDU.OUT3"`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// レール電圧
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rail: Option<Quantity>,
    /// 電流制限
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<Quantity>,
    /// バスのマスターのポート（`"OBC.CAN1"`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master: Option<String>,
}

impl Net {
    /// 種類だけを決めた空のネット。
    pub fn new(id: impl Into<Id<Net>>, kind: NetKind) -> Self {
        Self {
            id: id.into(),
            kind,
            label: None,
            source: None,
            rail: None,
            limit: None,
            protocol: None,
            bitrate: None,
            master: None,
        }
    }
}

/// ネットの型付きのフィールド（`SetNetField` の Op と store の書き込みが共有する）。None は消す（`kind` は消せない）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "field", content = "value", rename_all = "snake_case")]
pub enum NetField {
    Kind(NetKind),
    Label(Option<String>),
    Source(Option<String>),
    Rail(Option<Quantity>),
    Limit(Option<Quantity>),
    Protocol(Option<String>),
    Bitrate(Option<Quantity>),
    Master(Option<String>),
}

impl NetField {
    /// フィールド名（ファイルのキー・CLI の旗の名前）。
    pub fn name(&self) -> &'static str {
        match self {
            NetField::Kind(_) => "kind",
            NetField::Label(_) => "label",
            NetField::Source(_) => "source",
            NetField::Rail(_) => "rail",
            NetField::Limit(_) => "limit",
            NetField::Protocol(_) => "protocol",
            NetField::Bitrate(_) => "bitrate",
            NetField::Master(_) => "master",
        }
    }
}
