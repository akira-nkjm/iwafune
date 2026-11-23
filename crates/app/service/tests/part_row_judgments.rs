//! 部品の一覧の行（`PartRow`）が持つ判定: 診断の件数・未設定の数・マヌーバのスラスタに選べるか。
//! 数えるのも選ぶのも service と model の 1 か所で、Web は並べるだけにする。
use fune_service::dto::PartRow;
use fune_service::{Workspace, query};
use std::path::{Path, PathBuf};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn row(rows: &[PartRow], id: &str) -> PartRow {
    rows.iter().find(|p| p.id == id).unwrap().clone()
}

/// デモの部品: 推進系のスラスタだけがマヌーバのスラスタに選べ、診断は 1 件もなく、未設定の数は値の出所から数える。
#[test]
fn demo_rows_carry_the_counts_and_the_thruster_decision() {
    let ws = Workspace::open(fixture()).unwrap();
    let rows = query::design_view(&ws).unwrap().parts;
    assert!(row(&rows, "EPT").maneuver_thruster);
    assert!(!row(&rows, "PANEL_PX").maneuver_thruster);
    assert_eq!(rows.iter().filter(|p| p.maneuver_thruster).count(), 1);
    for p in &rows {
        assert_eq!(p.diagnostics.errors, 0, "{}", p.id);
        let unset = p
            .values
            .values()
            .filter(|v| v.origin == fune_model::resolve::Origin::Unset)
            .count();
        assert_eq!(p.unset_count as usize, unset, "{}", p.id);
    }
    assert!(
        rows.iter().any(|p| p.unset_count > 0),
        "the demo has a part with a not-set attribute"
    );
}

/// 部品のファイルの中の誤りは、その部品の行の件数に入る（ほかの部品には入らない）。
/// 種別を継承した独自の種別（スラスタの子）の部品も、マヌーバのスラスタに選べる。
#[test]
fn edited_project_counts_diagnostics_and_inherits_the_thruster_role() {
    let tmp = tempfile::tempdir().unwrap();
    copy_dir(&fixture(), tmp.path());
    let kind = tmp.path().join("defs/kind/ion_engine.toml");
    std::fs::write(
        &kind,
        "extends = \"propulsion_thruster\"\nlabel = \"イオンエンジン\"\n",
    )
    .unwrap();
    let part = tmp.path().join("parts/propulsion/ION.toml");
    std::fs::write(&part, "kind = \"ion_engine\"\n").unwrap();
    let panel = tmp.path().join("parts/structure/PANEL_PX.toml");
    let text = std::fs::read_to_string(&panel).unwrap();
    // 厚さに時間の単位を書く（次元の誤り）
    let broken = text.replace(
        "thickness     = { value = 1.5, unit = \"mm\" }",
        "thickness     = { value = 1.5, unit = \"s\" }",
    );
    assert_ne!(broken, text);
    std::fs::write(&panel, broken).unwrap();

    let ws = Workspace::open(tmp.path()).unwrap();
    let rows = query::design_view(&ws).unwrap().parts;
    assert!(row(&rows, "ION").maneuver_thruster);
    assert!(row(&rows, "EPT").maneuver_thruster);
    let counts = row(&rows, "PANEL_PX").diagnostics;
    assert!(counts.errors + counts.warnings > 0, "{counts:?}");
    assert_eq!(row(&rows, "PANEL_PY").diagnostics.errors, 0);
}

/// 構造の部品（系が構造）だけが `structural`。方向の属性と視野の対応は、属性の定義（`DefsView`）が返す。
#[test]
fn demo_marks_structure_and_the_direction_and_fov_attributes() {
    let ws = Workspace::open(fixture()).unwrap();
    let view = query::design_view(&ws).unwrap();
    assert!(row(&view.parts, "PANEL_PX").structural);
    assert!(!row(&view.parts, "EPT").structural);
    for p in &view.parts {
        assert_eq!(
            p.structural,
            p.system == Some(fune_service::dto::System::Structure),
            "{}",
            p.id
        );
    }
    let directions: Vec<&str> = view
        .defs
        .attributes
        .iter()
        .filter(|a| a.unit_vector)
        .map(|a| a.id.as_str())
        .collect();
    assert_eq!(
        directions,
        ["axis", "boresight", "direction", "normal", "spin_axis"]
    );
    let fovs: Vec<(&str, &str)> = view
        .defs
        .attributes
        .iter()
        .filter_map(|a| Some((a.id.as_str(), a.fov_axis.as_deref()?)))
        .collect();
    assert_eq!(fovs, [("fov_half_angle", "boresight")]);
}
