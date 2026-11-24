//! 形態（`configurations/<ID>.toml`）の読み書き・正規形・検査の診断。

use fune_model::{Diagnostic, Severity, Target, check};
use fune_store::FileSet;
use fune_store::format::{format_files, format_text};
use fune_store::read::design_of;
use fune_store::write::records::{CONFIGURATION_SECTIONS, record_doc};

const PROJECT: &str = "[project]\nname = \"t\"\n";
const MOUNTED: &str = "kind = \"structure\"\n\n[mount]\non = \"BUS\"\n\n[pose]\nposition = { x = 0.0, y = 0.2, z = 0.0 }\n\n[attr]\nmass = { value = 1.0, unit = \"kg\" }\n";
const CANONICAL: &str = "label       = \"Stowed\"\ndescription = \"Paddle folded onto the +X face\"\n\n[pose.PADDLE]\nposition    = { x = 2.6, y = -1.5, z = 0.0, unit = \"mm\" }\norientation = { w = 0.707107, x = 0.0, y = 0.0, z = -0.707107 }\n";

fn files(extra: &[(&str, &str)]) -> FileSet {
    let mut set = FileSet::new();
    set.insert("fune.toml", PROJECT);
    set.insert(
        "parts/BUS.toml",
        "kind = \"structure\"\n\n[attr]\nmass = { value = 1.0, unit = \"kg\" }\n",
    );
    set.insert("parts/PADDLE.toml", MOUNTED);
    for (p, t) in extra {
        set.insert(*p, *t);
    }
    set
}

fn diags(extra: &[(&str, &str)]) -> Vec<Diagnostic> {
    check(&design_of(&files(extra)))
}

fn find<'a>(d: &'a [Diagnostic], code: &str) -> Option<&'a Diagnostic> {
    d.iter().find(|x| x.code == code)
}

#[test]
fn a_configuration_is_read_by_file_name_and_checks_clean() {
    let set = files(&[("configurations/STOWED.toml", CANONICAL)]);
    let design = design_of(&set);
    let c = &design.configurations["STOWED"];
    assert_eq!(c.id.as_str(), "STOWED");
    assert_eq!(c.label.as_deref(), Some("Stowed"));
    assert_eq!(c.pose["PADDLE"].position.as_ref().unwrap().x, 2.6);
    let d: Vec<_> = check(&design)
        .into_iter()
        .filter(|x| x.severity != Severity::Info)
        .collect();
    assert!(d.is_empty(), "{d:?}");
    assert!(design.load_diagnostics.is_empty());
}

/// 書き出しは正規形（`fune fmt` が何も変えない）で、読み直すと同じ値。
#[test]
fn the_written_form_is_canonical_and_round_trips() {
    let design = design_of(&files(&[("configurations/STOWED.toml", CANONICAL)]));
    let c = &design.configurations["STOWED"];
    let doc = record_doc("configurations/STOWED.toml", c, CONFIGURATION_SECTIONS).unwrap();
    let written = fune_store::format::finish(doc);
    assert_eq!(written, CANONICAL);
    assert_eq!(
        format_text("configurations/STOWED.toml", CANONICAL).unwrap(),
        CANONICAL
    );
    let set = files(&[("configurations/STOWED.toml", CANONICAL)]);
    assert!(format_files(&set).is_empty());
}

#[test]
fn fmt_normalizes_integers_and_alignment() {
    let loose = "label = \"Stowed\"\n\n[pose.PADDLE]\nposition = { x = 2, y = 0, z = 0 }\n";
    let out = format_text("configurations/STOWED.toml", loose).unwrap();
    assert_eq!(
        out,
        "label = \"Stowed\"\n\n[pose.PADDLE]\nposition = { x = 2.0, y = 0.0, z = 0.0 }\n"
    );
}

#[test]
fn an_unknown_key_is_rejected_at_load() {
    let d = diags(&[("configurations/X.toml", "colour = \"red\"\n")]);
    assert!(d.iter().any(|x| x.severity == Severity::Error), "{d:?}");
    let design = design_of(&files(&[("configurations/X.toml", "colour = \"red\"\n")]));
    assert!(design.configurations.is_empty());
    let d = diags(&[("configurations/X.toml", "[pose.PADDLE]\nspeed = 1\n")]);
    assert!(d.iter().any(|x| x.severity == Severity::Error), "{d:?}");
}

#[test]
fn nested_directories_are_outside_the_convention() {
    let d = diags(&[("configurations/a/X.toml", "label = \"x\"\n")]);
    assert!(find(&d, "layout.unknown_file").is_some(), "{d:?}");
}

#[test]
fn a_pose_row_for_a_missing_part_is_ref_part() {
    let d = diags(&[(
        "configurations/X.toml",
        "[pose.GHOST]\nposition = { x = 1.0, y = 0.0, z = 0.0 }\n",
    )]);
    let found = find(&d, "ref.part").expect("ref.part");
    assert_eq!(found.severity, Severity::Error);
    assert_eq!(
        found.target,
        Some(Target::configuration("X", Some("pose.GHOST")))
    );
    assert_eq!(
        found.target.as_ref().unwrap().to_string(),
        "configurations.X.pose.GHOST"
    );
}

#[test]
fn pose_problems_use_the_part_pose_check_with_the_configuration_target() {
    let bad = "[pose.PADDLE]\nposition = { x = 1.0, y = 0.0, z = 0.0, unit = \"kg\" }\norientation = { w = 2.0, x = 0.0, y = 0.0, z = 0.0 }\n";
    let d = diags(&[("configurations/X.toml", bad)]);
    for code in ["unit.dimension", "pose.quaternion_norm"] {
        let f = find(&d, code).unwrap_or_else(|| panic!("{code} in {d:?}"));
        assert_eq!(f.severity, Severity::Error);
        assert_eq!(
            f.target,
            Some(Target::configuration("X", Some("pose.PADDLE")))
        );
        // 位置は形態のファイルの行
        assert_eq!(f.file.as_deref(), Some("configurations/X.toml"), "{f:?}");
        assert!(f.line.is_some(), "{f:?}");
    }
    let d = diags(&[(
        "configurations/X.toml",
        "[pose.PADDLE]\nposition = { x = 1.0, y = 0.0, z = 0.0, unit = \"furlong\" }\n",
    )]);
    assert!(find(&d, "unit.parse").is_some(), "{d:?}");
}

/// 取付の無い部品（根）への上書きは、機体座標での姿勢として解決が使う。警告は出ない（検査は取付のある部品と同じ）。
#[test]
fn an_override_on_a_root_is_not_a_warning() {
    let d = diags(&[(
        "configurations/X.toml",
        "[pose.BUS]\nposition = { x = 1.0, y = 0.0, z = 0.0 }\n",
    )]);
    assert!(
        find(&d, "configuration.pose_without_mount").is_none(),
        "{d:?}"
    );
    // 取付のある部品と同じ姿勢の検査は根にも掛かる
    let d = diags(&[(
        "configurations/X.toml",
        "[pose.BUS]\nposition = { x = 1.0, y = 0.0, z = 0.0, unit = \"furlong\" }\n",
    )]);
    assert!(find(&d, "unit.parse").is_some(), "{d:?}");
}

#[test]
fn invalid_and_duplicate_ids_are_reported() {
    let d = diags(&[("configurations/bad id.toml", "label = \"x\"\n")]);
    let f = find(&d, "id.invalid").expect("id.invalid");
    assert_eq!(f.target, Some(Target::configuration("bad id", None)));
    let d = diags(&[
        ("configurations/STOWED.toml", "label = \"a\"\n"),
        ("configurations/stowed.toml", "label = \"b\"\n"),
    ]);
    assert!(find(&d, "id.duplicate").is_some(), "{d:?}");
}

/// 英語名は `label` と `description` の間に書く正規の順。キーの無いファイルは `fune fmt` が 1 バイトも変えない。
#[test]
fn label_en_has_its_canonical_place_and_a_file_without_it_is_untouched() {
    let with_en =
        "label       = \"収納\"\nlabel_en    = \"Stowed\"\ndescription = \"Paddle folded\"\n";
    let design = design_of(&files(&[("configurations/STOWED.toml", with_en)]));
    let c = &design.configurations["STOWED"];
    assert_eq!(c.label_en.as_deref(), Some("Stowed"));
    let doc = record_doc("configurations/STOWED.toml", c, CONFIGURATION_SECTIONS).unwrap();
    assert_eq!(fune_store::format::finish(doc), with_en);
    assert_eq!(
        format_text("configurations/STOWED.toml", with_en).unwrap(),
        with_en
    );
    // `fune fmt` は順を入れ替えず、`=` の位置だけ揃える
    let loose = "label_en = \"Stowed\"\nlabel = \"収納\"\n";
    assert_eq!(
        format_text("configurations/STOWED.toml", loose).unwrap(),
        "label_en = \"Stowed\"\nlabel    = \"収納\"\n"
    );
    assert!(format_files(&files(&[("configurations/STOWED.toml", CANONICAL)])).is_empty());
}
