//! モードごとの単体消費電力を共通評価器で平均し、発熱入力と警告にする。

use super::ThermalLoad;
use crate::modes::{ModeDef, StateValues, UnlistedParts, evaluate};
use crate::spec::{AnalysisError, Check};
use sat_units::typed::{Power, Qty};
use serde::{Deserialize, Serialize};

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
    /// 冗長の方式。
    #[serde(default)]
    pub redundancy: crate::modes::Redundancy,
    /// 種別の状態の語彙（warm の予備状態を選ぶ）。
    #[serde(default)]
    pub states: Vec<String>,
    /// 1 台の状態ごとの消費電力（単体値。`by_state` に無い状態は `default`）。
    #[serde(default)]
    pub power: StateValues<Power>,
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

/// 部品 ID と内部発熱（不足は None）の一覧。
pub type ModePartHeat = Vec<(String, Option<Qty<Power>>)>;

impl ThermalModeLoads {
    /// 単体平均電力と員数を渡す。不足・未割り当ては modes の警告を返して合計から除く。
    /// 熱節点の対象外の部品の割り当ては、この解析では未知部品としない（[`UnlistedParts::Ignore`]）。
    pub fn evaluate(&self) -> Result<(Vec<ThermalLoad>, Vec<Check>), AnalysisError> {
        let ev = self.evaluation()?;
        let loads = self
            .parts
            .iter()
            .zip(&ev.means)
            .filter_map(|(p, (_, mean))| {
                Some(ThermalLoad {
                    node: p.node.clone(),
                    count: 1,
                    power: Qty::from_raw((*mean)?),
                    heat_fraction: p.heat_fraction,
                })
            })
            .collect();
        Ok((loads, ev.checks("消費電力")))
    }
    /// 部品別の内部発熱（不足は None）と警告。モード表示も同じ評価を使う。
    pub fn evaluate_parts(&self) -> Result<(ModePartHeat, Vec<Check>), AnalysisError> {
        let ev = self.evaluation()?;
        let parts = self
            .parts
            .iter()
            .zip(&ev.means)
            .map(|(p, (_, mean))| {
                let heat = mean
                    .map(|power| {
                        ThermalLoad {
                            node: p.node.clone(),
                            count: 1,
                            power: Qty::from_raw(power),
                            heat_fraction: p.heat_fraction,
                        }
                        .dissipation()
                    })
                    .transpose()?;
                Ok((p.id.clone(), heat))
            })
            .collect::<Result<_, AnalysisError>>()?;
        Ok((parts, ev.checks("消費電力")))
    }

    fn evaluation(&self) -> Result<crate::modes::ModeEvaluation, AnalysisError> {
        evaluate(
            &self.mode,
            self.parts.iter().map(|p| p.id.as_str()),
            UnlistedParts::Ignore,
            |id| {
                self.parts
                    .iter()
                    .find(|p| p.id == id)
                    .map(ThermalModePart::population)
                    .unwrap_or(crate::modes::Population {
                        count: 0,
                        redundancy: crate::modes::Redundancy::None,
                        states: &[],
                    })
            },
            |id, state| {
                let p = self.parts.iter().find(|p| p.id == id)?;
                p.power.get(state).map(Qty::value)
            },
        )
    }
}

impl ThermalModePart {
    /// 共通のモード評価に渡す員数と語彙。
    pub fn population(&self) -> crate::modes::Population<'_> {
        crate::modes::Population {
            count: self.count,
            redundancy: self.redundancy,
            states: &self.states,
        }
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
            power: StateValues {
                default: None,
                by_state: [
                    ("off".into(), Qty::from_raw(0.0)),
                    ("on".into(), Qty::from_raw(8.0)),
                ]
                .into(),
            },
            heat_fraction: Some(0.5),
            redundancy: crate::modes::Redundancy::None,
            states: Vec::new(),
        };
        let input = ThermalModeLoads {
            mode: ModeDef {
                id: "m".into(),
                label: "m".into(),
                assignments: vec![StateAssignment {
                    part: "a".into(),
                    state: "off".into(),
                    duty: [("on".into(), 0.25)].into(),
                    active: None,
                    spare_state: None,
                }],
                eclipse: Vec::new(),
            },
            parts: vec![part("a"), part("missing")],
        };
        let (loads, checks) = input.evaluate().unwrap();
        assert_eq!(loads.len(), 1);
        assert_eq!(loads[0].power.value(), 6.0); // 3 台 × 8 W × 25 %
        assert_eq!(loads[0].count, 1);
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].key, "unassigned");
    }
    #[test]
    fn warm_spare_heat_by_hand() {
        let input: ThermalModeLoads = serde_json::from_value(serde_json::json!({
            "mode": {"id":"TEST", "label":"", "assignments":[{"part":"RW", "state":"on", "active":3}]},
            "parts": [{"id":"RW", "node":"n", "count":4, "redundancy":"warm", "states":["on","standby"],
                "power":{"by_state":{"on":10, "standby":2}}, "heat_fraction":0.5}]
        })).unwrap();
        let (loads, checks) = input.evaluate().unwrap();
        assert!(checks.is_empty());
        assert_eq!(loads[0].power.value(), 32.0); // 3 × 10 + 1 × 2 W、熱への変換前
        assert_eq!(loads[0].count, 1); // 台数を二重に掛けない
        assert_eq!(loads[0].heat_fraction, Some(0.5));
    }
}
