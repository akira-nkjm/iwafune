//! 属性を `[attr.<名前>]` の表で書いたファイルに属性を足したときの形と位置（docs/model.md §10.2 の 3）。
//! 値の行のある `[attr]` は下の `[attr.<名前>]` より前、足した属性は ID の昇順（新しいファイルと同じ順）の直前の属性の後ろ。
//! 手で後ろに書いた `[attr]` は `fune fmt` が前へ直し、`--check` に引っかかる。

use fune_model::design::Part;
use fune_model::value::{Authored, Provenance, Scalar, Value};
use fune_store::FileSet;
use fune_store::format::format_text;
use fune_store::write::Edits;
use fune_store::write::part::{new_part_text, set_attr, unset_attr};

const FILE: &str = "parts/P.toml";

const TABLES: &str = "kind = \"x\"\n\n[attr.a_mass]\nvalue       = 1.0\nunit        = \"kg\"\nfrom.source = \"s\"\n\n[attr.c_size]\nvalue       = 3.0\nunit        = \"mm\"\nfrom.source = \"s\"\n";

fn quantity(v: f64, unit: &str, source: Option<&str>) -> Authored<Value> {
    Authored {
        value: Value::Scalar(Scalar::new(v, unit)),
        from: source.and_then(|s| Provenance::from_parts(Some(s.into()), None, None)),
    }
}

fn edit(text: &str, f: impl FnOnce(&mut Edits)) -> String {
    let mut set = FileSet::new();
    set.insert(FILE, text);
    let mut edits = Edits::new(&set);
    f(&mut edits);
    edits.files().get(FILE).unwrap().to_string()
}

#[test]
fn a_value_without_source_goes_before_the_attribute_tables() {
    let out = edit(TABLES, |e| {
        set_attr(e, FILE, "b_isp", &quantity(1200.0, "s", None)).unwrap();
    });
    let want = "kind = \"x\"\n\n[attr]\nb_isp = { value = 1200.0, unit = \"s\" }\n\n[attr.a_mass]\nvalue       = 1.0\nunit        = \"kg\"\nfrom.source = \"s\"\n\n[attr.c_size]\nvalue       = 3.0\nunit        = \"mm\"\nfrom.source = \"s\"\n";
    assert_eq!(out, want);
    assert_eq!(format_text(FILE, &out).unwrap(), out);
}

#[test]
fn a_value_with_source_lands_between_its_neighbours() {
    let out = edit(TABLES, |e| {
        set_attr(e, FILE, "b_isp", &quantity(1200.0, "s", Some("s"))).unwrap();
    });
    let a = out.find("[attr.a_mass]").unwrap();
    let b = out.find("[attr.b_isp]").unwrap();
    let c = out.find("[attr.c_size]").unwrap();
    assert!(a < b && b < c, "{out}");
    assert!(
        out.contains(
            "[attr.b_isp]\nvalue       = 1200.0\nunit        = \"s\"\nfrom.source = \"s\"\n"
        ),
        "{out}"
    );
    assert!(!out.contains("[attr]"), "{out}");
    assert_eq!(format_text(FILE, &out).unwrap(), out);
}

#[test]
fn first_and_last_positions_follow_the_sorted_order() {
    let out = edit(TABLES, |e| {
        set_attr(e, FILE, "0_first", &quantity(1.0, "s", Some("s"))).unwrap();
        set_attr(e, FILE, "z_last", &quantity(1.0, "s", Some("s"))).unwrap();
    });
    let pos = |k: &str| out.find(&format!("[attr.{k}]")).unwrap();
    assert!(
        pos("0_first") < pos("a_mass") && pos("c_size") < pos("z_last"),
        "{out}"
    );
}

/// 新しく作ったファイルの順で書かれたファイルでは、`unset` → `set`（同じ値・同じ出典）で元のバイト列に戻る。
#[test]
fn unset_then_set_round_trips_in_a_canonically_ordered_file() {
    for source in [None, Some("s")] {
        let mut part = Part::new("P");
        for (k, v) in [("a_mass", 1.0), ("b_isp", 1200.0), ("c_size", 3.0)] {
            part.attr.insert(k.into(), quantity(v, "x", source));
        }
        let original = new_part_text(&part);
        let back = edit(&original, |e| {
            assert!(unset_attr(e, FILE, "b_isp", None).unwrap());
            set_attr(e, FILE, "b_isp", &quantity(1200.0, "x", source)).unwrap();
        });
        assert_eq!(back, original, "source {source:?}");
    }
}

/// 値の行のある `[attr]` が `[attr.<名前>]` より後ろにある混ざった形は、fmt が前へ直す。
#[test]
fn fmt_moves_a_late_attr_header_before_its_tables() {
    let mixed = format!("{TABLES}\n[attr]\nb_isp = {{ value = 1200.0, unit = \"s\" }}\n");
    let out = format_text(FILE, &mixed).unwrap();
    assert_ne!(out, mixed);
    assert!(
        out.find("[attr]").unwrap() < out.find("[attr.a_mass]").unwrap(),
        "{out}"
    );
    assert_eq!(format_text(FILE, &out).unwrap(), out);
}

/// 状態ごとの値の 1 セルで新しい属性の表を作るときも、正規の位置に入る。
#[test]
fn a_new_per_state_attribute_table_lands_in_sorted_position() {
    let cell = Authored {
        value: Scalar::new(2.0, "W"),
        from: Provenance::from_parts(Some("s".into()), None, None),
    };
    let out = edit(TABLES, |e| {
        fune_store::write::part::set_attr_state(e, FILE, "b_power", "on", &cell).unwrap();
    });
    let pos = |k: &str| out.find(&format!("[attr.{k}]")).unwrap();
    assert!(
        pos("a_mass") < pos("b_power") && pos("b_power") < pos("c_size"),
        "{out}"
    );
    assert_eq!(format_text(FILE, &out).unwrap(), out);
}
