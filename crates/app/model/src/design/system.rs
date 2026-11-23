//! 複数の部品にまたがるエンティティ: ネット・ハーネス・解析ケース・出典（docs/model.md §7。モードは `mode`、フェーズは `phase`）。
//!
//! serde の形がそのままファイルの形（`id` はファイル名から決まるので書かない）。

use super::mode::Mode;
use super::phase::Phase;
use crate::enum_text::enum_text;
use crate::id::Id;
use crate::path::{PortRef, port_text};
use crate::value::Quantity;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use super::defs::NetKind;
pub use super::net::{Net, NetField, NetFieldName};
pub use super::source::{Distribution, Source, SourceKind};

/// ケーブル区間（物理。`harness/<ID>.toml`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Harness {
    #[serde(default)]
    pub id: Id<Harness>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// 端のポート（`"PCDU.OUT3"`。読み込むときに `PortRef` にする）
    #[serde(with = "port_text")]
    #[ts(as = "String")]
    pub from: PortRef,
    #[serde(with = "port_text")]
    #[ts(as = "String")]
    pub to: PortRef,
    #[serde(default)]
    pub nets: Vec<Id<Net>>,
    /// 線材（カタログ品）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wire: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cores: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<Quantity>,
}

enum_text! {
    /// 不確かさのどちら側を読むか。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
    pub enum UncertaintySide: "uncertainty side" {
        Nominal = "nominal",
        Min = "min",
        Max = "max",
    }
}

/// 解析ケース（`cases/<ID>.toml`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(rename = "AnalysisCase")]
pub struct Case {
    #[serde(default)]
    pub id: Id<Case>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 解析するモード（無ければ `analysis/power.toml` のモードの割合）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<Id<Mode>>,
    /// フェーズ（`phases/<ID>.toml`）。あれば、フェーズの軌道・姿勢則が設計のものの代わりに使われる
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<Id<Phase>>,
    /// 不確かさのどちら側を読むか（既定は nominal）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<UncertaintySide>,
    /// 熱の環境条件（`analysis/thermal.toml` の `[case.<名前>]`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thermal_case: Option<String>,
    /// β 角の上書き
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beta: Option<Quantity>,
    /// 時系列の解析で回す周回数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orbits: Option<u32>,
    /// 太陽からの距離（太陽を中心天体にする軌道だけ。軌道をこの半径の円軌道に置き換えて解析する。
    /// 惑星間の巡航の近日点・遠日点のケース。`fune case from-sun-distance`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sun_distance: Option<Quantity>,
}
