//! カタログ品の Op の解釈。書き換えられるのはこのプロジェクトの `catalog/`（`[[catalog]] path = …`）だけで、
//! 共有カタログ（`git = …`）の品は読み取り専用（部品での上書きか、共有カタログへの PR を案内する）。

use super::attr::{AttrTarget, unset_attr, write_attr};
use super::subdir::{move_file, parse_dir};
use crate::error::OpError;
use crate::op::Op;
use fune_model::design::{CatalogItem, Maturity, split_catalog_ref};
use fune_model::id::{Id, clash, validate_entity_id};
use fune_model::resolve::Origin;
use fune_model::value::Authored;
use fune_model::{Design, resolve};
use fune_store::Edits;
use fune_store::layout::{CatalogLocation, catalog_item_dir, catalog_location, file_in_dir};
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
        Op::MoveCatalogItem { item, dir } => mv(edits, design, item, dir),
        _ => Err(OpError::invalid("Not a catalog item op")),
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
        .find(|c| c.local_dir().is_some())
        .map(|c| format!("{}/{text}", c.name))
        .ok_or_else(|| {
            OpError::invalid("fune.toml has no catalog for this project ([[catalog]] path = …)")
        })
}

/// 書き換えられるカタログ品の（ファイル, カタログのディレクトリ）。
fn writable(design: &Design, reference: &str) -> Result<(String, String), OpError> {
    match catalog_location(design, reference) {
        CatalogLocation::Local { file, base } => Ok((file, base)),
        CatalogLocation::Shared { source } => Err(OpError::invalid(format!(
            "`{reference}` is an item of shared catalog `{source}`, so it cannot be edited from this project. \
             Override the value on a part (fune set <part>.<attribute> …) or open a PR on the shared catalog repository"
        ))),
        CatalogLocation::Unknown => Err(OpError::invalid(format!(
            "`{reference}` must be `<catalog>/<id>`, and the catalog must be one in [[catalog]] of fune.toml"
        ))),
    }
}

/// 既にあり、書き換えられるカタログ品（参照, ファイル）。
fn existing(design: &Design, text: &str) -> Result<(String, String), OpError> {
    let reference = full_ref(design, text)?;
    // 共有カタログの品は読み込まれていなくても「読み取り専用」と答える（NotFound より先に）。
    let (file, _) = writable(design, &reference)?;
    if !design.catalog.contains_key(&reference) {
        return Err(OpError::not_found(format!("Catalog item `{reference}`")));
    }
    Ok((reference, file))
}

/// 新しいカタログ品の（参照, ファイル）。ファイルは `dir`（カタログのディレクトリからの名前の列）の下。
/// 参照として使えるか（ID の規則と、同じカタログの中での大文字小文字を無視した一意性）を確かめる。
fn new_ref(design: &Design, text: &str, dir: &[String]) -> Result<(String, String), OpError> {
    let reference = full_ref(design, text)?;
    let (source, id) = split_catalog_ref(&reference)
        .ok_or_else(|| OpError::invalid(format!("`{reference}` must be `<catalog>/<id>`")))?;
    validate_entity_id(id).map_err(|e| OpError::invalid(e.to_string()))?;
    let same_source = design
        .catalog
        .keys()
        .filter_map(|k| split_catalog_ref(k).filter(|(s, _)| *s == source))
        .map(|(_, i)| i);
    if clash(same_source, id).is_some() {
        return Err(OpError::exists(format!("Catalog item `{reference}`")));
    }
    let (_, base) = writable(design, &reference)?;
    let file = file_in_dir(&base, dir, id);
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
        dir,
    } = op
    else {
        return Err(OpError::invalid("Not a CreateCatalogItem op"));
    };
    let dirs = parse_dir(dir.as_deref())?;
    let (reference, file) = new_ref(design, id, &dirs)?;
    if design.kind(kind).is_none() {
        return Err(OpError::not_found(format!("Kind `{kind}`")));
    }
    let mut item = CatalogItem {
        id: reference,
        kind: Id::new(kind),
        label: label.clone(),
        label_en: label_en.clone(),
        manufacturer: None,
        part_number: None,
        maturity: *maturity,
        tags: Vec::new(),
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
        .ok_or_else(|| OpError::not_found(format!("Part `{part_id}`")))?;
    let resolved = resolve(design);
    let Some(rp) = resolved.parts.get(part_id) else {
        return Err(OpError::not_found(format!("Part `{part_id}`")));
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
            "Catalog item `{reference}` is referenced by {} and cannot be deleted. Re-point the references first",
            users.join(", ")
        )));
    }
    edits.remove(&file)?;
    Ok(())
}

fn rename(edits: &mut Edits, design: &Design, from: &str, to: &str) -> Result<(), OpError> {
    let (from_ref, from_file) = existing(design, from)?;
    // 同じカタログの中の名前の変更はディレクトリを変えない
    let same_dir = split_catalog_ref(&from_ref).map(|(s, _)| s)
        == split_catalog_ref(&full_ref(design, to)?).map(|(s, _)| s);
    let dirs = if same_dir {
        catalog_item_dir(design, &from_ref)
    } else {
        Vec::new()
    };
    let (to_ref, to_file) = new_ref(design, to, &dirs)?;
    Ok(refs::rename_catalog(
        edits,
        design,
        (&from_ref, &from_file),
        (&to_ref, &to_file),
    )?)
}

/// カタログ品を別のディレクトリへ移す（ID は変わらないので、参照は書き換えない）。
fn mv(edits: &mut Edits, design: &Design, item: &str, dir: &str) -> Result<(), OpError> {
    let (reference, file) = existing(design, item)?;
    let (_, base) = writable(design, &reference)?;
    let (_, id) = split_catalog_ref(&reference)
        .ok_or_else(|| OpError::invalid(format!("`{reference}` must be `<catalog>/<id>`")))?;
    move_file(edits, &format!("`{reference}`"), &file, &base, id, dir)
}
