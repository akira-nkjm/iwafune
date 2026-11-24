//! 通信の回線の Op（`CreateCommLink`・`RemoveCommLink`・`RenameCommLink`）。fixtures/demo-6u に対して計画を立てる。

use fune_model::design::{LinkDirection, RefSite, Referent};
use fune_model::value::SettingValue;
use fune_model::{Design, check};
use fune_ops::{Op, OpError, plan, to_cli};
use fune_store::format::format_text;
use fune_store::read::design_of;
use fune_store::{FileSet, Snapshot, apply_changes, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

fn applied(snap: &Snapshot, ops: &[Op]) -> Snapshot {
    let p = plan(snap, ops).unwrap();
    let mut files: FileSet = snap.files.clone();
    apply_changes(&mut files, &p.changes);
    Snapshot {
        root: snap.root.clone(),
        design: design_of(&files),
        files,
    }
}

fn no_errors(d: &Design) {
    assert!(check(d).iter().all(|x| !x.is_error()), "{:?}", check(d));
}

fn create(id: &str) -> Op {
    Op::CreateCommLink {
        id: id.into(),
        direction: LinkDirection::Up,
        part: "UHF_TRX".into(),
        band: "uhf".into(),
    }
}

fn rename(id: &str, to: &str) -> Op {
    Op::RenameCommLink {
        id: id.into(),
        to: to.into(),
    }
}

fn set_downlink(name: &str) -> Op {
    Op::SetSetting {
        path: "analysis.comm.data_downlink".into(),
        value: Some(SettingValue::Text(name.into())),
    }
}

#[test]
fn create_writes_the_three_fields_that_make_a_link_load() {
    let snap = snapshot();
    let p = plan(&snap, &[create("x_up")]).unwrap();
    assert_eq!(p.changes.len(), 1);
    let after = p.changes[0].after.as_deref().unwrap();
    assert!(
        after.ends_with(
            "[link.x_up]\ndirection = \"up\"\npart      = \"UHF_TRX\"\nband      = \"uhf\"\n"
        ),
        "{after}"
    );
    assert_eq!(format_text("analysis/comm.toml", after).unwrap(), after);
    let link = &p.design.analysis.comm.as_ref().unwrap().links["x_up"];
    assert_eq!(link.direction, LinkDirection::Up);
    assert_eq!((link.part.as_str(), link.band.as_str()), ("UHF_TRX", "uhf"));
    no_errors(&p.design);
    assert_eq!(
        to_cli(&create("x_up")),
        ["fune link add x_up --direction up --part UHF_TRX --band uhf"]
    );
}

#[test]
fn create_works_without_a_comm_file_and_remove_restores_the_design() {
    let base = snapshot();
    let mut files = base.files.clone();
    files.remove("analysis/comm.toml");
    let snap = Snapshot {
        root: base.root.clone(),
        design: design_of(&files),
        files,
    };
    let p = plan(&snap, &[create("only")]).unwrap();
    let after = p.changes[0].after.as_deref().unwrap();
    assert_eq!(
        after,
        "[link.only]\ndirection = \"up\"\npart      = \"UHF_TRX\"\nband      = \"uhf\"\n"
    );
    // 作って消すと、変更は残らない（最後の回線なら `link` の表ごと消える）
    let p = plan(
        &snap,
        &[create("only"), Op::RemoveCommLink { id: "only".into() }],
    )
    .unwrap();
    let text = p
        .changes
        .iter()
        .find(|c| c.path == "analysis/comm.toml")
        .and_then(|c| c.after.as_deref())
        .unwrap_or_default();
    assert!(!text.contains("link"), "{text:?}");
}

#[test]
fn create_refuses_duplicates_bad_names_and_unknown_parts_or_bands() {
    let snap = snapshot();
    for id in ["uhf_up", "UHF_UP"] {
        assert!(matches!(
            plan(&snap, &[create(id)]),
            Err(OpError::AlreadyExists { .. })
        ));
    }
    for id in ["", "bad id", "a@b", "1x"] {
        assert!(
            matches!(plan(&snap, &[create(id)]), Err(OpError::Invalid { .. })),
            "{id:?}"
        );
    }
    let with = |part: &str, band: &str| Op::CreateCommLink {
        id: "y".into(),
        direction: LinkDirection::Down,
        part: part.into(),
        band: band.into(),
    };
    // 無い部品は最後の検査（`ref.part`）が拒む。空の部品・帯域は拒む
    match plan(&snap, &[with("NOPE", "uhf")]) {
        Err(OpError::Rejected { diagnostics }) => {
            assert!(
                diagnostics.iter().all(|d| d.code == "ref.part"),
                "{diagnostics:?}"
            )
        }
        other => panic!("expected Rejected, got {:?}", other.map(|p| p.changes)),
    }
    for (part, band) in [("", "uhf"), ("UHF_TRX", " ")] {
        assert!(matches!(
            plan(&snap, &[with(part, band)]),
            Err(OpError::Invalid { .. })
        ));
    }
    // 局に無い帯域は警告（`ref.band`）で、書き込みは止めない
    let p = plan(&snap, &[with("UHF_TRX", "no_band")]).unwrap();
    assert!(p.diagnostics.iter().any(|d| d.code == "ref.band"));
}

#[test]
fn rename_rewrites_the_table_the_downlink_choice_and_requirement_checks() {
    let snap = applied(&snapshot(), &[set_downlink("s_down")]);
    let before = snap.design.references(Referent::Link("s_down"));
    assert!(before.contains(&RefSite::CommDataDownlink), "{before:?}");
    let checked: Vec<&str> = before
        .iter()
        .filter_map(|s| match s {
            RefSite::RequirementCheck { requirement, .. } => Some(requirement.as_str()),
            _ => None,
        })
        .collect();
    // `margin:s_down@TOKYO_GS`（R_LINK_S）と `pfd:s_down`（R_PFD。局の無いキー）
    assert_eq!(checked, ["R_LINK_S", "R_PFD"]);

    let renamed = applied(&snap, &[rename("s_down", "s_dl")]);
    let d = &renamed.design;
    assert!(d.references(Referent::Link("s_down")).is_empty());
    assert_eq!(d.references(Referent::Link("s_dl")).len(), before.len());
    let comm = d.analysis.comm.as_ref().unwrap();
    assert!(comm.links.contains_key("s_dl") && !comm.links.contains_key("s_down"));
    assert_eq!(comm.data_downlink.as_deref(), Some("s_dl"));
    assert_eq!(
        d.requirements["R_LINK_S"].verify.check.as_deref(),
        Some("margin:s_dl@TOKYO_GS")
    );
    assert_eq!(
        d.requirements["R_PFD"].verify.check.as_deref(),
        Some("pfd:s_dl")
    );
    no_errors(d);
    // 戻すと元のバイト列になる
    let back = applied(&renamed, &[rename("s_dl", "s_down")]);
    assert_eq!(back.files, snap.files);
    // 部分一致の名前の回線のキーは書き換えない
    let other = applied(&snap, &[rename("uhf_down", "uhf_dl")]);
    assert_eq!(
        other.design.requirements["R_LINK_S"].verify.check,
        snap.design.requirements["R_LINK_S"].verify.check
    );
}

#[test]
fn rename_refuses_taken_invalid_and_missing_names() {
    let snap = snapshot();
    for to in ["uhf_up", "UHF_UP"] {
        assert!(matches!(
            plan(&snap, &[rename("uhf_down", to)]),
            Err(OpError::AlreadyExists { .. })
        ));
    }
    assert!(matches!(
        plan(&snap, &[rename("uhf_down", "bad id")]),
        Err(OpError::Invalid { .. })
    ));
    assert!(matches!(
        plan(&snap, &[rename("nope", "fresh")]),
        Err(OpError::NotFound { .. })
    ));
    assert_eq!(
        to_cli(&rename("uhf_down", "uhf_dl")),
        ["fune link rename uhf_down uhf_dl"]
    );
}

#[test]
fn remove_clears_the_downlink_choice_and_refuses_while_a_requirement_names_the_link() {
    let snap = applied(&snapshot(), &[set_downlink("uhf_down")]);
    let p = plan(
        &snap,
        &[Op::RemoveCommLink {
            id: "uhf_down".into(),
        }],
    )
    .unwrap();
    let comm = p.design.analysis.comm.as_ref().unwrap();
    assert!(!comm.links.contains_key("uhf_down"));
    assert_eq!(
        comm.data_downlink, None,
        "the choice that named the link is cleared"
    );
    no_errors(&p.design);
    // 要求の判定のキーが名指している回線は消せない
    match plan(
        &snap,
        &[Op::RemoveCommLink {
            id: "s_down".into(),
        }],
    ) {
        Err(OpError::Invalid { message }) => {
            assert!(
                message.contains("R_LINK_S") && message.contains("R_PFD"),
                "{message}"
            )
        }
        other => panic!("expected Invalid, got {:?}", other.map(|p| p.changes)),
    }
    assert!(matches!(
        plan(&snap, &[Op::RemoveCommLink { id: "nope".into() }]),
        Err(OpError::NotFound { .. })
    ));
}
