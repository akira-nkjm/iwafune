//! 取付関係から節点間の熱結合を作る。

use super::{ThermalConductiveLink, ThermalRadiativeLink};
use sat_units::typed::{Area, Qty, ThermalConductance};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 取付の両端が属する節点と、取付 1 か所あたりの結合。
/// 同じ節点内の取付は無視し、節点対は対称としてまとめる。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalMounts {
    /// 部品の取付と取付先を節点へ対応させた組（1 取付につき 1 行）。
    pub mounts: Vec<[String; 2]>,
    /// 1 取付の伝導コンダクタンス。並列取付の数を掛ける。
    pub conductance: Qty<ThermalConductance>,
    /// 節点対ごとの有効放射交換面積。取付数では増やさない。
    #[serde(default)]
    pub exchange_area: Option<Qty<Area>>,
}

impl ThermalMounts {
    /// 並列伝導をまとめ、異なる節点対につき放射結合を 1 つ置く。
    pub fn links(&self) -> (Vec<ThermalConductiveLink>, Vec<ThermalRadiativeLink>) {
        let mut pairs: BTreeMap<(&str, &str), u32> = BTreeMap::new();
        for [a, b] in &self.mounts {
            if a != b {
                let key = if a < b {
                    (a.as_str(), b.as_str())
                } else {
                    (b.as_str(), a.as_str())
                };
                *pairs.entry(key).or_default() += 1;
            }
        }
        let conductive = pairs
            .iter()
            .map(|(&(a, b), &n)| ThermalConductiveLink {
                a: a.to_owned(),
                b: b.to_owned(),
                conductance: Qty::from_raw(self.conductance.value() * f64::from(n)),
            })
            .collect();
        let radiative = self.exchange_area.map_or_else(Vec::new, |area| {
            pairs
                .keys()
                .map(|&(a, b)| ThermalRadiativeLink {
                    a: a.to_owned(),
                    b: b.to_owned(),
                    exchange_area: area,
                })
                .collect()
        });
        (conductive, radiative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_mounts_by_hand() {
        let m = ThermalMounts {
            mounts: vec![
                ["a".into(), "b".into()],
                ["b".into(), "a".into()],
                ["a".into(), "a".into()],
            ],
            conductance: Qty::from_raw(0.5),
            exchange_area: Some(Qty::from_raw(0.02)),
        };
        let (c, r) = m.links();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].conductance.value(), 1.0); // 0.5 W/K × 2
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].exchange_area.value(), 0.02);
    }
}
