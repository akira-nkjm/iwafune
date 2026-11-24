//! 参照の索引のうち、運用エンティティ（ハーネス・解析ケース・出典・マヌーバ・通信の回線）の ID の参照元。
//! 親（`refs.rs`）の `Design::references` が引く。参照元の種類は [`RefSite`]。

use super::RefSite;
use crate::design::{Design, EntityKind};
use crate::value::{Authored, Value};

impl Design {
    /// ハーネス区間を指すものは無い（名前の変更の索引を揃えるための空の索引）。
    pub(super) fn harness_refs(&self, _id: &str) -> Vec<RefSite> {
        Vec::new()
    }

    /// 解析ケースを指すもの: 共通設定の既定のケース → 要求の量化。
    pub(super) fn case_refs(&self, id: &str) -> Vec<RefSite> {
        let mut out = Vec::new();
        let is_default = self
            .analysis
            .common
            .as_ref()
            .is_some_and(|c| c.default_case.as_deref() == Some(id));
        if is_default {
            out.push(RefSite::CommonDefaultCase);
        }
        out.extend(
            self.requirements
                .values()
                .filter(|r| r.quantify.cases.iter().any(|c| c.as_str() == id))
                .map(|r| RefSite::RequirementCase {
                    requirement: r.id.to_string(),
                }),
        );
        out
    }

    /// 出典を指すもの: 部品の値 → カタログ品（`source`・`datasheet`・値）→ 要求の順。
    pub(super) fn source_refs(&self, id: &str) -> Vec<RefSite> {
        let cites = |v: &Authored<Value>| value_cites(v, id);
        let mut out = Vec::new();
        for p in self.parts.values() {
            for attr in p.attr.iter().filter(|(_, v)| cites(v)).map(|(a, _)| a) {
                out.push(RefSite::PartValueSource {
                    part: p.id.to_string(),
                    attr: attr.to_string(),
                });
            }
        }
        for (reference, item) in &self.catalog {
            let item_id = || reference.to_string();
            if item.source.as_deref() == Some(id) {
                out.push(RefSite::CatalogItemSource { item: item_id() });
            }
            if item.datasheet.as_deref() == Some(id) {
                out.push(RefSite::CatalogItemDatasheet { item: item_id() });
            }
            for attr in item.attr.iter().filter(|(_, v)| cites(v)).map(|(a, _)| a) {
                out.push(RefSite::CatalogValueSource {
                    item: item_id(),
                    attr: attr.to_string(),
                });
            }
        }
        out.extend(
            self.requirements
                .values()
                .filter(|r| r.source.as_ref().is_some_and(|s| s.as_str() == id))
                .map(|r| RefSite::RequirementSource {
                    requirement: r.id.to_string(),
                }),
        );
        out
    }

    /// マヌーバを指すもの: ほかのマヌーバの `after` → 設計の軌道の `maneuver` → フェーズの軌道の `maneuver`。
    pub(super) fn maneuver_refs(&self, id: &str) -> Vec<RefSite> {
        let after = self
            .maneuvers
            .values()
            .filter(|m| m.after.as_deref() == Some(id))
            .map(|m| RefSite::ManeuverAfter {
                maneuver: m.id.to_string(),
            });
        let design_orbit = self
            .conditions
            .orbit
            .as_ref()
            .filter(|o| o.maneuver.as_deref() == Some(id))
            .map(|_| RefSite::OrbitManeuver);
        let phase_orbits = self
            .phases
            .values()
            .filter(|p| {
                p.orbit
                    .as_ref()
                    .is_some_and(|o| o.maneuver.as_deref() == Some(id))
            })
            .map(|p| RefSite::PhaseOrbitManeuver {
                phase: p.id.to_string(),
            });
        after.chain(design_orbit).chain(phase_orbits).collect()
    }

    /// 通信の回線を指すもの: 下りの回線の選択 → 判定のキーが名指す要求。
    pub(super) fn link_refs(&self, id: &str) -> Vec<RefSite> {
        let mut out = Vec::new();
        let chosen = self
            .analysis
            .comm
            .as_ref()
            .is_some_and(|c| c.data_downlink.as_deref() == Some(id));
        if chosen {
            out.push(RefSite::CommDataDownlink);
        }
        out.extend(self.check_key_refs(EntityKind::Link, id));
        out
    }
}

/// 値（と状態ごとのセル）のどれかの `from.source` が `source` か。
fn value_cites(value: &Authored<Value>, source: &str) -> bool {
    let cited = |from: Option<&crate::value::Provenance>| {
        from.and_then(|f| f.source.as_deref()) == Some(source)
    };
    cited(value.from.as_ref())
        || matches!(&value.value, Value::ByState(cells)
            if cells.values().any(|c| cited(c.from.as_ref())))
}
