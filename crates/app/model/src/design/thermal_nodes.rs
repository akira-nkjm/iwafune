//! 熱の節点の対応（部品 → 節点）。**節点がどの部品からできるかを決めるのはここだけ。**
//!
//! 既定は「1 部品 = 1 節点」（節点の名前は部品の ID）。`analysis/thermal.toml` の `[nodes]`（節点 → 部品の一覧）は、
//! 複数の部品を 1 節点にまとめたいときだけ書く上書きで、そこに書いた部品はその節点に入り、自分の節点を持たない。
//! 節点の名前は部品の ID と同じ名前空間にあるので、`[nodes]` のキーが、`[nodes]` に載っていない部品の ID と同じになれば
//! 名前がぶつかる（`clashes`。`check` が `thermal.node_name_clash` の誤りにする）。ぶつかった部品は節点を持たない。
//!
//! 節点に熱容量があるか（質量・比熱・員数）はここでは見ない（部品の値は解決後の設計にあり、足りなければ射影が不足入力にする。
//! 熱容量が 0 の節点は解析が入力の誤りにする）。部品の集合は呼び出し側が渡す（設計の部品、または解決後の部品）。

use super::thermal_settings::ThermalSettings;
use std::collections::BTreeMap;

/// 節点 1 つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThermalNodeDef {
    /// 節点に入る部品（`[nodes]` に書いた順。部品ごとの節点では自分 1 つ）
    pub parts: Vec<String>,
    /// `[nodes]` に書いた節点（部品をまとめる上書き）。偽なら既定の 1 部品 1 節点
    pub lumped: bool,
}

/// 熱の節点と、部品 → 節点の対応（[`ThermalSettings::node_map`]）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThermalNodes {
    nodes: BTreeMap<String, ThermalNodeDef>,
    node_of: BTreeMap<String, String>,
    clashes: Vec<String>,
}

impl ThermalNodes {
    /// 節点の名前順の一覧。
    pub fn iter(&self) -> impl Iterator<Item = (&str, &ThermalNodeDef)> {
        self.nodes.iter().map(|(n, d)| (n.as_str(), d))
    }

    /// 節点 `name` があるか。
    pub fn contains(&self, name: &str) -> bool {
        self.nodes.contains_key(name)
    }

    /// 部品 `part` の属する節点（節点を持たない部品は `None`）。
    pub fn node_of(&self, part: &str) -> Option<&str> {
        self.node_of.get(part).map(String::as_str)
    }

    /// `[nodes]` のキーと名前がぶつかって節点を持てなかった部品（名前順）。
    pub fn clashes(&self) -> &[String] {
        &self.clashes
    }

    /// 部品 `part` が既定の 1 部品 1 節点の節点（自分の名前の節点）か。
    pub fn is_per_part(&self, part: &str) -> bool {
        self.nodes.get(part).is_some_and(|d| !d.lumped)
    }
}

impl ThermalSettings {
    /// 部品の集合 `parts` に対する熱の節点。`[nodes]` に書いた節点はそのまま、書かれていない部品は自分の ID の節点になる。
    /// 部品が 2 つの節点に書かれていれば、名前が後（辞書順）の節点を部品の節点にする（`check` が `thermal.node_overlap` にする）。
    pub fn node_map<'a>(&self, parts: impl IntoIterator<Item = &'a str>) -> ThermalNodes {
        let mut out = ThermalNodes::default();
        for (node, members) in &self.nodes {
            let def = ThermalNodeDef {
                parts: members.clone(),
                lumped: true,
            };
            out.nodes.insert(node.clone(), def);
            for part in members {
                out.node_of.insert(part.clone(), node.clone());
            }
        }
        for part in parts {
            if out.node_of.contains_key(part) {
                continue;
            }
            if out.nodes.contains_key(part) {
                out.clashes.push(part.to_owned());
                continue;
            }
            let def = ThermalNodeDef {
                parts: vec![part.to_owned()],
                lumped: false,
            };
            out.nodes.insert(part.to_owned(), def);
            out.node_of.insert(part.to_owned(), part.to_owned());
        }
        out.clashes.sort();
        out
    }
}

impl ThermalSettings {
    /// 節点の名前 `name` を設定に書いているか（明示の結合の両端と、発熱の場所の値）。部品の名前の変更が書き換える先の判定。
    pub fn names_node(&self, name: &str) -> bool {
        let in_pairs = |table: &BTreeMap<String, BTreeMap<String, crate::value::Quantity>>| {
            table
                .iter()
                .any(|(a, row)| a == name || row.contains_key(name))
        };
        in_pairs(&self.conductive)
            || in_pairs(&self.radiative)
            || self.heat_node.values().any(|n| n == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(nodes: &[(&str, &[&str])]) -> ThermalSettings {
        let mut s = ThermalSettings::default();
        for (n, parts) in nodes {
            s.nodes
                .insert((*n).into(), parts.iter().map(|p| (*p).into()).collect());
        }
        s
    }

    #[test]
    fn unlisted_parts_are_their_own_nodes() {
        let m = settings(&[("N", &["A", "B"])]).node_map(["A", "B", "C"]);
        let names: Vec<_> = m.iter().map(|(n, d)| (n, d.lumped)).collect();
        assert_eq!(names, [("C", false), ("N", true)]);
        assert_eq!(m.node_of("A"), Some("N"));
        assert_eq!(m.node_of("C"), Some("C"));
        assert!(m.is_per_part("C") && !m.is_per_part("A"));
        assert!(m.clashes().is_empty());
    }

    #[test]
    fn explicit_node_named_like_an_unlisted_part_clashes() {
        // 節点 `C` は A だけをまとめる。部品 C は `[nodes]` に載っていないので、節点 `C` と名前がぶつかる
        let m = settings(&[("C", &["A"])]).node_map(["A", "C"]);
        assert_eq!(m.clashes(), ["C"]);
        assert_eq!(m.node_of("C"), None);
        assert_eq!(m.node_of("A"), Some("C"));
        // 部品 C を自分の節点に書いてあればぶつからない
        let m = settings(&[("C", &["C", "A"])]).node_map(["A", "C"]);
        assert!(m.clashes().is_empty());
        // 部品 C が別の節点に書かれていれば、節点 `C` の名前は空いている
        let m = settings(&[("C", &["A"]), ("D", &["C"])]).node_map(["A", "C"]);
        assert!(m.clashes().is_empty());
        assert_eq!(m.node_of("C"), Some("D"));
    }
}
