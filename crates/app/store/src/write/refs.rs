//! 名前の変更・削除に伴う参照の書き換え（どのキーが部品・モード・ネットの ID を持つかを知っているのはここ）。
//!
//! | 参照 | 置き場所 |
//! |---|---|
//! | 部品 | modes の `[states]` のキー・nets の `source` / `master`・harness の `from` / `to`・部品の `mount.on`・配分のキー・`analysis/comm.toml` の `part`・`analysis/thermal.toml` の `nodes` |
//! | モード | cases の `mode`・`analysis/power.toml` の `mode_share` と `sequence`・`analysis/attitude.toml` の `law`・`analysis/thermal.toml` の `case.*.mode` |
//! | ネット | 部品のポートの `net`・harness の `nets` |
//! | カタログ品 | 部品の `catalog`・`candidates`・harness の `wire` |

use super::Edits;
use super::doc::{set, string, table_opt};
use super::part::{rename_mount_target, set_port};
use crate::error::StoreError;
use crate::layout;
use fune_model::Design;
use fune_model::path::PortRef;
use toml_edit::{Item, Key, Table, Value};

/// 表のキーの名前を変える（行の装飾と `=` の位置を保つ）。変えたら true。
pub fn rename_key(table: &mut Table, from: &str, to: &str) -> bool {
    let sorted = {
        let keys: Vec<&str> = table.iter().map(|(k, _)| k).collect();
        keys.windows(2).all(|w| w[0] <= w[1])
    };
    let Some((old, item)) = table.remove_entry(from) else {
        return false;
    };
    let mut key = Key::new(to);
    let mut decor = old.leaf_decor().clone();
    let old_suffix = decor.suffix().and_then(|s| s.as_str()).map_or(1, str::len);
    let total = old.display_repr().len() + old_suffix;
    let new_len = key.display_repr().len();
    decor.set_suffix(" ".repeat(total.saturating_sub(new_len).max(1)));
    *key.leaf_decor_mut() = decor;
    table.insert_formatted(&key, item);
    if sorted {
        table.sort_values();
    }
    true
}

/// 部品 `from` のポートなら、部品を `to` に替えた参照の書き方（`Display`）。
fn rename_port_owner(port: &PortRef, from: &str, to: &str) -> Option<String> {
    (port.part == from).then(|| PortRef::new(to, port.port.as_str()).to_string())
}

/// 部品の名前を変える（ファイルと組立品のディレクトリの移動・参照の書き換え）。
pub fn rename_part(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    let old_file =
        layout::part_file(design, from).ok_or_else(|| StoreError::Missing { path: from.into() })?;
    let dir = old_file
        .trim_end_matches(&format!("{from}.toml"))
        .to_string();
    edits.rename(&old_file, &format!("{dir}{to}.toml"))?;
    edits.rename_dir(&format!("{dir}{from}"), &format!("{dir}{to}"))?;
    for mode in design
        .modes
        .values()
        .filter(|m| m.states.contains_key(from))
    {
        edits.edit(&layout::mode_file(mode.id.as_str()), |doc| {
            if let Some(t) = table_opt(doc, &["states"]) {
                rename_key(t, from, to);
            }
            Ok(())
        })?;
    }
    rename_port_refs(edits, design, from, to)?;
    for part in design
        .parts
        .values()
        .filter(|p| p.mount.as_ref().is_some_and(|m| m.on.as_str() == from))
    {
        // 子部品は組立品のディレクトリごと移っているので、移った先のファイルを書き換える。
        let mut file = layout::part_file(design, part.id.as_str()).unwrap_or_default();
        if let Some(rest) = file.strip_prefix(&format!("{dir}{from}/")) {
            file = format!("{dir}{to}/{rest}");
        }
        rename_mount_target(edits, &file, to)?;
    }
    rename_budget_node(edits, design, from, to)?;
    rename_part_in_settings(edits, design, from, to)
}

/// グループの名前を変える。`parts/` の下のディレクトリを移し、fune.toml の `[groups]` の表示名のキー・配分のキー・
/// 図の配置（`view/*.toml` の `[nodes]`）のキーを書き換える。部品はディレクトリごと移るので、中の部品の参照は変わらない。
pub fn rename_group(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    let node = design
        .node(from)
        .ok_or_else(|| StoreError::Missing { path: from.into() })?;
    let dirs = layout::dirs_for_parent(design, Some(&node));
    let parent: String = dirs[..dirs.len().saturating_sub(1)]
        .iter()
        .map(|d| format!("{d}/"))
        .collect();
    edits.rename_dir(
        &format!("{}/{parent}{from}", layout::PARTS_DIR),
        &format!("{}/{parent}{to}", layout::PARTS_DIR),
    )?;
    edits.edit(layout::PROJECT_FILE, |doc| {
        if let Some(t) = table_opt(doc, &["groups"]) {
            rename_key(t, from, to);
        }
        Ok(())
    })?;
    rename_budget_node(edits, design, from, to)?;
    for view in design.views.values().filter(|v| v.nodes.contains_key(from)) {
        edits.edit(&layout::view_file(view.id.as_str()), |doc| {
            if let Some(t) = table_opt(doc, &["nodes"]) {
                rename_key(t, from, to);
            }
            Ok(())
        })?;
    }
    Ok(())
}

/// 文書の見出しのキーに書き換えを当てる（値があるものだけ）。
fn set_strings(
    edits: &mut Edits,
    file: &str,
    values: Vec<(&str, Option<String>)>,
) -> Result<(), StoreError> {
    if values.iter().all(|(_, v)| v.is_none()) {
        return Ok(());
    }
    edits.edit(file, |doc| {
        for (key, value) in values {
            if let Some(v) = value {
                set(doc.as_table_mut(), key, string(&v));
            }
        }
        Ok(())
    })
}

/// ネットの source / master、ハーネスの from / to の `PART.PORT` を書き換える。
fn rename_port_refs(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    let owner = |p: Option<&PortRef>| p.and_then(|p| rename_port_owner(p, from, to));
    for net in design.nets.values() {
        let values = vec![
            ("source", owner(net.source.as_ref())),
            ("master", owner(net.master.as_ref())),
        ];
        set_strings(edits, &layout::net_file(net.id.as_str()), values)?;
    }
    for h in design.harnesses.values() {
        let values = vec![("from", owner(Some(&h.from))), ("to", owner(Some(&h.to)))];
        set_strings(edits, &layout::harness_file(h.id.as_str()), values)?;
    }
    Ok(())
}

fn rename_budget_node(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    let present = design
        .budget
        .allocation
        .iter()
        .any(|(_, a)| a.alloc.contains_key(from));
    if !present {
        return Ok(());
    }
    edits.edit(layout::ALLOCATION_FILE, |doc| {
        for kind in ["mass", "power", "data"] {
            if let Some(t) = table_opt(doc, &[kind, "alloc"]) {
                rename_key(t, from, to);
            }
        }
        Ok(())
    })
}

/// 文字列の配列の要素を置き換える。
fn replace_in_array(item: &mut Item, from: &str, to: &str) {
    if let Some(array) = item.as_array_mut() {
        for v in array.iter_mut() {
            if v.as_str() == Some(from) {
                let decor = v.decor().clone();
                *v = Value::from(to);
                *v.decor_mut() = decor;
            }
        }
    }
}

fn rename_part_in_settings(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    if design
        .analysis
        .comm
        .as_ref()
        .is_some_and(|c| c.links.values().any(|l| l.part == from))
    {
        edits.edit(&layout::analysis_file("comm"), |doc| {
            retarget_subtables(doc, "link", "part", from, to);
            Ok(())
        })?;
    }
    if design
        .analysis
        .thermal
        .as_ref()
        .is_some_and(|t| t.nodes.values().any(|n| n.iter().any(|p| p == from)))
    {
        edits.edit(&layout::analysis_file("thermal"), |doc| {
            if let Some(nodes) = table_opt(doc, &["nodes"]) {
                for (_, item) in nodes.iter_mut() {
                    replace_in_array(item, from, to);
                }
            }
            Ok(())
        })?;
    }
    Ok(())
}

/// モードの名前を変える（ファイルの移動・ケースと解析の設定の参照の書き換え）。
pub fn rename_mode(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    edits.rename(&layout::mode_file(from), &layout::mode_file(to))?;
    for case in design
        .cases
        .values()
        .filter(|c| c.mode.as_ref().is_some_and(|m| m.as_str() == from))
    {
        edits.edit(&layout::case_file(case.id.as_str()), |doc| {
            set(doc.as_table_mut(), "mode", string(to));
            Ok(())
        })?;
    }
    rename_mode_in_settings(edits, design, from, to)
}

/// `[<table>.*]` の各表のうち `key = from` のものを `to` に書き換える。
fn retarget_subtables(
    doc: &mut toml_edit::DocumentMut,
    table: &str,
    key: &str,
    from: &str,
    to: &str,
) {
    if let Some(parent) = table_opt(doc, &[table]) {
        for (_, item) in parent.iter_mut() {
            if let Some(t) = item.as_table_mut()
                && t.get(key).and_then(Item::as_str) == Some(from)
            {
                set(t, key, string(to));
            }
        }
    }
}

fn rename_mode_in_settings(
    edits: &mut Edits,
    design: &Design,
    from: &str,
    to: &str,
) -> Result<(), StoreError> {
    let analysis = &design.analysis;
    if let Some(p) = &analysis.power
        && (p.mode_share.contains_key(from) || p.sequence.iter().any(|s| s.mode == from))
    {
        edits.edit(&layout::analysis_file("power"), |doc| {
            if let Some(t) = table_opt(doc, &["mode_share"]) {
                rename_key(t, from, to);
            }
            if let Some(seq) = doc
                .get_mut("sequence")
                .and_then(Item::as_array_of_tables_mut)
            {
                for t in seq.iter_mut() {
                    if t.get("mode").and_then(Item::as_str) == Some(from) {
                        set(t, "mode", string(to));
                    }
                }
            }
            Ok(())
        })?;
    }
    if analysis
        .attitude
        .as_ref()
        .is_some_and(|a| a.law.contains_key(from))
    {
        edits.edit(&layout::analysis_file("attitude"), |doc| {
            if let Some(t) = table_opt(doc, &["law"]) {
                rename_key(t, from, to);
            }
            Ok(())
        })?;
    }
    let thermal_refs = analysis
        .thermal
        .as_ref()
        .is_some_and(|t| t.cases.values().any(|c| c.mode.as_deref() == Some(from)));
    if thermal_refs {
        edits.edit(&layout::analysis_file("thermal"), |doc| {
            retarget_subtables(doc, "case", "mode", from, to);
            Ok(())
        })?;
    }
    Ok(())
}

/// 部品の削除に伴い、モードの行と配分を消す。
pub fn forget_part(edits: &mut Edits, design: &Design, id: &str) -> Result<(), StoreError> {
    for mode in design.modes.values().filter(|m| m.states.contains_key(id)) {
        super::entities::set_mode_row(edits, &layout::mode_file(mode.id.as_str()), id, None)?;
    }
    if design
        .budget
        .allocation
        .iter()
        .any(|(_, a)| a.alloc.contains_key(id))
    {
        edits.edit(layout::ALLOCATION_FILE, |doc| {
            for kind in ["mass", "power", "data"] {
                if let Some(t) = table_opt(doc, &[kind, "alloc"]) {
                    t.remove(id);
                }
            }
            Ok(())
        })?;
    }
    Ok(())
}

/// ネットの削除に伴い、部品のポートとハーネスから外す。
pub fn forget_net(edits: &mut Edits, design: &Design, net: &str) -> Result<(), StoreError> {
    for part in design.parts.values() {
        for (port, binding) in &part.ports {
            if binding.net.as_ref().is_some_and(|n| n.as_str() == net) {
                let file = layout::part_file(design, part.id.as_str()).unwrap_or_default();
                set_port(edits, &file, port, None)?;
            }
        }
    }
    for h in design
        .harnesses
        .values()
        .filter(|h| h.nets.iter().any(|n| n.as_str() == net))
    {
        edits.edit(&layout::harness_file(h.id.as_str()), |doc| {
            if let Some(array) = doc.get_mut("nets").and_then(Item::as_array_mut) {
                array.retain(|v| v.as_str() != Some(net));
            }
            Ok(())
        })?;
    }
    Ok(())
}

/// カタログ品の名前を変える（ファイルの移動と、部品の `catalog`・`candidates`、ハーネスの `wire` の書き換え）。
pub fn rename_catalog(
    edits: &mut Edits,
    design: &Design,
    (from, from_file): (&str, &str),
    (to, to_file): (&str, &str),
) -> Result<(), StoreError> {
    edits.rename(from_file, to_file)?;
    for part in design.parts.values() {
        let uses = part.catalog.as_deref() == Some(from);
        let candidate = part.candidates.iter().any(|c| c == from);
        if !uses && !candidate {
            continue;
        }
        let file = layout::part_file(design, part.id.as_str()).unwrap_or_default();
        edits.edit(&file, |doc| {
            if uses {
                set(doc.as_table_mut(), "catalog", string(to));
            }
            if let Some(item) = doc.get_mut("candidates") {
                replace_in_array(item, from, to);
            }
            Ok(())
        })?;
    }
    for h in design
        .harnesses
        .values()
        .filter(|h| h.wire.as_deref() == Some(from))
    {
        set_strings(
            edits,
            &layout::harness_file(h.id.as_str()),
            vec![("wire", Some(to.to_string()))],
        )?;
    }
    Ok(())
}
