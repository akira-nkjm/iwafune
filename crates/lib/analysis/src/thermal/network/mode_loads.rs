//! モードごとの単体消費電力と RF 出力を共通評価器で平均し、発熱入力と警告にする（電力 → 発熱は `ThermalLoad::dissipation`）。

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
    /// 熱を出す節点（既定は部品の属する節点。発熱の場所を指定すればその節点）。
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
    /// 1 台の状態ごとの RF 出力（単体値。書かれていない状態は 0）。
    #[serde(default)]
    pub rf_output: StateValues<Power>,
    /// 熱になる割合（書けば RF 出力より優先。`ThermalLoad::dissipation`）。
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

/// 部品 ID と発熱の源（平均電力が揃わない部品は None）の一覧。
type ModePartLoads = Vec<(String, Option<ThermalLoad>)>;

impl ThermalModeLoads {
    /// 単体平均電力・RF 出力と員数を渡す。不足・未割り当ては modes の警告を返して合計から除く。
    /// 熱節点の対象外の部品の割り当ては、この解析では未知部品としない（[`UnlistedParts::Ignore`]）。
    pub fn evaluate(&self) -> Result<(Vec<ThermalLoad>, Vec<Check>), AnalysisError> {
        let (loads, checks) = self.part_loads()?;
        Ok((loads.into_iter().filter_map(|(_, l)| l).collect(), checks))
    }

    /// 部品別の内部発熱（不足は None）と警告。モード表示も同じ評価を使う。
    pub fn evaluate_parts(&self) -> Result<(ModePartHeat, Vec<Check>), AnalysisError> {
        let (loads, checks) = self.part_loads()?;
        let parts = loads
            .into_iter()
            .map(|(id, l)| Ok((id, l.as_ref().map(ThermalLoad::dissipation).transpose()?)))
            .collect::<Result<_, AnalysisError>>()?;
        Ok((parts, checks))
    }

    /// 部品ごとの発熱の源（平均電力が揃わない部品は None）。RF 出力の平均は同じ評価器で別に求める
    /// （平均は状態の割合の線形和なので、発熱の平均 = 電力の平均 − RF 出力の平均）。
    fn part_loads(&self) -> Result<(ModePartLoads, Vec<Check>), AnalysisError> {
        let power = self.evaluation(|p, state| p.power.get(state).map(Qty::value))?;
        let rf =
            self.evaluation(|p, state| Some(p.rf_output.get(state).map_or(0.0, Qty::value)))?;
        let loads = self
            .parts
            .iter()
            .zip(power.means.iter().zip(&rf.means))
            .map(|(p, ((_, mean), (_, rf_mean)))| {
                let load = mean.map(|power| ThermalLoad {
                    node: p.node.clone(),
                    count: 1,
                    power: Qty::from_raw(power),
                    rf_output: rf_mean.filter(|v| *v > 0.0).map(Qty::from_raw),
                    heat_fraction: p.heat_fraction,
                });
                (p.id.clone(), load)
            })
            .collect();
        Ok((loads, power.checks("power")))
    }

    fn evaluation(
        &self,
        value: impl Fn(&ThermalModePart, &str) -> Option<f64>,
    ) -> Result<crate::modes::ModeEvaluation, AnalysisError> {
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
            |id, state| value(self.parts.iter().find(|p| p.id == id)?, state),
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
            rf_output: StateValues::default(),
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

    /// 手計算: 受信 0.6 W・送信 9.5 W（RF 2 W）の送信機を rx に置き tx 10 % → 電力の平均 0.9·0.6 + 0.1·9.5 = 1.49 W、
    /// RF の平均 0.1·2 = 0.2 W、発熱 1.29 W。heat_fraction 0.5 を書けば RF を見ず 0.745 W。
    #[test]
    fn rf_output_is_not_heat_by_hand() {
        let input = |fraction: Option<f64>| -> ThermalModeLoads {
            serde_json::from_value(serde_json::json!({
                "mode": {"id":"M", "label":"", "assignments":[{"part":"TX", "state":"rx", "duty":{"tx":0.1}}]},
                "parts": [{"id":"TX", "node":"n", "power":{"by_state":{"rx":0.6, "tx":9.5}},
                    "rf_output":{"by_state":{"tx":2.0}}, "heat_fraction": fraction}]
            }))
            .unwrap()
        };
        let heat = |fraction| {
            input(fraction).evaluate_parts().unwrap().0[0]
                .1
                .unwrap()
                .value()
        };
        assert!((heat(None) - 1.29).abs() < 1e-12);
        assert!((heat(Some(0.5)) - 0.745).abs() < 1e-12);
        let (loads, _) = input(None).evaluate().unwrap();
        assert!((loads[0].rf_output.unwrap().value() - 0.2).abs() < 1e-12);
    }
}
