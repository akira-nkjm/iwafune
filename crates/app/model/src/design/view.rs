//! 見た目だけの情報（`view/<名前>.toml`。系統図の配置など）。解析は読まない。

use crate::enum_text::enum_text;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// 1 つの図の配置（`view/<名前>.toml`）。ノードの ID は部品・グループ・ネットの ID。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ViewLayout {
    /// ファイル名から決まる（例: `"electrical"`）
    #[serde(default)]
    pub id: String,
    /// ノードの ID → 位置と大きさ（画面の座標。整数）
    #[serde(default)]
    pub nodes: BTreeMap<String, ViewNode>,
    /// 安定したネット・端点キー → 絶対座標の直交経路。
    #[serde(default)]
    pub edge: BTreeMap<String, ViewEdge>,
}

/// ノード 1 つの位置と大きさ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ViewNode {
    pub x: i32,
    pub y: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<i32>,
}

/// 線を手で引いた経路（`[edge]` の 1 行）。端の側は表示だけで、設計上のポートのつながりは変えない。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ViewEdge {
    /// 折れ線の点（画面の絶対座標の整数 `[x, y]`）。隣り合う点は水平か垂直、2 点以上
    pub points: Vec<[i32; 2]>,
    /// 始点の端子をカードのどの側に出すか（省略なら自動配置のまま）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_side: Option<ViewPortSide>,
    /// 終点の端子をカードのどの側に出すか
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_side: Option<ViewPortSide>,
    /// 始点の端子の、その側の上での位置（側の始まり＝左か上の角からの距離 [px]）。省略なら端子の並びで決まる位置
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_offset: Option<i32>,
    /// 終点の端子の、その側の上での位置
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_offset: Option<i32>,
}

impl ViewEdge {
    /// 経路が 2 点以上で、隣り合う点が水平か垂直に並んでいるか（**経路の規則はここだけ**。検査 `view.route` が使う）。
    pub fn is_orthogonal(&self) -> bool {
        self.points.len() >= 2
            && self
                .points
                .windows(2)
                .all(|p| p[0][0] == p[1][0] || p[0][1] == p[1][1])
    }

    /// 端子の位置（側の左か上の角からの距離）が 0 以上か（検査 `view.port_offset` が使う）。
    pub fn offsets_are_valid(&self) -> bool {
        [self.source_offset, self.target_offset]
            .into_iter()
            .flatten()
            .all(|o| o >= 0)
    }
}

enum_text! {
    /// カードの端子を出す側。設計ファイルと CLI での書き方（`left` など）は [`ViewPortSide::as_str`] だけで決める。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
    pub enum ViewPortSide: "terminal side" {
        Left = "left",
        Right = "right",
        Top = "top",
        Bottom = "bottom",
    }
}
