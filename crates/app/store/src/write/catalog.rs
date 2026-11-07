//! カタログ品のファイル（`catalog/<ID>.toml`）の正規形とフィールドの書き込み。属性の値の行は [`super::part`] と同じ関数で書く。

use super::Edits;
use super::doc::{set_opt, string};
use crate::codec::encode_value;
use crate::error::StoreError;
use crate::profile as format;
use fune_model::design::{CatalogField, CatalogItem};
use toml_edit::{DocumentMut, Item, Table};

/// 新しいカタログ品のファイル（正規形: 見出しのキー → `[attr]` → `[ports]`）。
pub fn new_catalog_text(item: &CatalogItem) -> String {
    let mut doc = DocumentMut::new();
    let top = doc.as_table_mut();
    top.insert("kind", string(item.kind.as_str()));
    let fields = [
        ("label", item.label.as_deref()),
        ("label_en", item.label_en.as_deref()),
        ("manufacturer", item.manufacturer.as_deref()),
        ("part_number", item.part_number.as_deref()),
        ("maturity", item.maturity.map(|m| m.as_str())),
        ("datasheet", item.datasheet.as_deref()),
        ("source", item.source.as_deref()),
    ];
    for (key, value) in fields {
        if let Some(v) = value {
            top.insert(key, string(v));
        }
    }
    if !item.attr.is_empty() {
        let mut attr = Table::new();
        for (id, value) in &item.attr {
            attr.insert(id.as_str(), encode_value(value));
        }
        top.insert("attr", Item::Table(attr));
    }
    if !item.ports.is_empty() {
        let mut ports = Table::new();
        for (name, kind) in &item.ports {
            ports.insert(name, string(kind.as_str()));
        }
        top.insert("ports", Item::Table(ports));
    }
    format::finish(doc)
}

/// 見出しのフィールドを書く（None なら消す）。
pub fn set_catalog_field(
    edits: &mut Edits,
    file: &str,
    field: &CatalogField,
) -> Result<(), StoreError> {
    let item = match field {
        CatalogField::Kind(k) => Some(string(k)),
        CatalogField::Maturity(m) => m.map(|m| string(m.as_str())),
        CatalogField::Label(v)
        | CatalogField::LabelEn(v)
        | CatalogField::Manufacturer(v)
        | CatalogField::PartNumber(v)
        | CatalogField::Datasheet(v)
        | CatalogField::Source(v) => v.as_deref().map(string),
    };
    edits.edit(file, |doc| {
        set_opt(doc.as_table_mut(), field.name(), item);
        Ok(())
    })
}
