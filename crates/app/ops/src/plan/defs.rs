//! 定義（属性・種別）の Op の解釈。足す・置き換える・消す。組み込みの定義は書き換えも削除もできない。
//! 置き換えで既存の値が合わなくなる編集は、ほかの Op と同じく計画の最後の検査（新しい error）で拒まれる。

use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::builtin;
use fune_model::check::attribute_def_problems;
use fune_model::design::{AttributeDef, KindDef, RefSite, Referent};
use fune_model::id::validate_local_name;
use fune_store::Edits;
use fune_store::layout;
use fune_store::write::entities;

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::CreateAttribute { def } => create_attribute(edits, design, def),
        Op::UpdateAttribute { def } => update_attribute(edits, design, def),
        Op::DeleteAttribute { id } => delete_attribute(edits, design, id),
        Op::CreateKind { def } => create_kind(edits, design, def),
        Op::UpdateKind { def } => update_kind(edits, design, def),
        Op::DeleteKind { id } => delete_kind(edits, design, id),
        _ => Err(OpError::invalid("定義の Op ではありません")),
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
        return Err(OpError::exists(format!("属性 `{id}`")));
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
        Err(builtin_error("属性", id, what))
    } else {
        Err(OpError::not_found(format!("属性 `{id}`")))
    }
}

fn update_attribute(edits: &mut Edits, design: &Design, def: &AttributeDef) -> Result<(), OpError> {
    let file = own_attribute(design, def.id.as_str(), "書き換え")?;
    check_attribute(def)?;
    edits.remove(&file)?;
    Ok(edits.create(&file, entities::new_attribute_text(&file, def)?)?)
}

fn delete_attribute(edits: &mut Edits, design: &Design, id: &str) -> Result<(), OpError> {
    let file = own_attribute(design, id, "削除")?;
    refuse_if_used("属性", id, design.references(Referent::Attribute(id)))?;
    edits.remove(&file)?;
    Ok(())
}

fn create_kind(edits: &mut Edits, design: &Design, def: &KindDef) -> Result<(), OpError> {
    let id = def.id.as_str();
    validate_local_name(id).map_err(|e| OpError::invalid(e.to_string()))?;
    if design.kind(id).is_some() || builtin::is_builtin_kind(id) {
        return Err(OpError::exists(format!("種別 `{id}`")));
    }
    let file = layout::kind_file(id);
    Ok(edits.create(&file, entities::new_kind_text(&file, def)?)?)
}

/// プロジェクト独自の種別定義のファイル（組み込みなら誤り）。
fn own_kind(design: &Design, id: &str, what: &str) -> Result<String, OpError> {
    if design.kinds.contains_key(id) {
        Ok(layout::kind_file(id))
    } else if builtin::is_builtin_kind(id) {
        Err(builtin_error("種別", id, what))
    } else {
        Err(OpError::not_found(format!("種別 `{id}`")))
    }
}

fn update_kind(edits: &mut Edits, design: &Design, def: &KindDef) -> Result<(), OpError> {
    let file = own_kind(design, def.id.as_str(), "書き換え")?;
    edits.remove(&file)?;
    Ok(edits.create(&file, entities::new_kind_text(&file, def)?)?)
}

fn delete_kind(edits: &mut Edits, design: &Design, id: &str) -> Result<(), OpError> {
    let file = own_kind(design, id, "削除")?;
    refuse_if_used("種別", id, design.references(Referent::Kind(id)))?;
    edits.remove(&file)?;
    Ok(())
}

fn builtin_error(what: &str, id: &str, verb: &str) -> OpError {
    OpError::invalid(format!(
        "{what} `{id}` は組み込みの定義なので{verb}できません（プロジェクト独自の定義だけが対象です）"
    ))
}

/// 使われていれば、使っているものを挙げて削除を拒む（カタログ品の削除と同じ文体）。
fn refuse_if_used(what: &str, id: &str, sites: Vec<RefSite>) -> Result<(), OpError> {
    let users: Vec<String> = sites.iter().filter_map(user_text).collect();
    if users.is_empty() {
        return Ok(());
    }
    Err(OpError::invalid(format!(
        "{what} `{id}` は {} から使われているので消せません。先に使っている側を付け替えるか消してください",
        users.join(", ")
    )))
}

/// 定義を使っているものの呼び名（定義の索引の参照元だけ）。
fn user_text(site: &RefSite) -> Option<String> {
    Some(match site {
        RefSite::PartKind { part } | RefSite::PartAttribute { part } => format!("部品 {part}"),
        RefSite::CatalogKind { item } | RefSite::CatalogAttribute { item } => {
            format!("カタログ品 {item}")
        }
        RefSite::KindExtends { kind } | RefSite::KindAttribute { kind } => format!("種別 {kind}"),
        _ => return None,
    })
}
