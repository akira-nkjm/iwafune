//! 組み込みと設計固有の語彙の検索。
use crate::{builtin, design::*};
use std::collections::BTreeMap;
impl Design {
    /// 属性定義（プロジェクト独自 → 組み込みの順に引く）。
    pub fn attribute(&self, id: &str) -> Option<&AttributeDef> {
        self.attributes
            .get(id)
            .or_else(|| builtin::attributes().get(id))
    }

    /// 種別定義（プロジェクト独自 → 組み込みの順に引く）。
    pub fn kind(&self, id: &str) -> Option<&KindDef> {
        self.kinds.get(id).or_else(|| builtin::kinds().get(id))
    }

    /// 属性の系（定義の `section`。無ければ [`builtin::DEFAULT_SECTION`]）。画面の区分はこれで決める。
    pub fn attribute_section(&self, id: &str) -> &str {
        self.attribute(id)
            .and_then(|d| d.section.as_deref())
            .unwrap_or(builtin::DEFAULT_SECTION)
    }

    /// 組み込みとプロジェクト独自を合わせた属性定義の一覧（ID 順）。
    pub fn all_attributes(&self) -> BTreeMap<&str, &AttributeDef> {
        let mut all: BTreeMap<&str, &AttributeDef> = builtin::attributes()
            .iter()
            .map(|(k, v)| (k.as_str(), v))
            .collect();
        all.extend(self.attributes.iter().map(|(k, v)| (k.as_str(), v)));
        all
    }

    /// 組み込みとプロジェクト独自を合わせた種別定義の一覧（ID 順）。
    pub fn all_kinds(&self) -> BTreeMap<&str, &KindDef> {
        let mut all: BTreeMap<&str, &KindDef> = builtin::kinds()
            .iter()
            .map(|(k, v)| (k.as_str(), v))
            .collect();
        all.extend(self.kinds.iter().map(|(k, v)| (k.as_str(), v)));
        all
    }

    /// 種別の継承の連鎖（自分 → 継承元の順。循環や未定義はそこで止める）。
    pub fn kind_chain(&self, id: &str) -> Vec<&KindDef> {
        let mut chain: Vec<&KindDef> = Vec::new();
        let mut next = self.kind(id);
        while let Some(kind) = next {
            if chain.iter().any(|k| k.id == kind.id) {
                break;
            }
            chain.push(kind);
            next = kind.extends.as_ref().and_then(|e| self.kind(e.as_str()));
        }
        chain
    }
}
