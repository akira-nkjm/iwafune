//! 定義（属性・種別）の Op の解釈。足す・置き換える・消す・名前を変える。組み込みの定義は書き換えも削除もできない。
//! 置き換えで既存の値が合わなくなる編集は、ほかの Op と同じく計画の最後の検査（新しい error）で拒まれる。

use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::builtin;
use fune_model::check::attribute_def_problems;
use fune_model::design::{AttributeDef, KindDef, RefSite, Referent};
use fune_model::id::validate_local_name;
use fune_store::Edits;
use fune_store::layout::{self, CatalogLocation, catalog_location};
use fune_store::write::{entities, refs};

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::CreateAttribute { def } => create_attribute(edits, design, def),
        Op::UpdateAttribute { def } => update_attribute(edits, design, def),
        Op::DeleteAttribute { id } => delete_attribute(edits, design, id),
        Op::CreateKind { def } => create_kind(edits, design, def),
        Op::UpdateKind { def } => update_kind(edits, design, def),
        Op::DeleteKind { id } => delete_kind(edits, design, id),
        Op::RenameAttribute { id, to } => rename_attribute(edits, design, id, to),
        Op::RenameKind { id, to } => rename_kind(edits, design, id, to),
        _ => Err(OpError::invalid("Not a definition op")),
    }
}

fn check_attribute(def: &AttributeDef) -> Result<(), OpError> {
    let problems = attribute_def_problems(def);
    if problems.is_empty() {
        return Ok(());
    }
    let text: Vec<String> = problems.into_iter().map(|(_, m)| m).collect();
    Err(OpError::invalid(text.join(" / ")))
}

fn create_attribute(edits: &mut Edits, design: &Design, def: &AttributeDef) -> Result<(), OpError> {
    let id = def.id.as_str();
    if design.attribute(id).is_some() {
        return Err(OpError::exists(format!("Attribute `{id}`")));
    }
    check_attribute(def)?;
    let file = layout::attribute_file(id);
    Ok(edits.create(&file, entities::new_attribute_text(&file, def)?)?)
}

/// プロジェクト独自の属性定義のファイル。組み込みなら書き換え・削除できない（`what` は動詞）。
fn own_attribute(design: &Design, id: &str, what: &str) -> Result<String, OpError> {
    if design.attributes.contains_key(id) {
        Ok(layout::attribute_file(id))
    } else if builtin::is_builtin_attribute(id) {
        Err(builtin_error("attribute", id, what))
    } else {
        Err(OpError::not_found(format!("Attribute `{id}`")))
    }
}

fn update_attribute(edits: &mut Edits, design: &Design, def: &AttributeDef) -> Result<(), OpError> {
    let file = own_attribute(design, def.id.as_str(), "edit")?;
    check_attribute(def)?;
    edits.remove(&file)?;
    Ok(edits.create(&file, entities::new_attribute_text(&file, def)?)?)
}

fn delete_attribute(edits: &mut Edits, design: &Design, id: &str) -> Result<(), OpError> {
    let file = own_attribute(design, id, "delete")?;
    refuse_if_used("Attribute", id, design.references(Referent::Attribute(id)))?;
    edits.remove(&file)?;
    Ok(())
}

fn create_kind(edits: &mut Edits, design: &Design, def: &KindDef) -> Result<(), OpError> {
    let id = def.id.as_str();
    validate_local_name(id).map_err(|e| OpError::invalid(e.to_string()))?;
    if design.kind(id).is_some() || builtin::is_builtin_kind(id) {
        return Err(OpError::exists(format!("Kind `{id}`")));
    }
    let file = layout::kind_file(id);
    Ok(edits.create(&file, entities::new_kind_text(&file, def)?)?)
}

/// プロジェクト独自の種別定義のファイル（組み込みなら誤り）。
fn own_kind(design: &Design, id: &str, what: &str) -> Result<String, OpError> {
    if design.kinds.contains_key(id) {
        Ok(layout::kind_file(id))
    } else if builtin::is_builtin_kind(id) {
        Err(builtin_error("kind", id, what))
    } else {
        Err(OpError::not_found(format!("Kind `{id}`")))
    }
}

fn update_kind(edits: &mut Edits, design: &Design, def: &KindDef) -> Result<(), OpError> {
    let file = own_kind(design, def.id.as_str(), "edit")?;
    edits.remove(&file)?;
    Ok(edits.create(&file, entities::new_kind_text(&file, def)?)?)
}

fn delete_kind(edits: &mut Edits, design: &Design, id: &str) -> Result<(), OpError> {
    let file = own_kind(design, id, "delete")?;
    refuse_if_used("Kind", id, design.references(Referent::Kind(id)))?;
    edits.remove(&file)?;
    Ok(())
}

/// 属性の名前を変える。参照の書き換えは store（`refs::rename_attribute`）。
fn rename_attribute(edits: &mut Edits, design: &Design, id: &str, to: &str) -> Result<(), OpError> {
    own_attribute(design, id, "rename")?;
    validate_local_name(to).map_err(|e| OpError::invalid(e.to_string()))?;
    if design.attribute(to).is_some() {
        return Err(OpError::exists(format!("Attribute `{to}`")));
    }
    refuse_shared_users(
        "attribute",
        id,
        design.references(Referent::Attribute(id)),
        design,
    )?;
    Ok(refs::rename_attribute(edits, design, id, to)?)
}

/// 種別の名前を変える。参照の書き換えは store（`refs::rename_kind`）。
fn rename_kind(edits: &mut Edits, design: &Design, id: &str, to: &str) -> Result<(), OpError> {
    own_kind(design, id, "rename")?;
    validate_local_name(to).map_err(|e| OpError::invalid(e.to_string()))?;
    if design.kind(to).is_some() || builtin::is_builtin_kind(to) {
        return Err(OpError::exists(format!("Kind `{to}`")));
    }
    refuse_shared_users("kind", id, design.references(Referent::Kind(id)), design)?;
    Ok(refs::rename_kind(edits, design, id, to)?)
}

/// 共有カタログの品（読み取り専用）が使っていれば、書き換えられないので名前の変更を拒む。
fn refuse_shared_users(
    what: &str,
    id: &str,
    sites: Vec<RefSite>,
    design: &Design,
) -> Result<(), OpError> {
    let shared: Vec<&str> = sites
        .iter()
        .filter_map(|site| match site {
            RefSite::CatalogKind { item } | RefSite::CatalogAttribute { item } => {
                Some(item.as_str())
            }
            _ => None,
        })
        .filter(|item| {
            !matches!(
                catalog_location(design, item),
                CatalogLocation::Local { .. }
            )
        })
        .collect();
    if shared.is_empty() {
        return Ok(());
    }
    Err(OpError::invalid(format!(
        "The {what} `{id}` is used by shared catalog items ({}), which are read-only, so it cannot be renamed. \
         Change the shared catalog first",
        shared.join(", ")
    )))
}

fn builtin_error(what: &str, id: &str, verb: &str) -> OpError {
    OpError::invalid(format!(
        "Cannot {verb} built-in {what} `{id}` (only project-defined ones can be changed)"
    ))
}

/// 使われていれば、使っているものを挙げて削除を拒む（カタログ品の削除と同じ文体）。
fn refuse_if_used(what: &str, id: &str, sites: Vec<RefSite>) -> Result<(), OpError> {
    let users: Vec<String> = sites.iter().filter_map(user_text).collect();
    if users.is_empty() {
        return Ok(());
    }
    Err(OpError::invalid(format!(
        "{what} `{id}` is used by {} and cannot be deleted. Re-point or delete the users first",
        users.join(", ")
    )))
}

/// 定義を使っているものの呼び名（定義の索引の参照元だけ）。
fn user_text(site: &RefSite) -> Option<String> {
    Some(match site {
        RefSite::PartKind { part } | RefSite::PartAttribute { part } => format!("part {part}"),
        RefSite::CatalogKind { item } | RefSite::CatalogAttribute { item } => {
            format!("catalog item {item}")
        }
        RefSite::KindExtends { kind } | RefSite::KindAttribute { kind } => format!("kind {kind}"),
        _ => return None,
    })
}
