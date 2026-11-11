//! カタログ品の Op の解釈。書き換えられるのはこのプロジェクトの `catalog/`（`[[catalog]] path = …`）だけで、
//! 共有カタログ（`git = …`）の品は読み取り専用（部品での上書きか、共有カタログへの PR を案内する）。

use super::attr::{AttrTarget, unset_attr, write_attr};
use crate::error::OpError;
use crate::op::Op;
use fune_model::design::{CatalogItem, Maturity, split_catalog_ref};
use fune_model::id::{Id, clash, validate_entity_id};
use fune_model::resolve::Origin;
use fune_model::value::Authored;
use fune_model::{Design, resolve};
use fune_store::Edits;
use fune_store::layout::{CatalogLocation, catalog_location, local_catalog_dir};
use fune_store::write::{catalog as wcatalog, refs};
use std::collections::BTreeMap;

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::CreateCatalogItem { .. } => create(edits, design, op),
        Op::SetCatalogValue {
            item,
            attr,
            state,
            value,
            from,
        } => {
            let (reference, file) = existing(design, item)?;
            let current = &design.catalog[&reference];
            let target = AttrTarget {
                file: &file,
                current: current.attr.get(attr.as_str()).map(|a| &a.value),
                states: design.kind_states(current.kind.as_str()),
                owner: &reference,
            };
            write_attr(
                edits,
                design,
                target,
                attr,
                state.as_deref(),
                value,
                from.clone(),
            )
        }
        Op::UnsetCatalogValue { item, attr, state } => {
            let (reference, file) = existing(design, item)?;
            unset_attr(edits, &file, &reference, attr, state.as_deref())
        }
        Op::SetCatalogField { item, field } => {
            let (_, file) = existing(design, item)?;
            Ok(wcatalog::set_catalog_field(edits, &file, field)?)
        }
        Op::RemoveCatalogItem { id } => remove(edits, design, id),
        Op::RenameCatalogItem { from, to } => rename(edits, design, from, to),
        _ => Err(OpError::invalid("カタログ品の Op ではありません")),
    }
}

/// `rw_30mNms` のようにカタログ名を省いた参照には、最初の `path` のカタログの名前を補う。
fn full_ref(design: &Design, text: &str) -> Result<String, OpError> {
    if text.contains('/') {
        return Ok(text.to_string());
    }
    design
        .project
        .catalogs
        .iter()
        .find(|c| local_catalog_dir(c).is_some())
        .map(|c| format!("{}/{text}", c.name))
        .ok_or_else(|| {
            OpError::invalid(
                "fune.toml にこのプロジェクトのカタログ（[[catalog]] path = …）がありません",
            )
        })
}

/// 書き換えられるカタログ品のファイル。
fn writable_file(design: &Design, reference: &str) -> Result<String, OpError> {
    match catalog_location(design, reference) {
        CatalogLocation::Local { file } => Ok(file),
        CatalogLocation::Shared { source } => Err(OpError::invalid(format!(
            "`{reference}` は共有カタログ `{source}` の品なので、このプロジェクトからは書き換えられません。\
             部品で値を上書きする（fune set <部品>.<属性> …）か、共有カタログのリポジトリに PR を出してください"
        ))),
        CatalogLocation::Unknown => Err(OpError::invalid(format!(
            "`{reference}` は `<カタログ名>/<ID>` の形で、カタログ名は fune.toml の [[catalog]] にあるものにします"
        ))),
    }
}

/// 既にあり、書き換えられるカタログ品（参照, ファイル）。
fn existing(design: &Design, text: &str) -> Result<(String, String), OpError> {
    let reference = full_ref(design, text)?;
    // 共有カタログの品は読み込まれていなくても「読み取り専用」と答える（NotFound より先に）。
    let file = writable_file(design, &reference)?;
    if !design.catalog.contains_key(&reference) {
        return Err(OpError::not_found(format!("カタログ品 `{reference}`")));
    }
    Ok((reference, file))
}

/// 新しいカタログ品の参照として使えるか（ID の規則と、同じカタログの中での大文字小文字を無視した一意性）。
fn new_ref(design: &Design, text: &str) -> Result<(String, String), OpError> {
    let reference = full_ref(design, text)?;
    let (source, id) = split_catalog_ref(&reference).ok_or_else(|| {
        OpError::invalid(format!("`{reference}` は `<カタログ名>/<ID>` の形にします"))
    })?;
    validate_entity_id(id).map_err(|e| OpError::invalid(e.to_string()))?;
    let same_source = design
        .catalog
        .keys()
        .filter_map(|k| split_catalog_ref(k).filter(|(s, _)| *s == source))
        .map(|(_, i)| i);
    if clash(same_source, id).is_some() {
        return Err(OpError::exists(format!("カタログ品 `{reference}`")));
    }
    let file = writable_file(design, &reference)?;
    Ok((reference, file))
}

fn create(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    let Op::CreateCatalogItem {
        id,
        kind,
        label,
        label_en,
        maturity,
        copy_from_part,
    } = op
    else {
        return Err(OpError::invalid("CreateCatalogItem ではありません"));
    };
    let (reference, file) = new_ref(design, id)?;
    if design.kind(kind).is_none() {
        return Err(OpError::not_found(format!("種別 `{kind}`")));
    }
    let mut item = CatalogItem {
        id: reference,
        kind: Id::new(kind),
        label: label.clone(),
        label_en: label_en.clone(),
        manufacturer: None,
        part_number: None,
        maturity: *maturity,
        datasheet: None,
        source: None,
        attr: BTreeMap::new(),
        ports: BTreeMap::new(),
    };
    if let Some(part) = copy_from_part {
        promote(design, part, &mut item)?;
    }
    Ok(edits.create(&file, wcatalog::new_catalog_text(&item))?)
}

/// 部品の今の値（カタログ値＋上書き。既定値で埋めたものは除く）とポートの雛形をカタログ品に写す。
fn promote(design: &Design, part_id: &str, item: &mut CatalogItem) -> Result<(), OpError> {
    let part = design
        .parts
        .get(part_id)
        .ok_or_else(|| OpError::not_found(format!("部品 `{part_id}`")))?;
    let resolved = resolve(design);
    let Some(rp) = resolved.parts.get(part_id) else {
        return Err(OpError::not_found(format!("部品 `{part_id}`")));
    };
    for (attr, value) in &rp.attr {
        let written = matches!(value.origin, Origin::Catalog | Origin::Override);
        if let (true, Some(v)) = (written, &value.authored) {
            let authored = Authored {
                value: v.clone(),
                from: value.from.clone(),
            };
            item.attr.insert(Id::new(attr), authored);
        }
    }
    item.ports = rp
        .ports
        .iter()
        .filter_map(|(name, port)| port.kind.map(|k| (name.clone(), k)))
        .collect();
    let old = part.catalog.as_ref().and_then(|c| design.catalog.get(c));
    item.source = old.and_then(|c| c.source.clone());
    item.manufacturer = old.and_then(|c| c.manufacturer.clone());
    item.part_number = old.and_then(|c| c.part_number.clone());
    item.datasheet = old.and_then(|c| c.datasheet.clone());
    item.label = item
        .label
        .clone()
        .or_else(|| part.label.clone())
        .or_else(|| old.and_then(|c| c.label.clone()));
    item.maturity = item.maturity.or(rp.maturity).or(Some(Maturity::New));
    Ok(())
}

fn remove(edits: &mut Edits, design: &Design, id: &str) -> Result<(), OpError> {
    let (reference, file) = existing(design, id)?;
    let users = design.catalog_users(&reference);
    if !users.is_empty() {
        return Err(OpError::invalid(format!(
            "カタログ品 `{reference}` は {} から参照されているので消せません。先に参照を付け替えてください",
            users.join(", ")
        )));
    }
    edits.remove(&file)?;
    Ok(())
}

fn rename(edits: &mut Edits, design: &Design, from: &str, to: &str) -> Result<(), OpError> {
    let (from_ref, from_file) = existing(design, from)?;
    let (to_ref, to_file) = new_ref(design, to)?;
    Ok(refs::rename_catalog(
        edits,
        design,
        (&from_ref, &from_file),
        (&to_ref, &to_file),
    )?)
}
