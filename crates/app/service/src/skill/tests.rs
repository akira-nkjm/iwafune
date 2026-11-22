//! skill の入れ方の確かめ（一時ディレクトリ）。

use super::*;

fn text(version: &str) -> String {
    TEMPLATE.replace(VERSION_SLOT, version)
}

#[test]
fn rendered_text_has_the_frontmatter_and_the_running_version() {
    let t = render();
    assert!(t.starts_with("---\nname: fune\ndescription: "));
    assert!(!t.contains("{{"));
    assert_eq!(version_marker(&t).as_deref(), Some(VERSION));
}

#[test]
fn marker_is_read_only_from_the_frontmatter() {
    assert_eq!(version_marker("no frontmatter"), None);
    let outside = "---\nname: fune\n---\n  fune-version: \"9.9.9\"\n";
    assert_eq!(version_marker(outside), None);
}

#[test]
fn new_file_is_created_with_its_directories() {
    let tmp = tempfile::tempdir().unwrap();
    let skills = tmp.path().join("a/b");
    let o = install(&skills, false).unwrap();
    assert_eq!(o.status, InstallStatus::Created);
    let written = std::fs::read_to_string(skills.join("fune/SKILL.md")).unwrap();
    assert_eq!(written, render());
}

#[test]
fn identical_file_is_up_to_date_and_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    install(tmp.path(), false).unwrap();
    let path = tmp.path().join("fune/SKILL.md");
    let before = std::fs::metadata(&path).unwrap().modified().unwrap();
    let o = install(tmp.path(), false).unwrap();
    assert_eq!(o.status, InstallStatus::UpToDate);
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        before
    );
}

#[test]
fn file_from_another_version_is_replaced() {
    let tmp = tempfile::tempdir().unwrap();
    install_text(tmp.path(), &text("0.0.1"), "0.0.1", false).unwrap();
    let o = install(tmp.path(), false).unwrap();
    assert_eq!(o.status, InstallStatus::Updated);
    assert_eq!(o.previous_version.as_deref(), Some("0.0.1"));
    let written = std::fs::read_to_string(tmp.path().join("fune/SKILL.md")).unwrap();
    assert_eq!(written, render());
}

#[test]
fn edited_file_is_refused_unless_forced() {
    let tmp = tempfile::tempdir().unwrap();
    install(tmp.path(), false).unwrap();
    let path = tmp.path().join("fune/SKILL.md");
    let edited = format!("{}\nMy own note.\n", render());
    std::fs::write(&path, &edited).unwrap();
    let e = install(tmp.path(), false).unwrap_err();
    assert!(matches!(e, ServiceError::SkillModified { .. }), "{e}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), edited);
    let o = install(tmp.path(), true).unwrap();
    assert_eq!(o.status, InstallStatus::Overwritten);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), render());
}

#[test]
fn file_without_a_marker_is_treated_as_edited() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("fune")).unwrap();
    std::fs::write(tmp.path().join("fune/SKILL.md"), "something else\n").unwrap();
    assert!(install(tmp.path(), false).is_err());
}

#[test]
fn default_dir_is_the_project_root_or_the_start_directory() {
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(
        default_skills_dir(tmp.path()),
        tmp.path().join(".claude/skills")
    );
    fune_store::project::init(tmp.path(), "p").unwrap();
    let sub = tmp.path().join("parts");
    assert_eq!(default_skills_dir(&sub), tmp.path().join(".claude/skills"));
}
