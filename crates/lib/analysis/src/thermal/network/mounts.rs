//! 取付関係から節点間の熱結合を作る。

use super::ThermalRadiativeLink;
use super::mount_contact::{
    ThermalContact, ThermalContactFallback, ThermalContactPair, ThermalContactUsed, contact_area,
};
use crate::spec::AnalysisError;
use crate::structure::interference::DEFAULT_TOLERANCE;
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
    /// 1 取付の伝導コンダクタンス。並列取付の数を掛ける。接触面積から出せない取付はこの値を使う。
    pub conductance: Qty<ThermalConductance>,
    /// 節点対ごとの有効放射交換面積。取付数では増やさない。
    #[serde(default)]
    pub exchange_area: Option<Qty<Area>>,
    /// 取付の伝導を両端の部品の箱の接触面積から出す入力（書いたときだけ）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub contact: Option<ThermalContact>,
}

/// 節点対 1 つの取付から作った伝導。
#[derive(Debug, Clone, PartialEq)]
pub struct MountConductance {
    pub a: String,
    pub b: String,
    /// 取付の伝導の合計
    pub conductance: Qty<ThermalConductance>,
    /// 接触面積から出した取付があるときの内訳
    pub contact: Option<ThermalContactPair>,
}

/// 取付から作った結合（伝導は節点対ごとの合計、放射は節点対ごとに 1 つ）。
#[derive(Debug, Default)]
pub struct MountLinks {
    pub conductive: Vec<MountConductance>,
    pub radiative: Vec<ThermalRadiativeLink>,
    /// 接触面積から出す設定のときの使われ方
    pub contact: Option<ThermalContactUsed>,
}

/// 節点対の取付を数えるときの途中経過。
#[derive(Default)]
struct PairSum {
    conductance: f64,
    area: f64,
    geometric: u32,
    fixed: u32,
}

impl ThermalMounts {
    /// 並列伝導をまとめ、異なる節点対につき放射結合を 1 つ置く。
    /// 接触面積から出す設定なら、取付ごとに `h × A`（出せなければ 1 取付あたりの値）を足す。
    pub fn links(&self) -> Result<MountLinks, AnalysisError> {
        if let Some(c) = &self.contact
            && c.mounts.len() != self.mounts.len()
        {
            return Err(AnalysisError::InvalidInput(format!(
                "Mount contact boxes ({}) do not match the mounts ({})",
                c.mounts.len(),
                self.mounts.len()
            )));
        }
        let mut used = self.contact.as_ref().map(|c| ThermalContactUsed {
            conductance_per_area: c.conductance_per_area,
            contact_tolerance: DEFAULT_TOLERANCE,
            geometric_mounts: 0,
            fallbacks: Vec::new(),
        });
        let pairs = self.sum_pairs(used.as_mut());
        if let Some(u) = used.as_mut() {
            u.fallbacks
                .sort_by(|x, y| (&x.part, &x.mounted_on).cmp(&(&y.part, &y.mounted_on)));
        }
        let conductive = pairs
            .iter()
            .map(|(&(a, b), sum)| MountConductance {
                a: a.to_owned(),
                b: b.to_owned(),
                conductance: Qty::from_raw(sum.conductance),
                contact: (sum.geometric > 0).then(|| ThermalContactPair {
                    area: Qty::from_raw(sum.area),
                    contact_mounts: sum.geometric,
                    fixed_mounts: sum.fixed,
                }),
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
        Ok(MountLinks {
            conductive,
            radiative,
            contact: used,
        })
    }

    /// 異なる節点の取付を節点対ごとに足す（接触面積から出す設定なら、取付ごとに `h × A`。出せなければ 1 取付あたりの値）。
    fn sum_pairs(
        &self,
        mut used: Option<&mut ThermalContactUsed>,
    ) -> BTreeMap<(&str, &str), PairSum> {
        let mut pairs: BTreeMap<(&str, &str), PairSum> = BTreeMap::new();
        for (i, [a, b]) in self.mounts.iter().enumerate() {
            if a == b {
                continue;
            }
            let key = if a < b {
                (a.as_str(), b.as_str())
            } else {
                (b.as_str(), a.as_str())
            };
            let sum = pairs.entry(key).or_default();
            match self.mount_area(i, used.as_deref_mut()) {
                Some((area, h)) => {
                    sum.conductance += h * area;
                    sum.area += area;
                    sum.geometric += 1;
                }
                None => {
                    sum.conductance += self.conductance.value();
                    sum.fixed += 1;
                }
            }
        }
        pairs
    }

    /// 取付 `i` の（接触面積 [m²]、`h`）。接触面積から出す設定でなければ、出せなければ `None`
    /// （出せない理由は `used.fallbacks` に足し、出せたら `used.geometric_mounts` を数える）。
    fn mount_area(&self, i: usize, used: Option<&mut ThermalContactUsed>) -> Option<(f64, f64)> {
        let (contact, used) = (self.contact.as_ref()?, used?);
        let boxes = &contact.mounts[i];
        match contact_area(boxes) {
            Ok(area) => {
                used.geometric_mounts += 1;
                Some((area, contact.conductance_per_area.value()))
            }
            Err(reason) => {
                used.fallbacks.push(ThermalContactFallback {
                    part: boxes.parts[0].clone(),
                    mounted_on: boxes.parts[1].clone(),
                    reason,
                });
                None
            }
        }
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
            contact: None,
        };
        let links = m.links().unwrap();
        assert_eq!(links.conductive.len(), 1);
        assert_eq!(links.conductive[0].conductance.value(), 1.0); // 0.5 W/K × 2
        assert!(links.conductive[0].contact.is_none());
        assert!(links.contact.is_none());
        assert_eq!(links.radiative.len(), 1);
        assert_eq!(links.radiative[0].exchange_area.value(), 0.02);
    }
}
