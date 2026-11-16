//! 部品・フォルダの系（`Design::system_of`）。置き場所（`design`）と組み込みの語彙の役割 → 系（`builtin` の `role_system`）の
//! 両方を見るので、`design` と `builtin` の循環を避けて、両方より上のこの葉のモジュールに置く。

use crate::design::{AnalysisSystem, Design, NodeRef};

impl Design {
    /// 部品・フォルダの系。置き場所の最上位の系を優先し、部品なら次に種別の役割の系、どちらも無ければ None。
    pub fn system_of(&self, part: &str) -> Option<AnalysisSystem> {
        let node = self.node(part)?;
        let ancestors = self.ancestors(&node);
        ancestors
            .last()
            .or_else(|| matches!(node, NodeRef::Group(_)).then_some(&node))
            .and_then(|n| n.id().parse().ok())
            .or_else(|| {
                let part = self.parts.get(part)?;
                self.kind_system(self.part_kind_id(part)?)
            })
    }

    /// 種別の系（種別の役割（継承を含む）の系。役割が無い、または役割に系が無ければ None）。
    /// 部品の系（`system_of`）とカタログ品の系（`CatalogItemView.system`）が共有する唯一の判定
    pub fn kind_system(&self, kind: &str) -> Option<AnalysisSystem> {
        let role = self.kind_role(kind)?;
        crate::builtin::labels()
            .role_system
            .get(&role)?
            .as_deref()?
            .parse()
            .ok()
    }
}
