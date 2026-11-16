//! 内部発熱の源（部品など）と、消費電力から発熱への変換。**電力 → 発熱の変換はここだけ。**
//!
//! 発熱の決め方（優先の順）:
//! 1. `heat_fraction` があれば 発熱 = 消費電力 × `heat_fraction`（RF 出力は見ない。従来の上書き）
//! 2. 無ければ 発熱 = 消費電力 − RF 出力（`rf_output`。省略時 0 なので、すべて熱になる）

use crate::spec::AnalysisError;
use sat_units::typed::{Power, Qty};
use serde::{Deserialize, Serialize};

/// 節点への内部発熱の源（部品など）。発熱は [`ThermalLoad::dissipation`] で求める。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalLoad {
    /// 員数。power と rf_output は単体値（省略時 1）。
    #[serde(default = "crate::spec::default_count")]
    pub count: u32,
    /// 熱を出す節点
    pub node: String,
    /// 1 台あたりの消費電力（モードの平均も単体値）
    pub power: Qty<Power>,
    /// 1 台あたりの RF 出力（アンテナから出ていき熱にならない電力。0〜消費電力、省略時 0）
    #[serde(default)]
    pub rf_output: Option<Qty<Power>>,
    /// 消費電力のうち熱になる割合（0〜1）。書けば RF 出力より優先する
    #[serde(default)]
    pub heat_fraction: Option<f64>,
}

impl ThermalLoad {
    /// 員数を掛けた内部発熱（決め方はモジュールの説明）。検査と計算はここだけ。
    pub fn dissipation(&self) -> Result<Qty<Power>, AnalysisError> {
        let p = self.power.value();
        let per_unit = match self.heat_fraction {
            Some(f) if p.is_finite() && f.is_finite() && (0.0..=1.0).contains(&f) => p * f,
            Some(_) => {
                return Err(self.invalid("power must be finite and heat_fraction within 0 to 1"));
            }
            None => {
                let rf = self.rf_output.map_or(0.0, Qty::value);
                if !(p.is_finite() && rf.is_finite() && (0.0..=p).contains(&rf)) {
                    return Err(
                        self.invalid("power must be finite and RF output within 0 to power")
                    );
                }
                p - rf
            }
        };
        Ok(Qty::from_raw(per_unit * f64::from(self.count)))
    }

    fn invalid(&self, rule: &str) -> AnalysisError {
        AnalysisError::InvalidInput(format!("Node {} heat dissipation: {rule}", self.node))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(power: f64, rf: Option<f64>, fraction: Option<f64>) -> ThermalLoad {
        ThermalLoad {
            count: 2,
            node: "n".into(),
            power: Qty::from_raw(power),
            rf_output: rf.map(Qty::from_raw),
            heat_fraction: fraction,
        }
    }

    /// 手計算: 9.5 W の送信機で RF 2 W → 7.5 W × 2 台 = 15 W。heat_fraction 0.5 を書けば RF を見ず 9.5 × 0.5 × 2 = 9.5 W。
    /// どちらも無ければ 19 W。RF が消費電力を超える・負の割合は入力の誤り。
    #[test]
    fn dissipation_precedence_by_hand() {
        let q = |l: ThermalLoad| l.dissipation().map(Qty::value);
        assert_eq!(q(load(9.5, Some(2.0), None)).unwrap(), 15.0);
        assert_eq!(q(load(9.5, Some(2.0), Some(0.5))).unwrap(), 9.5);
        assert_eq!(q(load(9.5, None, None)).unwrap(), 19.0);
        assert!(q(load(1.0, Some(2.0), None)).is_err());
        assert!(q(load(1.0, None, Some(-0.1))).is_err());
    }
}
