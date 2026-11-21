//! serde の形がそのままファイルの形のエンティティ（解析ケース・出典・ハーネス）の書き込み。
//!
//! 新しいファイルは serde の順（正規の順）で書き、既にあれば値の変わった行だけを書き換える。
//! 入れ子の値（`{ value, unit }` など）はインライン表にし、出典の付いた値（`length` など）は 1 行 1 要素の
//! 表にする（§10.2 の 3。形は `crate::profile::lay_out` と同じ規則の `spread_rows`）。

use super::Edits;
use super::doc;
use super::part::sync_table;
use crate::error::StoreError;
use crate::profile as format;
use serde::Serialize;
use toml_edit::{DocumentMut, Item, Table, Value};

/// 入れ子の表をインライン表にする（中の入れ子も含めて）。`sections` に挙げた最上位の表は見出しの表のまま残し、その中を
/// インライン表にする（`[target]` の下の `altitude = { value, unit }` など。§10.2 の 2 段まで）。
/// `sections` の名前の末尾が `/` なら、その表の子の表も見出しの表で残す（`link/` なら `[link.uhf]`。§10.2 の 3 段まで）。
fn inline_nested(table: &mut Table, sections: &[&str]) {
    let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
    for key in keys {
        if let Some(Item::Table(t)) = table.get_mut(&key) {
            let children = format!("{key}/");
            if sections.contains(&children.as_str()) {
                let names: Vec<String> = t.iter().map(|(k, _)| k.to_string()).collect();
                for name in names {
                    if let Some(Item::Table(c)) = t.get_mut(&name) {
                        inline_nested(c, &[]);
                    }
                }
                continue;
            }
            inline_nested(t, &[]);
            if sections.contains(&key.as_str()) {
                continue;
            }
            let inline = std::mem::take(t).into_inline_table();
            table.insert(&key, Item::Value(Value::InlineTable(inline)));
        }
    }
}

/// 値 → ファイルの文書（`id` はファイル名で決まるので書かない）。
pub fn record_doc<T: Serialize>(
    path: &str,
    value: &T,
    sections: &[&str],
) -> Result<DocumentMut, StoreError> {
    let text = toml::to_string(value).map_err(|e| StoreError::edit(path, e.to_string()))?;
    let mut doc = doc::parse(path, &text)?;
    doc.as_table_mut().remove("id");
    inline_nested(doc.as_table_mut(), sections);
    format::spread_rows(doc.as_table_mut(), 0);
    Ok(doc)
}

/// ファイルを値に合わせる（無ければ作る）。
pub fn write_record<T: Serialize>(
    edits: &mut Edits,
    path: &str,
    value: &T,
) -> Result<(), StoreError> {
    write_record_with_sections(edits, path, value, &[])
}

/// マヌーバのファイルで見出しの表にする最上位の表（docs/model.md の `maneuvers/`）。
pub const MANEUVER_SECTIONS: &[&str] = &["target", "constraints"];

/// 要求のファイルで見出しの表にする最上位の表（docs/model.md の `requirements/`）。
pub const REQUIREMENT_SECTIONS: &[&str] = &["constraint", "quantify", "verify"];

/// 地上局のファイルで見出しの表にする表（`[link.<帯域>]`。docs/model.md の `conditions/ground/`）。
pub const GROUND_SECTIONS: &[&str] = &["link/"];

/// ファイルを値に合わせる（無ければ作る）。`sections` の最上位の表は見出しの表で書く（マヌーバの `[target]`・`[constraints]`）。
pub fn write_record_with_sections<T: Serialize>(
    edits: &mut Edits,
    path: &str,
    value: &T,
    sections: &[&str],
) -> Result<(), StoreError> {
    let desired = record_doc(path, value, sections)?;
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

#[cfg(test)]
mod maneuver_tests {
    use super::*;
    use fune_model::design::{
        Maneuver, ManeuverCategory, ManeuverConstraints, ManeuverMethod, ManeuverTarget,
    };
    use fune_model::value::Quantity;

    /// `[target]`・`[constraints]` は見出しの表、中の量はインライン表。書いた形は `fune fmt` の正規形で、値を 1 つ変えればその行だけ変わる。
    #[test]
    fn maneuvers_keep_their_section_tables() {
        let mut m = Maneuver {
            id: "RAISE".into(),
            label: Some("軌道上昇".into()),
            category: ManeuverCategory::Insertion,
            method: ManeuverMethod::LowThrust,
            thruster: Some("EPT".into()),
            after: None,
            via_altitude: None,
            target: ManeuverTarget {
                altitude: Some(Quantity::new(600.0, "km")),
                ..ManeuverTarget::default()
            },
            constraints: ManeuverConstraints {
                sunlit_only: Some(true),
                mode: None,
                ..ManeuverConstraints::default()
            },
        };
        let path = "maneuvers/RAISE.toml";
        let mut edits = Edits::new(&crate::read::FileSet::new());
        write_record_with_sections(&mut edits, path, &m, MANEUVER_SECTIONS).unwrap();
        let text = edits.files().get(path).unwrap().to_string();
        assert_eq!(
            text,
            "label    = \"軌道上昇\"\ncategory = \"insertion\"\nmethod   = \"low_thrust\"\nthruster = \"EPT\"\n\n[target]\naltitude = { value = 600.0, unit = \"km\" }\n\n[constraints]\nsunlit_only = true\n"
        );
        assert_eq!(crate::format::format_text(path, &text).unwrap(), text);
        m.target.altitude = Some(Quantity::new(650.0, "km"));
        write_record_with_sections(&mut edits, path, &m, MANEUVER_SECTIONS).unwrap();
        let after = edits.files().get(path).unwrap();
        assert_eq!(after, text.replace("600.0", "650.0"));
    }
}
