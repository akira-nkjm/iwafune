//! 形態の姿勢の行は参照の索引に載り、部品の名前の変更でキーが書き換わり、部品の削除で行が消える（モードの表の行と同じ方針）。

use fune_model::check;
use fune_model::design::{RefSite, Referent};
use fune_ops::{Op, plan};
use fune_store::read::design_of;
use fune_store::{FileSet, Snapshot};
use std::path::PathBuf;

const CONFIGURATION: &str = "label = \"Stowed\"\n\n[pose.PADDLE]\nposition = { x = 2.6, y = -1.5, z = 0.0 }\n\n[pose.ARM]\nposition = { x = 0.0, y = 0.1, z = 0.0 }\n";

fn snapshot() -> Snapshot {
    let mut files = FileSet::new();
    files.insert("fune.toml", "[project]\nname = \"t\"\n");
    files.insert("parts/BUS.toml", "kind = \"structure\"\n");
    files.insert(
        "parts/PADDLE.toml",
        "kind = \"structure\"\n\n[mount]\non = \"BUS\"\n\n[pose]\nposition = { x = 0.0, y = 0.2, z = 0.0 }\n",
    );
    files.insert(
        "parts/ARM.toml",
        "kind = \"structure\"\n\n[mount]\non = \"BUS\"\n",
    );
    files.insert("configurations/STOWED.toml", CONFIGURATION);
    let design = design_of(&files);
    Snapshot {
        root: PathBuf::new(),
        files,
        design,
    }
}

fn after<'a>(p: &'a fune_ops::Plan, path: &str) -> &'a str {
    p.changes
        .iter()
        .find(|c| c.path == path)
        .and_then(|c| c.after.as_deref())
        .unwrap_or_else(|| panic!("{path} not changed"))
}

#[test]
fn the_index_lists_the_configuration_row() {
    let snap = snapshot();
    let sites = snap.design.references(Referent::Part("PADDLE"));
    assert!(sites.contains(&RefSite::ConfigurationPose {
        configuration: "STOWED".into()
    }));
    assert!(
        !snap
            .design
            .references(Referent::Part("BUS"))
            .iter()
            .any(|s| matches!(s, RefSite::ConfigurationPose { .. }))
    );
}

#[test]
fn renaming_a_part_rewrites_the_key_in_the_configuration() {
    let p = plan(
        &snapshot(),
        &[Op::RenamePart {
            id: "PADDLE".into(),
            to: "WING".into(),
        }],
    )
    .unwrap();
    assert_eq!(
        after(&p, "configurations/STOWED.toml"),
        CONFIGURATION.replace("[pose.PADDLE]", "[pose.WING]")
    );
    let d = &p.design;
    assert!(d.configurations["STOWED"].pose.contains_key("WING"));
    assert!(!d.configurations["STOWED"].pose.contains_key("PADDLE"));
    let errs: Vec<_> = check(d).into_iter().filter(|x| x.is_error()).collect();
    assert!(errs.is_empty(), "{errs:?}");
}

#[test]
fn removing_a_part_removes_its_row_like_a_mode_row() {
    let p = plan(
        &snapshot(),
        &[Op::RemovePart {
            id: "PADDLE".into(),
        }],
    )
    .unwrap();
    assert_eq!(
        after(&p, "configurations/STOWED.toml"),
        "label = \"Stowed\"\n\n[pose.ARM]\nposition = { x = 0.0, y = 0.1, z = 0.0 }\n"
    );
    assert!(
        !p.design.configurations["STOWED"]
            .pose
            .contains_key("PADDLE")
    );
    assert!(check(&p.design).iter().all(|x| !x.is_error()));
}

#[test]
fn removing_the_last_row_leaves_no_empty_table() {
    let mut snap = snapshot();
    snap.files.insert(
        "configurations/STOWED.toml",
        "label = \"Stowed\"\n\n[pose.PADDLE]\nposition = { x = 2.6, y = -1.5, z = 0.0 }\n",
    );
    snap.design = design_of(&snap.files);
    let p = plan(
        &snap,
        &[Op::RemovePart {
            id: "PADDLE".into(),
        }],
    )
    .unwrap();
    assert_eq!(
        after(&p, "configurations/STOWED.toml"),
        "label = \"Stowed\"\n"
    );
}

/// 選択元を両方書き換え、参照のどちらかが残る限り削除を拒む。
#[test]
fn configuration_selection_references_follow_rename_and_block_removal() {
    use fune_model::value::SettingValue;
    use fune_ops::OpError;

    let snap = snapshot();
    let launch = |id: Option<&str>| Op::SetSetting {
        path: "conditions.launch.configuration".into(),
        value: id.map(|id| SettingValue::Text(id.into())),
    };
    let phase = |id: Option<&str>| Op::SetPhase {
        phase: fune_model::design::Phase {
            id: "EARLY".into(),
            configuration: id.map(Into::into),
            ..Default::default()
        },
    };
    let setup = [launch(Some("STOWED")), phase(Some("STOWED"))];
    let rename = Op::RenameConfiguration {
        id: "STOWED".into(),
        to: "FOLDED".into(),
    };
    let renamed = plan(&snap, &[setup.as_slice(), &[rename]].concat()).unwrap();
    assert_eq!(
        renamed.design.references(Referent::Configuration("FOLDED")),
        [
            RefSite::LaunchConfiguration,
            RefSite::PhaseConfiguration {
                phase: "EARLY".into()
            }
        ]
    );
    assert!(
        renamed
            .design
            .references(Referent::Configuration("STOWED"))
            .is_empty()
    );
    assert_eq!(
        after(&renamed, "conditions/launch.toml"),
        "configuration = \"FOLDED\"\n"
    );
    assert!(after(&renamed, "phases/EARLY.toml").contains("= \"FOLDED\""));

    let remove = Op::RemoveConfiguration {
        id: "STOWED".into(),
    };
    for (clear, referrer) in [
        (launch(None), "`configuration` of phase(s) EARLY"),
        (phase(None), "`configuration` in conditions/launch.toml"),
    ] {
        let ops = [setup.as_slice(), &[clear, remove.clone()]].concat();
        let Err(OpError::Invalid { message }) = plan(&snap, &ops) else {
            panic!("missing reference refusal");
        };
        assert!(message.contains(referrer), "{message}");
        assert!(message.contains("Change those first"), "{message}");
    }
    let cleared = plan(
        &snap,
        &[setup.as_slice(), &[launch(None), phase(None), remove]].concat(),
    )
    .unwrap();
    assert!(!cleared.design.configurations.contains_key("STOWED"));
}
