//! フェーズ（`phases/<ID>.toml`。docs/model.md §7.10）。ミッションの段階（打ち上げ直後・定常運用・廃棄など）。
//!
//! フェーズは並び（`order`）・始まりと終わりの条件（自由文）・期間・使うモードを持ち、軌道（`[orbit]`）と
//! モードごとの姿勢則（`[law.<モード>]`）をそのフェーズだけ上書きできる。解析ケースが `phase` でフェーズを指すと、
//! 射影はそのケースの間だけ設計の軌道・姿勢則の代わりにフェーズの値を使う（`fune_projection::Context::orbit_settings`・
//! `Context::attitude_law`）。serde の形がそのままファイルの形（`id` はファイル名から決まるので書かない）。

use super::analysis::AttitudeLawSetting;
use super::conditions::OrbitSettings;
use super::mode::Mode;
use crate::Design;
use crate::id::Id;
use crate::value::Quantity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// フェーズ（`phases/<ID>.toml`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Phase {
    #[serde(default)]
    pub id: Id<Phase>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// 並び順（整数。一覧は `(order, ID)` の順。省略は 0）
    #[serde(default)]
    pub order: i64,
    /// フェーズが始まる条件（自由文）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    /// フェーズが終わる条件（自由文）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    /// フェーズの期間（時間の次元）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<Quantity>,
    /// このフェーズで使うモード（`modes/` への参照）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modes: Vec<Id<Mode>>,
    /// 軌道の上書き（`conditions/orbit.toml` と同じ形。あれば設計の軌道を丸ごと置き換える。キーごとの合成はしない）。
    /// `source = "transfer"` と `maneuver` で、日付つきの遷移のマヌーバから巡航の軌道を取れる（フェーズの専用の欄は無い）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orbit: Option<OrbitSettings>,
    /// モード ID → 姿勢則（`analysis/aocs.toml` の `[law.<モード>]` と同じ形。そのモードの姿勢則をこのフェーズだけ置き換える）
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub law: BTreeMap<String, AttitudeLawSetting>,
}

impl Phase {
    /// 軌道か姿勢則を上書きしているか（一覧の表示用）。
    pub fn overrides(&self) -> bool {
        self.orbit.is_some() || !self.law.is_empty()
    }
}

impl Design {
    /// フェーズを並びの順（`(order, ID)`）に並べる。**フェーズの並びはここだけで決める。**
    pub fn ordered_phases(&self) -> Vec<&Phase> {
        let mut out: Vec<&Phase> = self.phases.values().collect();
        out.sort_by(|a, b| (a.order, &a.id).cmp(&(b.order, &b.id)));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phase(id: &str, order: i64) -> Phase {
        Phase {
            id: Id::new(id),
            order,
            ..Phase::default()
        }
    }

    #[test]
    fn phases_are_ordered_by_order_then_id() {
        let mut d = Design::default();
        for (id, order) in [("B", 2), ("A", 2), ("Z", 1), ("Y", -1)] {
            d.phases.insert(Id::new(id), phase(id, order));
        }
        let ids: Vec<_> = d
            .ordered_phases()
            .iter()
            .map(|p| p.id.to_string())
            .collect();
        assert_eq!(ids, ["Y", "Z", "A", "B"]);
    }

    #[test]
    fn empty_tables_are_not_written_and_order_defaults_to_zero() {
        let p: Phase = toml::from_str("label = \"x\"").unwrap();
        assert_eq!(p.order, 0);
        let json = serde_json::to_value(&p).unwrap();
        assert!(
            json.get("orbit").is_none() && json.get("law").is_none() && json.get("modes").is_none()
        );
        assert!(!p.overrides());
    }
}
