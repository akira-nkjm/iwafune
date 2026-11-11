//! ネットの型（部品と運用エンティティが共有）。
use super::defs::NetKind;
pub use crate::field_names::NetFieldName;
use crate::path::{PortRef, port_text};
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
    /// 電源の供給元のポート（`"PCDU.OUT3"`。読み込むときに `PortRef` にする）
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "port_text::option"
    )]
    #[ts(as = "Option<String>")]
    pub source: Option<PortRef>,
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
    /// バスのマスターのポート（`"OBC.CAN1"`。読み込むときに `PortRef` にする）
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "port_text::option"
    )]
    #[ts(as = "Option<String>")]
    pub master: Option<PortRef>,
}

/// 戻り（GND）のネットとみなす ID の区切り（`_` で区切った語。大文字小文字を無視）。
pub const RETURN_NET_TOKENS: &[&str] = &["GND", "RTN"];

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

    /// 戻り（GND）のネットか。**判定はここだけ**（系統図の GND の層・`NetView.is_return`）。
    /// 電源のネットで、レール電圧が 0 のもの、または ID に [`RETURN_NET_TOKENS`] の語を含むもの（`GND`・`P5V_RTN`）。
    pub fn is_return(&self) -> bool {
        if self.kind != NetKind::Power {
            return false;
        }
        let zero_rail = self.rail.as_ref().is_some_and(|q| q.value == 0.0);
        let named = self.id.as_str().split('_').any(|word| {
            RETURN_NET_TOKENS
                .iter()
                .any(|t| word.eq_ignore_ascii_case(t))
        });
        zero_rail || named
    }
}

/// ネットの型付きのフィールド（`SetNetField` の Op と store の書き込みが共有する）。None は消す（`kind` は消せない）。
/// ポートの参照は JSON でも `"PCDU.OUT3"` の文字列（`fune_model::path::port_text`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "field", content = "value", rename_all = "snake_case")]
pub enum NetField {
    Kind(NetKind),
    Label(Option<String>),
    Source(
        #[serde(with = "port_text::option")]
        #[ts(as = "Option<String>")]
        Option<PortRef>,
    ),
    Rail(Option<Quantity>),
    Limit(Option<Quantity>),
    Protocol(Option<String>),
    Bitrate(Option<Quantity>),
    Master(
        #[serde(with = "port_text::option")]
        #[ts(as = "Option<String>")]
        Option<PortRef>,
    ),
}

impl NetField {
    /// フィールド名（[`NetFieldName`]。名前の一覧は `NetFieldName::NAMES`）。
    /// 名前 → 型付きのフィールドの読み方は `fune_service::edit`（そのテストが全部の名前の往復を確かめる）。
    pub fn key(&self) -> NetFieldName {
        match self {
            NetField::Kind(_) => NetFieldName::Kind,
            NetField::Label(_) => NetFieldName::Label,
            NetField::Source(_) => NetFieldName::Source,
            NetField::Rail(_) => NetFieldName::Rail,
            NetField::Limit(_) => NetFieldName::Limit,
            NetField::Protocol(_) => NetFieldName::Protocol,
            NetField::Bitrate(_) => NetFieldName::Bitrate,
            NetField::Master(_) => NetFieldName::Master,
        }
    }

    /// フィールド名の綴り（ファイルのキー・CLI の旗の名前）。
    pub fn name(&self) -> &'static str {
        self.key().as_str()
    }

    /// ネットのそのフィールドの今の値（新しいファイル・丸ごとの書き込みと `fune net add` の旗が、1 フィールドずつと同じ形で書くため）。
    pub fn of(net: &Net, key: NetFieldName) -> Self {
        match key {
            NetFieldName::Kind => NetField::Kind(net.kind),
            NetFieldName::Label => NetField::Label(net.label.clone()),
            NetFieldName::Source => NetField::Source(net.source.clone()),
            NetFieldName::Rail => NetField::Rail(net.rail.clone()),
            NetFieldName::Limit => NetField::Limit(net.limit.clone()),
            NetFieldName::Protocol => NetField::Protocol(net.protocol.clone()),
            NetFieldName::Bitrate => NetField::Bitrate(net.bitrate.clone()),
            NetFieldName::Master => NetField::Master(net.master.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_nets_are_zero_volt_or_named_gnd() {
        let mut net = Net::new("P5V", NetKind::Power);
        assert!(!net.is_return());
        net.rail = Some(Quantity::new(0.0, "V"));
        assert!(net.is_return());
        for id in ["GND", "PWR_gnd", "P5V_RTN"] {
            assert!(Net::new(id, NetKind::Power).is_return(), "{id}");
        }
        // 語の一部だけの一致は数えない。電源でなければ戻りではない
        assert!(!Net::new("GNDX", NetKind::Power).is_return());
        assert!(!Net::new("GND", NetKind::Data).is_return());
    }
}
