//! 形態（`configurations/<ID>.toml`。docs/model.md §7.11）。部品の姿勢を上書きする名前つきの層（収納・展開など）。
//!
//! 部品のファイルに書いた配置（名前の無い「基準」）への、まばらな上書き。`[pose.<部品>]` はその部品の `[pose]` を
//! まるごと置き換える（キーごとの合成はしない。取付先と取付点は基準のまま）。書いていない部品は基準のままで、
//! 子の部品は取付の連鎖でついて来る。どの姿勢が有効かを決めるのは [`Design::pose_in`] だけ。
//! serde の形がそのままファイルの形（`id` はファイル名から決まるので書かない）。

use super::part::{Part, Pose};
use crate::Design;
use crate::id::Id;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// 形態（`configurations/<ID>.toml`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    #[serde(default)]
    pub id: Id<Configuration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 部品 ID → 姿勢（その部品の `[pose]` をこの形態だけ置き換える）
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pose: BTreeMap<Id<Part>, Pose>,
}

impl Design {
    /// 形態 `configuration`（`None` は基準）での部品の姿勢。形態がその部品の姿勢を書いていれば（またはその形態が無ければ基準の）
    /// 書かれた姿勢。**どの姿勢が有効かを決めるのはここだけ**（解決の取付の合成だけが呼ぶ。xtask の `single_home` が確かめる）。
    pub fn pose_in<'a>(&'a self, part: &'a Part, configuration: Option<&str>) -> Option<&'a Pose> {
        configuration
            .and_then(|c| self.configurations.get(c))
            .and_then(|c| c.pose.get(part.id.as_str()))
            .or(part.pose.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Vector3;

    fn pose_at(x: f64) -> Pose {
        Pose {
            position: Some(Vector3 {
                x,
                y: 0.0,
                z: 0.0,
                unit: None,
                frame: None,
            }),
            orientation: None,
        }
    }

    #[test]
    fn pose_in_prefers_the_configuration_and_falls_back_to_the_base() {
        let mut d = Design::default();
        let mut part = Part::new("P");
        part.pose = Some(pose_at(1.0));
        d.parts.insert(Id::new("P"), part.clone());
        let mut c = Configuration {
            id: Id::new("STOWED"),
            ..Configuration::default()
        };
        c.pose.insert(Id::new("P"), pose_at(2.0));
        d.configurations.insert(Id::new("STOWED"), c);
        assert_eq!(d.pose_in(&part, None), Some(&pose_at(1.0)));
        assert_eq!(d.pose_in(&part, Some("STOWED")), Some(&pose_at(2.0)));
        // 無い形態と、その部品を書いていない形態は基準
        assert_eq!(d.pose_in(&part, Some("NOPE")), Some(&pose_at(1.0)));
        let other = Part::new("Q");
        assert_eq!(d.pose_in(&other, Some("STOWED")), None);
    }

    #[test]
    fn empty_pose_table_is_not_written_and_unknown_keys_are_rejected() {
        let c: Configuration = toml::from_str("label = \"x\"").unwrap();
        let json = serde_json::to_value(&c).unwrap();
        assert!(json.get("pose").is_none());
        assert!(toml::from_str::<Configuration>("colour = 1").is_err());
    }
}
