//! 名前の変わった設定のキー。古い名前は読めるまま（model の serde の `alias`）warning にし（`toml.renamed_key`）、
//! `fune fmt` と設定の書き込みが新しい名前に書き換える。**古いキーと新しいキーの対応はここだけ**
//! （model の `alias` との一致は `store/tests/renamed_keys.rs` が確かめる）。

use crate::layout::analysis_file;
use fune_model::design::AnalysisSystem;
use toml_edit::{Item, Key, Table, TableLike};

/// 表のパスの任意の名前（`[case.<名前>]` の `<名前>`）。
const ANY: &str = "*";

/// 名前の変わったキー 1 つ。
#[derive(Debug, Clone, Copy)]
pub struct RenamedKey {
    /// 設定のファイルの系（`analysis/<系>.toml`）
    pub system: AnalysisSystem,
    /// キーを持つ表のパス（[`ANY`] は任意の名前）
    pub table: &'static [&'static str],
    /// 古い名前
    pub old: &'static str,
    /// 新しい名前
    pub new: &'static str,
}

/// 名前の変わったキーの一覧。
pub const RENAMED_KEYS: &[RenamedKey] = &[
    // 中心天体の一律の赤外放射（地球に限らないので。#132）
    RenamedKey {
        system: AnalysisSystem::Thermal,
        table: &["case", ANY],
        old: "earth_ir",
        new: "planet_ir",
    },
];

/// ファイル `path` に当てるキーの名前の変更。
fn rules(path: &str) -> impl Iterator<Item = &'static RenamedKey> + '_ {
    RENAMED_KEYS
        .iter()
        .filter(move |r| analysis_file(r.system) == path)
}

/// 表のパス `table` に当たる表を集める（[`ANY`] はすべての子の表）。
fn tables<'t>(root: &'t dyn TableLike, table: &[&str], out: &mut Vec<&'t dyn TableLike>) {
    let Some((first, rest)) = table.split_first() else {
        out.push(root);
        return;
    };
    for (k, item) in root.iter() {
        if *first != ANY && k != *first {
            continue;
        }
        if let Some(t) = item.as_table_like() {
            tables(t, rest, out);
        }
    }
}

/// 古い名前のキー（ファイルの中の位置の順ではなく、規則と表の順）。
pub fn old_keys<'d>(path: &str, root: &'d dyn TableLike) -> Vec<(&'d Key, &'static RenamedKey)> {
    let mut out = Vec::new();
    for rule in rules(path) {
        let mut found = Vec::new();
        tables(root, rule.table, &mut found);
        out.extend(
            found
                .into_iter()
                .filter_map(|t| t.get_key_value(rule.old).map(|(k, _)| (k, rule))),
        );
    }
    out
}

/// 古い名前のキーを新しい名前に書き換える（キーの位置と値・コメントはそのまま。見出しの表は `=` を揃え直す）。
/// 両方あれば古い名前を消す（新しい名前の値を残す）。
pub fn rename_keys(path: &str, root: &mut Table) {
    for rule in rules(path) {
        match rule.table.split_first() {
            Some((first, rest)) => rename_under(root, first, rest, rule),
            None => rename_at(root, rule),
        }
    }
}

/// 設定のキーの列（`[case, hot, earth_ir]` など）の最後が古い名前なら新しい名前にする（`fune set` で古い名前を打っても
/// 新しい名前に書く）。
pub fn current_keys(path: &str, keys: &[String]) -> Vec<String> {
    let mut out = keys.to_vec();
    if let Some((last, parents)) = out.split_last_mut() {
        let hit = rules(path).find(|r| {
            *last == r.old
                && r.table.len() == parents.len()
                && r.table
                    .iter()
                    .zip(parents.iter())
                    .all(|(t, k)| *t == ANY || t == k)
        });
        if let Some(rule) = hit {
            *last = rule.new.to_owned();
        }
    }
    out
}

/// `t` の子のうち `first` に当たる表の下（残りのパス `rest`）で書き換える。
fn rename_under(t: &mut dyn TableLike, first: &str, rest: &[&str], rule: &RenamedKey) {
    for (k, item) in t.iter_mut() {
        if first != ANY && k.get() != first {
            continue;
        }
        match (rest.split_first(), item) {
            (Some((next, rest)), item) => {
                if let Some(child) = item.as_table_like_mut() {
                    rename_under(child, next, rest, rule);
                }
            }
            (None, Item::Table(child)) => rename_at(child, rule),
            (None, item) => {
                if let Some(child) = item.as_table_like_mut() {
                    rename_one(child, rule, false);
                }
            }
        }
    }
}

/// 見出しの表で書き換え、書き換えたら `=` を揃え直す。
fn rename_at(t: &mut Table, rule: &RenamedKey) {
    if rename_one(t, rule, true) {
        super::align_rows(t);
    }
}

/// 表の中で 1 つのキーを書き換える（書き換えたか）。表の行の順を保つため、後ろの行を抜いて入れ直す。
/// 見出しの表（`aligned`）ではキーの後ろの空白を捨てて揃え直しに任せ、インライン表では空白も残す。
fn rename_one(t: &mut dyn TableLike, rule: &RenamedKey, aligned: bool) -> bool {
    if !t.contains_key(rule.old) {
        return false;
    }
    if t.contains_key(rule.new) {
        t.remove(rule.old);
        return true;
    }
    let keys: Vec<String> = t.iter().map(|(k, _)| k.to_owned()).collect();
    let moved: Vec<(Key, Item)> = keys
        .iter()
        .skip_while(|k| k.as_str() != rule.old)
        .filter_map(|k| {
            let key = t.key(k)?.clone();
            let item = t.remove(k)?;
            if key.get() != rule.old {
                return Some((key, item));
            }
            let mut new = Key::new(rule.new);
            let decor = key.leaf_decor();
            if aligned {
                if let Some(prefix) = decor.prefix() {
                    new.leaf_decor_mut().set_prefix(prefix.clone());
                }
            } else {
                *new.leaf_decor_mut() = decor.clone();
            }
            Some((new, item))
        })
        .collect();
    for (key, item) in moved {
        t.entry_format(&key).or_insert(item);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml_edit::DocumentMut;

    const OLD: &str = "[case.hot]\nalbedo   = 0.35\nearth_ir = { value = 258.0, unit = \"W/m^2\" }  # 夏\nbeta     = { value = 60.0, unit = \"deg\" }\n";

    #[test]
    fn renames_in_place_and_keeps_comments() {
        let mut doc: DocumentMut = OLD.parse().unwrap();
        assert_eq!(old_keys("analysis/thermal.toml", doc.as_table()).len(), 1);
        assert!(old_keys("analysis/power.toml", doc.as_table()).is_empty());
        rename_keys("analysis/thermal.toml", doc.as_table_mut());
        let text = doc.to_string();
        let keys: Vec<&str> = text
            .lines()
            .filter_map(|l| l.split_once('=').map(|(k, _)| k.trim()))
            .collect();
        assert_eq!(keys, ["albedo", "planet_ir", "beta"]);
        assert!(text.contains("# 夏"));
        assert!(old_keys("analysis/thermal.toml", doc.as_table()).is_empty());
    }

    /// 揃え直し: 同じ表の `=` は書き換えたあとも揃う。
    #[test]
    fn realigns_the_table() {
        let mut doc: DocumentMut = "[case.hot]\nsolar_flux = 1.0\nearth_ir   = 2.0\n"
            .parse()
            .unwrap();
        rename_keys("analysis/thermal.toml", doc.as_table_mut());
        assert_eq!(
            doc.to_string(),
            "[case.hot]\nsolar_flux = 1.0\nplanet_ir  = 2.0\n"
        );
    }

    #[test]
    fn maps_old_setting_keys() {
        let keys = |k: &[&str]| k.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let path = "analysis/thermal.toml";
        assert_eq!(
            current_keys(path, &keys(&["case", "hot", "earth_ir"])),
            keys(&["case", "hot", "planet_ir"])
        );
        assert_eq!(
            current_keys(path, &keys(&["earth_ir"])),
            keys(&["earth_ir"])
        );
        assert_eq!(
            current_keys("analysis/power.toml", &keys(&["case", "hot", "earth_ir"])),
            keys(&["case", "hot", "earth_ir"])
        );
    }

    #[test]
    fn keeps_the_new_value_when_both_are_written() {
        let both = format!("{OLD}planet_ir = {{ value = 237.0, unit = \"W/m^2\" }}\n");
        let mut doc: DocumentMut = both.parse().unwrap();
        rename_keys("analysis/thermal.toml", doc.as_table_mut());
        let text = doc.to_string();
        assert!(!text.contains("earth_ir"));
        assert!(text.contains("237.0"));
    }
}
