//! 書き込みが作るファイルは、すぐ `fune fmt --check` と `fune check`（TOML の書き方の検査）に通る（#504）。
//! 設定の整数（`0`・`-3`）は整数のままでよいキーにだけ整数で書き、ほかは float にする。
//! 子の表だけの見出し（最初の `[attr.<名前>]` を足したあとの `[attr]`）は残さない。

use fune_model::value::{Authored, Provenance, Scalar, SettingValue, Value};
use fune_store::FileSet;
use fune_store::format::format_text;
use fune_store::write::Edits;
use fune_store::write::entities::set_setting;
use fune_store::write::part::{set_attr, unset_attr};

fn keys(path: &[&str]) -> Vec<String> {
    path.iter().map(|s| (*s).to_owned()).collect()
}

/// 空のファイル集合に設定を 1 つ書いた本文。
fn written_setting(file: &str, path: &[&str], value: &SettingValue) -> String {
    let set = FileSet::new();
    let mut edits = Edits::new(&set);
    set_setting(&mut edits, file, &keys(path), Some(value)).unwrap();
    edits.files().get(file).unwrap().to_string()
}

/// 物理量・無次元の量の設定に打った整数は float で書かれ、`toml.integer` の対象にならない（再現: `fune set conditions.orbit.eccentricity 0`）。
#[test]
fn an_integer_typed_for_a_real_setting_is_written_as_a_float() {
    for (file, key, typed, want) in [
        (
            "conditions/orbit.toml",
            "eccentricity",
            0,
            "eccentricity = 0.0\n",
        ),
        (
            "analysis/aocs.toml",
            "drag_coefficient",
            2,
            "drag_coefficient = 2.0\n",
        ),
        (
            "analysis/power.toml",
            "initial_soc",
            1,
            "initial_soc = 1.0\n",
        ),
        (
            "analysis/aocs.toml",
            "srp_coefficient",
            -3,
            "srp_coefficient = -3.0\n",
        ),
    ] {
        let out = written_setting(file, &[key], &SettingValue::Integer(typed));
        assert_eq!(out, want, "{file}");
        assert_eq!(format_text(file, &out).unwrap(), out, "{file}");
    }
}

/// 整数のままでよいキー（員数・回数・升の数）は整数のまま。
#[test]
fn an_integer_key_stays_an_integer() {
    let out = written_setting(
        "analysis/power.toml",
        &["orbits"],
        &SettingValue::Integer(3),
    );
    assert_eq!(out, "orbits = 3\n");
    let out = written_setting(
        "analysis/thermal.toml",
        &["subdivide", "PANEL_PX", "u"],
        &SettingValue::Integer(3),
    );
    assert!(out.contains("u = 3\n"), "{out}");
    assert_eq!(format_text("analysis/thermal.toml", &out).unwrap(), out);
}

const FILE: &str = "parts/P.toml";

fn sourced(v: f64) -> Authored<Value> {
    Authored {
        value: Value::Scalar(Scalar::new(v, "kg")),
        from: Provenance::from_parts(Some("s".into()), Some("p.1".into()), Some("n".into())),
    }
}

fn plain(v: f64) -> Authored<Value> {
    Authored {
        value: Value::Scalar(Scalar::new(v, "kg")),
        from: None,
    }
}

/// 属性のまだ無い部品に、出典付きの最初の属性を足しても、値の行の無い `[attr]` が残らない。
#[test]
fn the_first_sourced_attribute_leaves_no_bare_attr_header() {
    let mut set = FileSet::new();
    set.insert(FILE, "kind = \"x\"\n");
    let mut edits = Edits::new(&set);
    set_attr(&mut edits, FILE, "mass", &sourced(1.0)).unwrap();
    let out = edits.files().get(FILE).unwrap().to_string();
    assert!(!out.contains("[attr]"), "{out}");
    assert_eq!(format_text(FILE, &out).unwrap(), out);
    // 値の行（出典なし）を足すと `[attr]` が現れ、消すとまた消える
    let mut edits = Edits::new(&set);
    set_attr(&mut edits, FILE, "mass", &sourced(1.0)).unwrap();
    set_attr(&mut edits, FILE, "a", &plain(2.0)).unwrap();
    let out = edits.files().get(FILE).unwrap().to_string();
    assert!(out.contains("[attr]\na = "), "{out}");
    assert_eq!(format_text(FILE, &out).unwrap(), out);
    unset_attr(&mut edits, FILE, "a", None).unwrap();
    let out = edits.files().get(FILE).unwrap().to_string();
    assert!(!out.contains("[attr]"), "{out}");
    assert_eq!(format_text(FILE, &out).unwrap(), out);
}

/// 最後の値を消して空になった設定のファイルは、`fune fmt` が書く空のファイルと同じ（改行 1 つ）。
#[test]
fn a_setting_file_emptied_by_unset_is_already_formatted() {
    let file = "conditions/orbit.toml";
    let mut set = FileSet::new();
    set.insert(file, "eccentricity = 0.0\n");
    let mut edits = Edits::new(&set);
    set_setting(&mut edits, file, &keys(&["eccentricity"]), None).unwrap();
    let out = edits.files().get(file).unwrap().to_string();
    assert_eq!(format_text(file, &out).unwrap(), out);
}
