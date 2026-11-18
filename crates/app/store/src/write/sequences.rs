//! モードの並び（`analysis/power.toml` の `[sequences.<名前>]`）の書き込みと、モードの時刻の指定（`when`・`start`）の書き込み。
//! 並びの読み方・選び方は model の `design::power_sequences`。従来の名前の無い `[[sequence]]` は `default` の並びとして読め、
//! `default` を書くと `[[sequence]]` は消える（`[sequences.default]` に移る）。

use super::Edits;
use super::doc::{self, set_opt, string};
use super::entities::{quantity, start_ref};
use crate::error::StoreError;
use fune_model::design::{DEFAULT_SEQUENCE, ModeSegment, ModeWhen, StartRef};
use toml_edit::{ArrayOfTables, Item, Table};

/// 並びの区間（`[[sequences.<名前>.segments]]`）。`=` は揃える。
fn segments_item(segments: &[ModeSegment]) -> Item {
    let mut array = ArrayOfTables::new();
    for s in segments {
        let mut t = Table::new();
        t.insert("mode", string(&s.mode));
        t.insert("duration", quantity(&s.duration));
        crate::profile::align_table(&mut t);
        array.push(t);
    }
    Item::ArrayOfTables(array)
}

/// 名前付きの並びを丸ごと書く（`[sequences.<name>]` の `anchor` と `segments`）。
/// `default` を書くときは従来の `[[sequence]]` を消す。
pub fn set_sequence(
    edits: &mut Edits,
    file: &str,
    name: &str,
    anchor: Option<&StartRef>,
    segments: &[ModeSegment],
) -> Result<(), StoreError> {
    edits.edit_or_create(file, |doc| {
        if name == DEFAULT_SEQUENCE {
            doc.as_table_mut().remove("sequence");
        }
        let table = doc::table_mut(doc, &["sequences", name])?;
        set_opt(table, "anchor", anchor.map(start_ref));
        table.remove("segments");
        table.insert("segments", segments_item(segments));
        Ok(())
    })
}

/// 名前付きの並び（`default` なら従来の `[[sequence]]` も）を消す。使う並び（`active_sequence`）が消す並びなら外す。
/// 消した並びが無ければ false。
pub fn remove_sequence(edits: &mut Edits, file: &str, name: &str) -> Result<bool, StoreError> {
    edits.edit(file, |doc| {
        let root = doc.as_table_mut();
        let mut removed = false;
        if let Some(t) = root.get_mut("sequences").and_then(Item::as_table_mut) {
            removed |= t.remove(name).is_some();
        }
        if name == DEFAULT_SEQUENCE {
            removed |= root.remove("sequence").is_some();
        }
        if root.get("active_sequence").and_then(Item::as_str) == Some(name) {
            root.remove("active_sequence");
        }
        doc::drop_if_empty(doc, &["sequences"]);
        Ok(removed)
    })
}

/// モードの時刻の指定（`when`・`start`）を書く（None で削除。`when = any` は何も書かないのと同じなので `any` も書かない）。
pub fn set_mode_timing(
    edits: &mut Edits,
    file: &str,
    when: Option<ModeWhen>,
    start: Option<&StartRef>,
) -> Result<(), StoreError> {
    edits.edit(file, |doc| {
        let when = when.filter(|w| *w != ModeWhen::Any);
        set_opt(doc.as_table_mut(), "when", when.map(|w| string(w.as_str())));
        set_opt(doc.as_table_mut(), "start", start.map(start_ref));
        doc::order_tables(doc, &["states", "eclipse"]);
        Ok(())
    })
}
