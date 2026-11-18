//! ネットの分類の型（系統図の電源の系統と星形の中心。決め方は `query::net_class`）。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 電源のネットの系統（表示だけの分類。設計データには書かない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum PowerClass {
    /// 戻り（GND）
    Return,
    /// 供給元が電池
    Battery,
    /// 太陽電池パネルがつながる
    Solar,
    /// レール電圧を持つ安定化バス（電圧ごとの分けは `NetView.rail`）
    Regulated,
    /// レール電圧の無い非安定化バス
    Unregulated,
}

/// ネットの星形の中心のポート。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct NetHub {
    pub port: fune_model::path::PortRef,
    /// 線を各端から中心へ向けるか。供給元の書かれていない電源のネットで、役割（PCDU）から中心を選んだときだけ true。
    /// ほかは中心から各端へ向ける
    pub collects: bool,
}
