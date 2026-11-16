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
                let role = self.part_kind_id(part).and_then(|k| self.kind_role(k))?;
                crate::builtin::labels()
                    .role_system
                    .get(&role)?
                    .as_deref()?
                    .parse()
                    .ok()
            })
    }
}
