//! 部品の置き先（グループ・組立品）の解釈。親の連鎖（`under`）つきの指定から `parts/` の下のディレクトリの列を作る。

use crate::error::OpError;
use fune_model::Design;
use fune_model::design::NodeRef;
use fune_model::id::{clash, validate_node_id};
use fune_store::layout::dirs_for_parent;

/// 新しいノード（部品・グループ）の ID として使えるか（規則と、大文字小文字を無視した一意性 `id::clash`）。
pub(super) fn new_node_id(design: &Design, id: &str) -> Result<(), OpError> {
    validate_node_id(id).map_err(|e| OpError::invalid(e.to_string()))?;
    let nodes = design
        .parts
        .keys()
        .map(|k| k.as_str())
        .chain(design.groups.keys().map(|k| k.as_str()));
    if clash(nodes, id).is_some() {
        return Err(OpError::exists(format!("ID `{id}`")));
    }
    Ok(())
}

/// 置き先の指定 → (置き先の既存のノードの連鎖, `parts/` の下のディレクトリの列)。
/// `under` が空なら従来どおり（`to` が既存ならそこ、無ければ最上位に作るグループ）。
/// `under` があれば `under + [to]` を外側から辿り、既存の段は親が合っているか確かめ、無い段は作る。
pub(super) fn target_dirs(
    design: &Design,
    to: Option<&str>,
    under: &[String],
) -> Result<(Vec<NodeRef>, Vec<String>), OpError> {
    let Some(to) = to else {
        if under.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        return Err(OpError::invalid(
            "A parent chain needs a target group; specify the group to move into",
        ));
    };
    if under.is_empty() {
        return match design.node(to) {
            Some(node) => {
                let dirs = dirs_for_parent(design, Some(&node));
                Ok((vec![node], dirs))
            }
            None => {
                new_node_id(design, to)?;
                Ok((Vec::new(), vec![to.to_string()]))
            }
        };
    }
    let chain: Vec<&str> = under
        .iter()
        .map(String::as_str)
        .chain(std::iter::once(to))
        .collect();
    walk_chain(design, &chain)
}

/// 連鎖（外側から）を辿る。先頭は既存のノード。既存の段は親が前の段であること、無い段は前の段がグループであること。
fn walk_chain(design: &Design, chain: &[&str]) -> Result<(Vec<NodeRef>, Vec<String>), OpError> {
    let mut seen: Vec<String> = Vec::new();
    let mut existing: Vec<NodeRef> = Vec::new();
    let mut dirs: Vec<String> = Vec::new();
    for (i, seg) in chain.iter().enumerate() {
        let lower = seg.to_lowercase();
        if seen.contains(&lower) {
            return Err(OpError::invalid(format!(
                "`{seg}` appears twice in the group path"
            )));
        }
        seen.push(lower);
        match design.node(seg) {
            Some(node) => {
                match existing.last() {
                    // 前の段が新しく作る段なら、既存のノードはその下にはいない
                    _ if i > 0 && existing.len() < i => {
                        return Err(OpError::exists(format!(
                            "`{seg}` already exists, but not under `{}`",
                            chain[i - 1]
                        )));
                    }
                    None => dirs = dirs_for_parent(design, Some(&node)),
                    Some(prev) => {
                        if design.parent_of(&node) != Some(prev) {
                            return Err(OpError::exists(format!(
                                "`{seg}` already exists, but not under `{}`",
                                prev.id()
                            )));
                        }
                        dirs.push(seg.to_string());
                    }
                }
                existing.push(node);
            }
            None => {
                match existing.last() {
                    Some(NodeRef::Group(_)) if existing.len() == i => {}
                    Some(NodeRef::Part(p)) if existing.len() == i => {
                        return Err(OpError::invalid(format!(
                            "Cannot create group `{seg}` under part `{}`; groups go under a subsystem or another group",
                            p.as_str()
                        )));
                    }
                    _ if i == 0 => {
                        return Err(OpError::not_found(format!(
                            "`{seg}` (the first segment of a group path must be an existing subsystem or group)"
                        )));
                    }
                    _ => {}
                }
                new_node_id(design, seg)?;
                dirs.push(seg.to_string());
            }
        }
    }
    Ok((existing, dirs))
}
