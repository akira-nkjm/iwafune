//! 節点の温度範囲をモードでの部品の電源の状態で選ぶ（`ThermalNode::ranges_in`）。電源の状態の読み方は
//! `modes::power_states`、種類の選び方は `ThermalRangeKind::applies`（どちらもここでは決めない）。

use super::{ThermalNode, ThermalNodePart};
use crate::modes::{ModeDef, Population, power_states};
use crate::spec::AnalysisError;
use crate::thermal::range::{
    ThermalNodeRange, ThermalPowerState, ThermalRangeExclusion, ThermalRangeKind, ThermalTempRange,
};

impl ThermalNode {
    /// モード `modes`（ケースの区間ごと。`None` はモードの無い区間）での種類ごとの温度範囲（[`ThermalRangeKind::ALL`] の順。
    /// 範囲を当てる部品が無い種類は出さない）。部品の範囲は、どの区間かでその種類を当てる電源の状態にあれば共通部分に入れ、
    /// どの区間でも当てなければ外した部品として返す。モードが無い・部品に割り当てが無い・電源の状態が決まらない区間が
    /// あれば 3 種すべてを当てる（`modes` が空ならモードの無いケース）。
    pub fn ranges_in(
        &self,
        modes: &[Option<&ModeDef>],
    ) -> Result<Vec<ThermalNodeRange>, AnalysisError> {
        let mut out = Vec::new();
        for kind in ThermalRangeKind::ALL {
            let mut range: Option<ThermalTempRange> = None;
            let mut excluded = Vec::new();
            for p in &self.parts {
                let Some(r) = p.ranges.get(kind) else {
                    continue;
                };
                match p.exclusions(kind, modes)? {
                    Some(why) => excluded.extend(why),
                    None => range = Some(range.map_or(r, |a| a.intersect(r))),
                }
            }
            if let Some(range) = range {
                let (range, required) = match self.required.get(kind.key()) {
                    Some(req) => req.tighten(range),
                    None => (range, Vec::new()),
                };
                out.push(ThermalNodeRange {
                    kind,
                    range,
                    excluded,
                    required,
                });
            }
        }
        Ok(out)
    }
}

impl ThermalNodePart {
    /// モードの電源の状態を数える員数と語彙。
    fn population(&self) -> Population<'_> {
        Population {
            count: self.count,
            redundancy: self.redundancy,
            states: &self.states,
        }
    }

    /// 種類 `kind` の範囲をどの区間でも当てなければ、区間ごとの外した理由。当てる区間があれば `None`。
    fn exclusions(
        &self,
        kind: ThermalRangeKind,
        modes: &[Option<&ModeDef>],
    ) -> Result<Option<Vec<ThermalRangeExclusion>>, AnalysisError> {
        if modes.is_empty() {
            return Ok(None);
        }
        let mut out = Vec::with_capacity(modes.len());
        for mode in modes {
            let Some((mode, a)) = mode.and_then(|m| Some((m, m.assignment(&self.id)?))) else {
                return Ok(None);
            };
            let s = power_states(a, self.population())?;
            if s.is_unknown() || kind.applies(s) {
                return Ok(None);
            }
            out.push(ThermalRangeExclusion {
                part: self.id.clone(),
                mode: mode.id.clone(),
                power_state: if s.on {
                    ThermalPowerState::On
                } else {
                    ThermalPowerState::Off
                },
            });
        }
        Ok(Some(out))
    }
}

#[cfg(test)]
mod tests {
    use crate::modes::ModeDef;
    use crate::spec::Verdict;
    use crate::thermal::network::ThermalNode;
    use crate::thermal::range::{
        ThermalMarginPolicy, ThermalRangeKind, range_checks, range_margins,
    };

    /// 0 °C = 273.15 K。
    const C0: f64 = 273.15;

    /// 節点 ELEC: OBC（動作 −20〜60 °C・保管 −40〜85 °C）と CAM（動作 −10〜40 °C・保管 −30〜60 °C）。
    /// モード SAFE は OBC を on、CAM を off、MISSION は両方 on。
    fn node() -> ThermalNode {
        let part = |id: &str, op: (f64, f64), st: (f64, f64)| {
            serde_json::json!({"id": id, "mass": 1.0, "specific_heat": 900.0,
                "operating": {"min": C0 + op.0, "max": C0 + op.1},
                "storage": {"min": C0 + st.0, "max": C0 + st.1},
                "states": ["off", "on"]})
        };
        serde_json::from_value(serde_json::json!({"id": "ELEC", "parts": [
            part("OBC", (-20.0, 60.0), (-40.0, 85.0)),
            part("CAM", (-10.0, 40.0), (-30.0, 60.0)),
        ]}))
        .unwrap()
    }

    fn mode(id: &str, cam: &str) -> ModeDef {
        serde_json::from_value(serde_json::json!({"id": id, "label": id, "assignments": [
            {"part": "OBC", "state": "on"}, {"part": "CAM", "state": cam}]}))
        .unwrap()
    }

    /// 手計算: 節点の温度 −12 °C（不確かさ 0・警告の幅 5 K）。
    /// - モードなし: 動作は共通部分 −10〜40 °C で下限を 2 K 下回り violation、保管は −30〜60 °C で 18 K（Ok）。
    /// - SAFE（CAM が off）: CAM の動作の範囲の外でも保管の範囲の中なので violation にならない。動作は OBC だけの
    ///   −20〜60 °C で 8 K（Ok）、保管は CAM だけの −30〜60 °C で 18 K（Ok。on の OBC は保管から外す）。
    /// - MISSION（両方 on）: 動作は −10〜40 °C で violation、保管は当てる部品が無いので判定しない。
    #[test]
    fn off_part_is_judged_against_storage_only_by_hand() {
        let n = node();
        let margin = ThermalMarginPolicy::default().used();
        let t = C0 - 12.0;
        let judge = |modes: &[Option<&ModeDef>]| {
            range_margins(t, t, &n.ranges_in(modes).unwrap(), &margin)
                .into_iter()
                .map(|m| {
                    (
                        m.kind,
                        m.verdict,
                        (m.margin_low.value() * 1e9).round() / 1e9,
                    )
                })
                .collect::<Vec<_>>()
        };
        use ThermalRangeKind::{Operating, Storage};
        use Verdict::{Ok, Violation};
        assert_eq!(
            judge(&[]),
            [(Storage, Ok, 18.0), (Operating, Violation, -2.0)]
        );
        assert_eq!(judge(&[None]), judge(&[]));
        let safe = mode("SAFE", "off");
        assert_eq!(
            judge(&[Some(&safe)]),
            [(Storage, Ok, 18.0), (Operating, Ok, 8.0)]
        );
        let mission = mode("MISSION", "on");
        assert_eq!(judge(&[Some(&mission)]), [(Operating, Violation, -2.0)]);
        // 区間が 2 つ（SAFE と MISSION）なら、どちらかの区間で当てる範囲はすべて当てる
        assert_eq!(
            judge(&[Some(&safe), Some(&mission)]),
            [(Storage, Ok, 18.0), (Operating, Violation, -2.0)]
        );
    }

    /// 外した部品は判定文に書く（どの範囲で判定したか）。key は種類のまま。
    #[test]
    fn excluded_parts_are_named_in_the_message() {
        let n = node();
        let margin = ThermalMarginPolicy::default().used();
        let safe = mode("SAFE", "off");
        let m = range_margins(C0, C0, &n.ranges_in(&[Some(&safe)]).unwrap(), &margin);
        let checks = range_checks("ELEC", &m, &margin);
        assert_eq!(checks[0].key, "range_high:ELEC:storage");
        assert_eq!(checks[1].key, "range_low:ELEC:storage");
        assert!(
            checks[0]
                .message
                .contains("Excludes powered-on OBC in mode SAFE"),
            "{}",
            checks[0].message
        );
        assert_eq!(checks[2].key, "range_high:ELEC:operating");
        assert!(
            checks[2]
                .message
                .contains("Excludes powered-off CAM in mode SAFE"),
            "{}",
            checks[1].message
        );
        assert!(
            range_checks(
                "ELEC",
                &range_margins(C0, C0, &n.ranges_in(&[]).unwrap(), &margin),
                &margin
            )
            .iter()
            .all(|c| !c.message.contains("Excludes"))
        );
    }

    /// 割り当ての無い部品（構体など）はモードがあっても 3 種すべてを当てる。
    #[test]
    fn unassigned_parts_keep_all_ranges() {
        let n = node();
        let only_obc: ModeDef = serde_json::from_value(serde_json::json!({"id": "M", "label": "M",
            "assignments": [{"part": "OBC", "state": "off"}]}))
        .unwrap();
        let r = n.ranges_in(&[Some(&only_obc)]).unwrap();
        // 保管: OBC（off）と CAM（割り当て無し）の共通部分 −30〜60 °C、動作: CAM だけ −10〜40 °C
        let kinds: Vec<_> = r
            .iter()
            .map(|r| (r.kind, (r.range.min.value() - C0).round()))
            .collect();
        assert_eq!(
            kinds,
            [
                (ThermalRangeKind::Storage, -30.0),
                (ThermalRangeKind::Operating, -10.0)
            ]
        );
        assert_eq!(r[1].excluded.len(), 1);
    }
}
