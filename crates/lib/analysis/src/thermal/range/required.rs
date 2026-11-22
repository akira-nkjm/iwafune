//! 要求が節点の温度範囲に重ねる限界。**要求は部品の範囲を厳しくするだけ**で、広げない
//! （部品の温度範囲は部品の性質で、要求はそれより厳しい運用の条件を足すだけ）。
//! 部品の範囲より緩い要求は効かないので、判定文にそう書く（`checks`）。

use super::{ThermalRangeSide, ThermalTempRange};
use sat_units::typed::{Qty, Temperature};
use serde::{Deserialize, Serialize};

/// 要求が与える限界 1 つ（下限か上限）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalRequiredBound {
    /// 限界の温度
    pub limit: Qty<Temperature>,
    /// 限界を決めた要求の ID
    pub requirement: String,
}

/// 1 種類の範囲に、要求が重ねる下限と上限（無い側は `None`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalRequiredRange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<ThermalRequiredBound>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<ThermalRequiredBound>,
}

/// 要求の限界が判定にどう効いたか（部品の範囲と違うときだけ。同じ値なら残さない）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalRequiredUse {
    /// 下限か（`false` なら上限）
    pub low: bool,
    pub requirement: String,
    pub limit: Qty<Temperature>,
    /// 部品の範囲より厳しくて、判定の限界になったか（`false` は部品の範囲より緩く、効かなかった）
    pub tightens: bool,
}

impl ThermalRequiredUse {
    /// 側 `side` の使われ方か。
    pub(super) fn is_side(&self, side: ThermalRangeSide) -> bool {
        self.low == (side == ThermalRangeSide::Low)
    }
}

impl ThermalRequiredRange {
    /// 部品の範囲 `range` に要求の限界を重ねる（厳しい方を使う）。重ねた範囲と、部品の範囲と値が違う限界の使われ方。
    pub fn tighten(&self, range: ThermalTempRange) -> (ThermalTempRange, Vec<ThermalRequiredUse>) {
        let mut out = range;
        let mut uses = Vec::new();
        if let Some(b) = &self.min {
            let (part, req) = (range.min.value(), b.limit.value());
            if req > part {
                out.min = b.limit;
            }
            if req != part {
                uses.push(use_of(true, b, req > part));
            }
        }
        if let Some(b) = &self.max {
            let (part, req) = (range.max.value(), b.limit.value());
            if req < part {
                out.max = b.limit;
            }
            if req != part {
                uses.push(use_of(false, b, req < part));
            }
        }
        (out, uses)
    }
}

fn use_of(low: bool, b: &ThermalRequiredBound, tightens: bool) -> ThermalRequiredUse {
    ThermalRequiredUse {
        low,
        requirement: b.requirement.clone(),
        limit: b.limit,
        tightens,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(min: f64, max: f64) -> ThermalTempRange {
        ThermalTempRange {
            min: Qty::from_raw(min),
            max: Qty::from_raw(max),
        }
    }

    fn bound(limit: f64) -> Option<ThermalRequiredBound> {
        Some(ThermalRequiredBound {
            limit: Qty::from_raw(limit),
            requirement: "R".into(),
        })
    }

    /// 手計算: 部品 270〜320 K。下限 280 K は厳しいので 280 K になり、上限 330 K は緩いので 320 K のまま効かない。
    /// 下限が部品と同じ 270 K なら変えず、記録も残さない。
    #[test]
    fn a_requirement_only_tightens_the_part_range() {
        let req = ThermalRequiredRange {
            min: bound(280.0),
            max: bound(330.0),
        };
        let (r, uses) = req.tighten(range(270.0, 320.0));
        assert_eq!((r.min.value(), r.max.value()), (280.0, 320.0));
        assert_eq!(uses.len(), 2);
        assert!(uses[0].low && uses[0].tightens);
        assert!(!uses[1].low && !uses[1].tightens);
        let same = ThermalRequiredRange {
            min: bound(270.0),
            max: None,
        };
        let (r, uses) = same.tighten(range(270.0, 320.0));
        assert_eq!((r.min.value(), r.max.value()), (270.0, 320.0));
        assert!(uses.is_empty());
    }
}
