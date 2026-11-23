//! 要求の判定のキーが名指す名前の参照元。**キーの書式は `sat_checkkey` が持ち**（解析のキーの組み立てと同じ表）、ここは
//! 「どの要求の `[verify].check` が、このエンティティを名指しているか」を索引（`Design::references`）に載せるだけ。
//! 名前の変更ではそのキーを書き換え（`fune_store::write::refs`）、削除は fune-ops が拒む。

use super::RefSite;
use crate::design::{Design, EntityKind};

impl Design {
    /// `kind` のエンティティ `name` を判定のキーで名指している要求（要求の ID 順）。
    pub(super) fn check_key_refs(&self, kind: EntityKind, name: &str) -> Vec<RefSite> {
        self.requirements
            .values()
            .filter(|r| r.verify.check_names(kind, name))
            .map(|r| RefSite::RequirementCheck {
                requirement: r.id.to_string(),
                entity: kind,
            })
            .collect()
    }

    /// 熱の節点（`analysis/thermal.toml` の `[nodes]` のキー）を使っているもの。節点を名前で指すのは、
    /// 温度範囲の判定のキー（`range_low:<節点>:<種類>`）を書いた要求だけ。節点の名前を変える Op は無い
    /// （`SetSetting` で `[nodes]` を直す）ので、書き換えはなく、節点を消す設定の書き込みを fune-ops が拒むための索引。
    pub(super) fn thermal_node_refs(&self, name: &str) -> Vec<RefSite> {
        self.check_key_refs(EntityKind::ThermalNode, name)
    }
}
