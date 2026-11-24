//! 軌道の出どころ（`source`・`maneuver`）の読み書き: 設計の軌道（`conditions/orbit.toml`）とフェーズの `[orbit]` のキー名、
//! 読んだ値、書かなければ出ないこと（既存のファイルは変わらない）、設定の表に欄があること。

use fune_model::Design;
use fune_model::design::{OrbitSettings, OrbitSource};
use fune_model::path::SettingPath;
use fune_store::FileSet;
use fune_store::read::design_of;

const PROJECT: &str = "[project]\nname = \"t\"\nschema = 2\n";

fn read(files: &[(&str, &str)]) -> Design {
    let mut set = FileSet::new();
    set.insert("fune.toml", PROJECT);
    for (p, t) in files {
        set.insert(*p, *t);
    }
    let design = design_of(&set);
    assert!(
        design.load_diagnostics.is_empty(),
        "{:?}",
        design.load_diagnostics
    );
    design
}

/// ファイルのキーの名前は `source`・`maneuver`。設計の軌道とフェーズの軌道のどちらにも書け、読むと型付きの値になる。
#[test]
fn the_keys_read_back_in_the_design_orbit_and_in_a_phase() {
    let d = read(&[
        (
            "conditions/orbit.toml",
            "source = \"transfer\"\nmaneuver = \"MV\"\n",
        ),
        (
            "phases/CRUISE.toml",
            "order = 1\n\n[orbit]\nsource = \"transfer\"\nmaneuver = \"MV\"\n",
        ),
    ]);
    let design = d.conditions.orbit.as_ref().unwrap();
    assert_eq!(design.source, Some(OrbitSource::Transfer));
    assert_eq!(design.maneuver.as_deref(), Some("MV"));
    let phase = d.phases["CRUISE"].orbit.as_ref().unwrap();
    assert_eq!(phase.source_kind(), OrbitSource::Transfer);
    assert_eq!(phase.maneuver.as_deref(), Some("MV"));
    // 設定の表から引ける（`fune get`・`fune set` と同じパス）
    let value = |key: &str| d.setting(&SettingPath::conditions(&["orbit", key]));
    assert!(value("source").is_some() && value("maneuver").is_some());
    // 知らない出どころは読めない（閉じた語彙）
    let mut set = FileSet::new();
    set.insert("fune.toml", PROJECT);
    set.insert("conditions/orbit.toml", "source = \"teleport\"\n");
    assert!(!design_of(&set).load_diagnostics.is_empty());
}

/// 書かなければ書き出さない（出どころの既定は `elements`）。設定の表には未入力の欄として出る（選んで入れるため）。
#[test]
fn an_orbit_without_a_source_writes_nothing_and_shows_empty_cells() {
    let text = toml::to_string(&OrbitSettings::default()).unwrap();
    assert!(
        !text.contains("source") && !text.contains("maneuver"),
        "{text}"
    );
    let d = read(&[(
        "conditions/orbit.toml",
        "altitude = { value = 550.0, unit = \"km\" }\n",
    )]);
    assert_eq!(
        d.conditions.orbit.as_ref().unwrap().source_kind(),
        OrbitSource::Elements
    );
    let entries = d.setting_entries();
    for key in ["source", "maneuver"] {
        let entry = entries
            .iter()
            .find(|e| e.path.to_string() == format!("conditions.orbit.{key}"))
            .unwrap_or_else(|| panic!("no entry for {key}"));
        assert!(entry.value.is_none(), "{key} is empty");
    }
}
