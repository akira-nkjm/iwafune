//! モードごとの単体消費電力を共通評価器で平均し、発熱入力と警告にする。

use super::ThermalLoad;
use crate::modes::{ModeDef, evaluate};
use crate::spec::{AnalysisError, Check};
use sat_units::typed::{Power, Qty};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// モード発熱の部品（単体値と員数）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalModePart {
    /// モード割り当ての部品 ID。
    pub id: String,
    /// 所属節点。
    pub node: String,
    /// 員数（省略で 1）。
    #[serde(default = "crate::spec::default_count")]
    pub count: u32,
    /// 状態によらない単体電力（状態別に無い場合の値）。
    #[serde(default)]
    pub power: Option<Qty<Power>>,
    /// 状態ごとの単体電力。
    #[serde(default)]
    pub by_state: BTreeMap<String, Qty<Power>>,
    /// 熱になる割合（省略で 1）。
    #[serde(default)]
    pub heat_fraction: Option<f64>,
}

/// 熱ケースで評価するモードと部品。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalModeLoads {
    /// 共通のモード定義。
    pub mode: ModeDef,
    /// 節点に含まれる部品。
    pub parts: Vec<ThermalModePart>,
}

impl ThermalModeLoads {
    /// 単体平均電力と員数を渡す。不足・未割り当ては modes の警告を返して合計から除く。
    pub fn evaluate(&self) -> Result<(Vec<ThermalLoad>, Vec<Check>), AnalysisError> {
        let ev = evaluate(
            &self.mode,
            self.parts.iter().map(|p| p.id.as_str()),
            |id, state| {
                let p = self.parts.iter().find(|p| p.id == id)?;
                p.by_state.get(state).copied().or(p.power).map(Qty::value)
            },
        )?;
        let loads = self
            .parts
            .iter()
            .zip(&ev.means)
            .filter_map(|(p, (_, mean))| {
                Some(ThermalLoad {
                    node: p.node.clone(),
                    count: p.count,
                    power: Qty::from_raw((*mean)?),
                    heat_fraction: p.heat_fraction,
                })
            })
            .collect();
        // 熱節点の対象外の部品の割り当ては、この解析では未知部品としない。
        let checks = ev
            .checks("消費電力")
            .into_iter()
            .filter(|c| c.key != "unknown_part")
            .collect();
        Ok((loads, checks))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::StateAssignment;

    #[test]
    fn mode_mean_and_warning_by_hand() {
        let part = |id: &str| ThermalModePart {
            id: id.into(),
            node: "n".into(),
            count: 3,
            power: None,
            by_state: [
                ("off".into(), Qty::from_raw(0.0)),
                ("on".into(), Qty::from_raw(8.0)),
            ]
            .into(),
            heat_fraction: Some(0.5),
        };
        let input = ThermalModeLoads {
            mode: ModeDef {
                id: "m".into(),
                label: "m".into(),
                assignments: vec![StateAssignment {
                    part: "a".into(),
                    state: "off".into(),
                    duty: [("on".into(), 0.25)].into(),
                }],
            },
            parts: vec![part("a"), part("missing")],
        };
        let (loads, checks) = input.evaluate().unwrap();
        assert_eq!(loads.len(), 1);
        assert_eq!(loads[0].power.value(), 2.0); // 8 W × 25 %
        assert_eq!(loads[0].count, 3);
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].key, "unassigned");
    }
}
