//! マヌーバ（`maneuvers/<ID>.toml`）。推進系の解析が共有する入力（モードの表と同じ位置づけ）。
//!
//! 実行の順は `after`（前のマヌーバ。フェーズは #9 で足す）でつなぎ、[`Design::ordered_maneuvers`] だけが決める。
//! 目標の軌道（`[target]`）は変えたい要素だけを書く。`[constraints]` は細かい段階（数値伝搬）でだけ効き、
//! 粗い段階の解析は無視したことを結果に書く。

use super::conditions::CentralBodyKind;
use super::part::Part;
use super::system::Mode;
use crate::Design;
use crate::enum_text::enum_text;
use crate::id::Id;
use crate::value::Quantity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use ts_rs::TS;

enum_text! {
    /// ΔV の区分（ΔV バジェットの行）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
    pub enum ManeuverCategory: "maneuver category" {
        Insertion = "insertion",
        StationKeeping = "station_keeping",
        Unloading = "unloading",
        Deorbit = "deorbit",
        Other = "other",
    }
}

enum_text! {
    /// マヌーバの計算の段階（`propagated` は推力を入れた数値伝搬で、`[constraints]` が効く）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
    pub enum ManeuverMethod: "maneuver calculation method" {
        Impulsive = "impulsive",
        FiniteBurn = "finite_burn",
        LowThrust = "low_thrust",
        Propagated = "propagated",
    }
}

enum_text! {
    /// 推力の向きの決まり（`propagated` だけで効く）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
    pub enum ManeuverSteering: "thrust steering" {
        Tangential = "tangential",
        Inertial = "inertial",
    }
}

/// 目標の軌道（変えたい要素だけ）。`altitude` は円軌道（近地点と遠地点の両方）で、`perigee_altitude`・`apogee_altitude` と一緒には書かない。
/// `body` を書くとその天体を回る軌道への遷移（パッチドコニック）で、高度はその天体の半径から数える。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(rename = "ManeuverTargetOrbit")]
pub struct ManeuverTarget {
    /// 目標の中心天体（前の軌道と違えば天体間の遷移）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<CentralBodyKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub altitude: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub perigee_altitude: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apogee_altitude: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inclination: Option<Quantity>,
}

impl ManeuverTarget {
    /// 何も書かれていないか（ファイルに `[target]` を書かない）。
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    /// 書かれた要素のキーと値（宣言の順）。
    pub fn entries(&self) -> [(&'static str, Option<&Quantity>); 4] {
        [
            ("altitude", self.altitude.as_ref()),
            ("perigee_altitude", self.perigee_altitude.as_ref()),
            ("apogee_altitude", self.apogee_altitude.as_ref()),
            ("inclination", self.inclination.as_ref()),
        ]
    }

    /// 要素のキー → 次元名。
    pub fn dimension(key: &str) -> &'static str {
        if key == "inclination" {
            "Angle"
        } else {
            "Length"
        }
    }
}

/// 細かい段階（数値伝搬）でだけ効く制約。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ManeuverConstraints {
    /// 日照中だけ噴射する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sunlit_only: Option<bool>,
    /// 噴射するモード
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<Id<Mode>>,
    /// デューティ（噴いてよい時間のうち噴く割合、0 より大きく 1 以下。省略で 1）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duty: Option<f64>,
    /// 推力の向き（省略で接線方向）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steering: Option<ManeuverSteering>,
}

impl ManeuverConstraints {
    /// 何も書かれていないか（ファイルに `[constraints]` を書かない）。
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    /// 書かれている制約のキー（宣言の順）。
    pub fn keys(&self) -> Vec<&'static str> {
        [
            ("sunlit_only", self.sunlit_only.is_some()),
            ("mode", self.mode.is_some()),
            ("duty", self.duty.is_some()),
            ("steering", self.steering.is_some()),
        ]
        .into_iter()
        .filter_map(|(k, set)| set.then_some(k))
        .collect()
    }
}

/// マヌーバ（`maneuvers/<ID>.toml`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Maneuver {
    #[serde(default)]
    pub id: Id<Maneuver>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub category: ManeuverCategory,
    pub method: ManeuverMethod,
    /// 使うスラスタ（部品 ID。推力・比推力・消費電力は部品から）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thruster: Option<Id<Part>>,
    /// 前のマヌーバ（無ければ最初。フェーズの ID は #9 で足す）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    /// バイエリプティック遷移の中間の遠地点の高度（impulsive・finite_burn だけ）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via_altitude: Option<Quantity>,
    #[serde(default, skip_serializing_if = "ManeuverTarget::is_empty")]
    pub target: ManeuverTarget,
    #[serde(default, skip_serializing_if = "ManeuverConstraints::is_empty")]
    pub constraints: ManeuverConstraints,
}

impl Design {
    /// マヌーバを実行の順に並べる: `after` の無いもの（と、`after` がマヌーバでないもの）を ID 順に始点にし、
    /// それぞれの後ろに `after` でつながるものを深さ優先（同じ前を持つものは ID 順）で続ける。
    /// 循環していてたどれないものは最後に ID 順で並べる（`check` が誤りにする）。**実行の順はここだけで決める。**
    pub fn ordered_maneuvers(&self) -> Vec<&Maneuver> {
        let is_root = |m: &Maneuver| {
            m.after
                .as_deref()
                .is_none_or(|a| !self.maneuvers.contains_key(a))
        };
        let mut seen = BTreeSet::new();
        let mut out = Vec::with_capacity(self.maneuvers.len());
        let mut stack: Vec<&Maneuver> = self
            .maneuvers
            .values()
            .filter(|m| is_root(m))
            .rev()
            .collect();
        while let Some(m) = stack.pop() {
            if !seen.insert(m.id.as_str()) {
                continue;
            }
            out.push(m);
            let next = self
                .maneuvers
                .values()
                .filter(|n| n.after.as_deref() == Some(m.id.as_str()))
                .rev();
            stack.extend(next);
        }
        let rest = self
            .maneuvers
            .values()
            .filter(|m| !seen.contains(m.id.as_str()));
        out.extend(rest);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn maneuver(id: &str, after: Option<&str>) -> Maneuver {
        Maneuver {
            id: Id::new(id),
            label: None,
            category: ManeuverCategory::Other,
            method: ManeuverMethod::Impulsive,
            thruster: None,
            after: after.map(str::to_owned),
            via_altitude: None,
            target: ManeuverTarget::default(),
            constraints: ManeuverConstraints::default(),
        }
    }

    fn order(list: &[(&str, Option<&str>)]) -> Vec<String> {
        let mut d = Design::default();
        for (id, after) in list {
            d.maneuvers.insert(Id::new(*id), maneuver(id, *after));
        }
        d.ordered_maneuvers()
            .iter()
            .map(|m| m.id.to_string())
            .collect()
    }

    #[test]
    fn maneuvers_follow_their_predecessor() {
        // ID 順なら DEORBIT が先だが、DEORBIT は RAISE のあと
        assert_eq!(
            order(&[("DEORBIT", Some("RAISE")), ("RAISE", None)]),
            ["RAISE", "DEORBIT"]
        );
        // 鎖は途切れずに続け、別の始点はそのあと。マヌーバでない `after`（フェーズ）は始点
        assert_eq!(
            order(&[
                ("A", None),
                ("B", Some("LEOP")),
                ("C", Some("A")),
                ("D", Some("C"))
            ]),
            ["A", "C", "D", "B"]
        );
        // 循環はたどれないので最後に ID 順
        assert_eq!(
            order(&[("X", Some("Y")), ("Y", Some("X")), ("Z", None)]),
            ["Z", "X", "Y"]
        );
    }

    #[test]
    fn empty_tables_are_not_written() {
        let m = maneuver("M", None);
        let json = serde_json::to_value(&m).unwrap();
        assert!(json.get("target").is_none() && json.get("constraints").is_none());
        let mut c = ManeuverConstraints::default();
        assert!(c.keys().is_empty());
        c.sunlit_only = Some(true);
        assert_eq!(c.keys(), ["sunlit_only"]);
    }
}
