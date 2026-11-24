//! 承知済みの組（`analysis/structure.toml` の `[[acknowledged]]`）の書き込み。足す（同じ組・同じ形態なら理由を書き換える）・取り消す・
//! 部品と形態の名前の変更で書き換える。一覧の意味・照合の規則は model の `design::acknowledged`。

use super::Edits;
use super::doc::{set, set_opt, string};
use crate::error::StoreError;
use fune_model::design::{AcknowledgedFinding, FindingKind};
use toml_edit::{ArrayOfTables, Item, Table};

/// 表（`[[acknowledged]]` の 1 つ）から承知を読む（読めない行は None）。
fn read(t: &Table) -> Option<AcknowledgedFinding> {
    let text = |key: &str| t.get(key).and_then(Item::as_str).map(str::to_owned);
    Some(AcknowledgedFinding {
        finding: text("finding")?.parse::<FindingKind>().ok()?,
        first: text("first")?,
        second: text("second")?,
        configuration: text("configuration"),
        reason: text("reason").unwrap_or_default(),
    })
}

/// 承知 1 つの表（キーは `=` を揃える）。
fn table_of(a: &AcknowledgedFinding) -> Table {
    let mut t = Table::new();
    t.insert("finding", string(a.finding.as_str()));
    t.insert("first", string(&a.first));
    t.insert("second", string(&a.second));
    if let Some(c) = &a.configuration {
        t.insert("configuration", string(c));
    }
    t.insert("reason", string(&a.reason));
    crate::profile::align_table(&mut t);
    t
}

/// 承知を足す。同じ組・同じ形態の承知があれば、理由を書き換える。
pub fn acknowledge(
    edits: &mut Edits,
    file: &str,
    entry: &AcknowledgedFinding,
) -> Result<(), StoreError> {
    edits.edit_or_create(file, |doc| {
        let root = doc.as_table_mut();
        let array = root
            .entry("acknowledged")
            .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()))
            .as_array_of_tables_mut()
            .ok_or_else(|| StoreError::Missing {
                path: format!("{file}: `acknowledged` is not an array of tables"),
            })?;
        let same = |t: &Table| read(t).is_some_and(|e| e.same_target(entry));
        let found = array.iter().position(same);
        if let Some(index) = found {
            if let Some(t) = array.get_mut(index) {
                set(t, "reason", string(&entry.reason));
            }
        } else {
            array.push(table_of(entry));
        }
        Ok(())
    })
}

/// 承知を取り消す（同じ組・同じ形態の承知を消す）。消したら true。空になった一覧は書かない。
pub fn unacknowledge(
    edits: &mut Edits,
    file: &str,
    target: &AcknowledgedFinding,
) -> Result<bool, StoreError> {
    edits.edit(file, |doc| {
        let root = doc.as_table_mut();
        let Some(array) = root
            .get_mut("acknowledged")
            .and_then(Item::as_array_of_tables_mut)
        else {
            return Ok(false);
        };
        let Some(index) = array
            .iter()
            .position(|t| read(t).is_some_and(|e| e.same_target(target)))
        else {
            return Ok(false);
        };
        array.remove(index);
        if array.is_empty() {
            root.remove("acknowledged");
        }
        Ok(true)
    })
}

/// `index` 番目の承知の部品（`first`・`second`）の名前を変える。
pub fn rename_part(
    edits: &mut Edits,
    file: &str,
    index: usize,
    (from, to): (&str, &str),
) -> Result<(), StoreError> {
    edits.edit(file, |doc| {
        if let Some(t) = entry_mut(doc, index) {
            for key in ["first", "second"] {
                if t.get(key).and_then(Item::as_str) == Some(from) {
                    set(t, key, string(to));
                }
            }
        }
        Ok(())
    })
}

/// `index` 番目の承知の形態の名前を変える。
pub fn rename_configuration(
    edits: &mut Edits,
    file: &str,
    index: usize,
    (from, to): (&str, &str),
) -> Result<(), StoreError> {
    edits.edit(file, |doc| {
        if let Some(t) = entry_mut(doc, index)
            && t.get("configuration").and_then(Item::as_str) == Some(from)
        {
            set_opt(t, "configuration", Some(string(to)));
        }
        Ok(())
    })
}

fn entry_mut(doc: &mut toml_edit::DocumentMut, index: usize) -> Option<&mut Table> {
    doc.get_mut("acknowledged")
        .and_then(Item::as_array_of_tables_mut)
        .and_then(|a| a.get_mut(index))
}
