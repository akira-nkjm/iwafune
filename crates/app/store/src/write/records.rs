//! serde の形がそのままファイルの形のエンティティ（解析ケース・出典・ハーネス）の書き込み。
//!
//! 新しいファイルは serde の順（正規の順）で書き、既にあれば値の変わった行だけを書き換える。
//! 入れ子の値（`{ value, unit, from }` など）はインライン表にする（§10.2 の 3）。

use super::Edits;
use super::doc;
use super::part::sync_table;
use crate::error::StoreError;
use crate::profile as format;
use serde::Serialize;
use toml_edit::{DocumentMut, Item, Table, Value};

/// 入れ子の表をインライン表にする（中の入れ子も含めて）。
fn inline_nested(table: &mut Table) {
    let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
    for key in keys {
        if let Some(Item::Table(t)) = table.get_mut(&key) {
            inline_nested(t);
            let inline = std::mem::take(t).into_inline_table();
            table.insert(&key, Item::Value(Value::InlineTable(inline)));
        }
    }
}

/// 値 → ファイルの文書（`id` はファイル名で決まるので書かない）。
pub fn record_doc<T: Serialize>(path: &str, value: &T) -> Result<DocumentMut, StoreError> {
    let text = toml::to_string(value).map_err(|e| StoreError::edit(path, e.to_string()))?;
    let mut doc = doc::parse(path, &text)?;
    doc.as_table_mut().remove("id");
    inline_nested(doc.as_table_mut());
    Ok(doc)
}

/// ファイルを値に合わせる（無ければ作る）。
pub fn write_record<T: Serialize>(
    edits: &mut Edits,
    path: &str,
    value: &T,
) -> Result<(), StoreError> {
    let desired = record_doc(path, value)?;
    if edits.exists(path) {
        edits.edit(path, |doc| {
            sync_table(doc.as_table_mut(), desired.as_table().clone());
            Ok(())
        })
    } else {
        edits.create(path, format::finish(desired))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fune_model::design::Harness;
    use fune_model::path::PortRef;
    use fune_model::value::Quantity;

    #[test]
    fn records_use_inline_quantities_and_minimal_diffs() {
        let mut h = Harness {
            id: "H".into(),
            label: None,
            from: PortRef::new("A", "P"),
            to: PortRef::new("B", "P"),
            nets: vec!["N".into()],
            wire: None,
            cores: Some(2),
            length: Some(Quantity::new(0.35, "m")),
        };
        let mut edits = Edits::new(&crate::read::FileSet::new());
        write_record(&mut edits, "harness/H.toml", &h).unwrap();
        let text = edits.files().get("harness/H.toml").unwrap().to_string();
        assert_eq!(
            text,
            "from   = \"A.P\"\nto     = \"B.P\"\nnets   = [\"N\"]\ncores  = 2\nlength = { value = 0.35, unit = \"m\" }\n"
        );
        h.length = Some(Quantity::new(0.4, "m"));
        write_record(&mut edits, "harness/H.toml", &h).unwrap();
        let after = edits.files().get("harness/H.toml").unwrap();
        assert_eq!(after, text.replace("0.35", "0.4"));
    }
}
