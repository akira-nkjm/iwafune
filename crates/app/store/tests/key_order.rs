//! レコード（形態・ハーネスなど）の書き込みで、既にあるファイルに足したキーの位置。
//! 足したキーは、新しく作ったファイルと同じ正規の順の位置に入る（編集の履歴で同じレコードの形が変わらない）。

use fune_model::design::{Configuration, Harness, Pose};
use fune_model::path::PortRef;
use fune_model::value::{Quantity, Vector3};
use fune_store::FileSet;
use fune_store::write::Edits;
use fune_store::write::records::{CONFIGURATION_SECTIONS, write_record_with_sections};
use serde::Serialize;

/// 値を書く（`existing` があればそのファイルへの編集、無ければ新しいファイル）。書いたあとの文字列を返す。
fn written<T: Serialize>(existing: Option<&str>, value: &T, sections: &[&str]) -> String {
    let mut set = FileSet::new();
    if let Some(text) = existing {
        set.insert("x/X.toml", text);
    }
    let mut edits = Edits::new(&set);
    write_record_with_sections(&mut edits, "x/X.toml", value, sections).unwrap();
    edits.files().get("x/X.toml").unwrap().to_string()
}

fn configuration(
    label: Option<&str>,
    label_en: Option<&str>,
    description: Option<&str>,
) -> Configuration {
    let mut c = Configuration {
        label: label.map(String::from),
        label_en: label_en.map(String::from),
        description: description.map(String::from),
        ..Configuration::default()
    };
    c.pose.insert(
        "P".into(),
        Pose {
            position: Some(Vector3::unitless(1.0, 2.0, 3.0)),
            ..Pose::default()
        },
    );
    c
}

const POSE: &str = "[pose.P]\nposition = { x = 1.0, y = 2.0, z = 3.0 }\n";

/// 既にあるファイルにキーを足した形は、そのキーも含めて新しく作った形と同じ。
#[test]
fn a_key_added_to_an_existing_record_lands_where_a_fresh_file_has_it() {
    let two = configuration(Some("Stowed"), None, Some("Folded"));
    let three = configuration(Some("Stowed"), Some("Stowed (en)"), Some("Folded"));
    let sections = CONFIGURATION_SECTIONS;
    let before = written(None, &two, sections);
    assert!(before.starts_with("label       = \"Stowed\"\ndescription = \"Folded\"\n"));
    let edited = written(Some(&before), &three, sections);
    assert_eq!(edited, written(None, &three, sections));
    assert!(edited.starts_with(
        "label       = \"Stowed\"\nlabel_en    = \"Stowed (en)\"\ndescription = \"Folded\"\n"
    ));
    // 消す方向も、新しく作った形に戻る
    assert_eq!(written(Some(&edited), &two, sections), before);
    // 先頭に足す（`label` が無いところへ `label` を足す）
    let only_description = written(None, &configuration(None, None, Some("Folded")), sections);
    assert_eq!(
        written(Some(&only_description), &two, sections),
        written(None, &two, sections)
    );
}

fn harness() -> Harness {
    Harness {
        id: "H".into(),
        label: None,
        from: PortRef::new("A", "P"),
        to: PortRef::new("B", "P"),
        nets: vec!["N".into()],
        wire: None,
        cores: Some(2),
        length: Some(Quantity::new(0.35, "m")),
    }
}

/// 途中の省略できるキー（ハーネスの `wire`）も、先頭の `label` も、新しく作った形と同じ位置に入る。
#[test]
fn an_optional_middle_key_is_inserted_in_the_canonical_place() {
    let mut h = harness();
    let before = written(None, &h, &[]);
    h.wire = Some("local/wire_awg26".into());
    let edited = written(Some(&before), &h, &[]);
    assert_eq!(edited, written(None, &h, &[]));
    assert!(edited.find("nets").unwrap() < edited.find("wire").unwrap());
    assert!(edited.find("wire").unwrap() < edited.find("cores").unwrap());
    h.label = Some("Harness".into());
    assert_eq!(written(Some(&edited), &h, &[]), written(None, &h, &[]));
}

/// 手で並べ替えたファイルは並べ替えない。足した 1 行だけが、正規の順での直前のキーのすぐ後ろに入る。
/// 既にある行のコメントと空白は動かない。
#[test]
fn a_hand_ordered_file_keeps_its_order_and_only_the_new_key_is_placed() {
    let hand =
        format!("# kept\ndescription = \"Folded\" # trailing\nlabel       = \"Stowed\"\n\n{POSE}");
    let value = configuration(Some("Stowed"), Some("Stowed (en)"), Some("Folded"));
    let out = written(Some(&hand), &value, CONFIGURATION_SECTIONS);
    assert_eq!(
        out,
        format!(
            "# kept\ndescription = \"Folded\" # trailing\nlabel       = \"Stowed\"\nlabel_en    = \"Stowed (en)\"\n\n{POSE}"
        )
    );
    // 値だけの変更は、順もコメントも動かさない
    let changed = configuration(Some("Stowed 2"), None, Some("Folded"));
    let again = written(Some(&hand), &changed, CONFIGURATION_SECTIONS);
    assert_eq!(again, hand.replace("\"Stowed\"", "\"Stowed 2\""));
}

/// 直前のキーが表に無い（正規の順で先頭）ときは、行の先頭に入る（見出しの表より前）。
#[test]
fn a_new_first_key_goes_to_the_top_of_the_rows_before_any_table() {
    let existing = format!("description = \"Folded\"\n\n{POSE}");
    let value = configuration(Some("Stowed"), None, Some("Folded"));
    let out = written(Some(&existing), &value, CONFIGURATION_SECTIONS);
    assert_eq!(
        out,
        format!("label       = \"Stowed\"\ndescription = \"Folded\"\n\n{POSE}")
    );
}

/// 手で並べ替えたハーネスの途中に足す: `wire` は正規の順での直前の `nets` のすぐ後ろ（末尾ではない）。ほかの行は順もコメントも動かない（`=` は足した行のところで揃え直される）。
#[test]
fn a_new_key_in_a_hand_ordered_record_follows_its_canonical_predecessor() {
    let hand = "from = \"A.P\"\nnets = [\"N\"] # keep\nto = \"B.P\"\ncores = 2\nlength = { value = 0.35, unit = \"m\" }\n";
    let mut h = harness();
    h.wire = Some("w".into());
    let out = written(Some(hand), &h, &[]);
    assert_eq!(
        out,
        "from   = \"A.P\"\nnets   = [\"N\"] # keep\nwire   = \"w\"\nto     = \"B.P\"\ncores  = 2\nlength = { value = 0.35, unit = \"m\" }\n"
    );
}
