//! 見た目だけの情報（`view/<名前>.toml`。系統図の配置など）。解析は読まない。

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

/// カードの端子を出す側。設計ファイルと CLI での書き方（`left` など）は [`ViewPortSide::as_str`] だけで決める。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ViewPortSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl ViewPortSide {
    /// すべての側（CLI の選択肢の順）。
    pub const ALL: [Self; 4] = [Self::Left, Self::Right, Self::Top, Self::Bottom];

    /// 設計ファイルと CLI での書き方。serde の名前（snake_case）と同じ（テストで確かめる）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
        }
    }
}

impl std::fmt::Display for ViewPortSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ViewPortSide {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|side| side.as_str() == s)
            .ok_or_else(|| format!("`{s}` は端子の側ではありません（left / right / top / bottom）"))
    }
}

#[cfg(test)]
mod tests {
    use super::ViewPortSide;

    #[test]
    fn side_names_match_serde_and_parse_back() {
        for side in ViewPortSide::ALL {
            let json = serde_json::to_string(&side).unwrap();
            assert_eq!(json, format!("\"{}\"", side.as_str()));
            assert_eq!(side.as_str().parse::<ViewPortSide>(), Ok(side));
        }
        assert!("middle".parse::<ViewPortSide>().is_err());
    }
}
