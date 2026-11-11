//! カタログ品のファイル（`catalog/<ID>.toml`）の正規形とフィールドの書き込み。属性の値の行は [`super::part`] と同じ関数で書く。

use super::Edits;
use super::doc::{set_opt, string};
use crate::codec::encode_value;
use crate::error::StoreError;
use crate::profile as format;
use fune_model::design::{CatalogField, CatalogFieldName, CatalogItem};
use toml_edit::{DocumentMut, Item, Table};

/// 型付きのフィールド → ファイルの値（None なら書かない）。新しいファイルの見出しと 1 フィールドの書き込みで共通。
fn field_item(field: &CatalogField) -> Option<Item> {
    match field {
        CatalogField::Kind(k) => Some(string(k)),
        CatalogField::Maturity(m) => m.map(|m| string(m.as_str())),
        CatalogField::Label(v)
        | CatalogField::LabelEn(v)
        | CatalogField::Manufacturer(v)
        | CatalogField::PartNumber(v)
        | CatalogField::Datasheet(v)
        | CatalogField::Source(v) => v.as_deref().map(string),
    }
}

/// 新しいカタログ品のファイル（正規形: 見出しのキー（`CatalogFieldName` の順）→ `[attr]` → `[ports]`）。
pub fn new_catalog_text(item: &CatalogItem) -> String {
    let mut doc = DocumentMut::new();
    let top = doc.as_table_mut();
    for &key in CatalogFieldName::ALL {
        if let Some(value) = field_item(&CatalogField::of(item, key)) {
            top.insert(key.as_str(), value);
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
    let item = field_item(field);
    edits.edit(file, |doc| {
        set_opt(doc.as_table_mut(), field.name(), item);
        Ok(())
    })
}
