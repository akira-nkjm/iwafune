//! ネット・ハーネス・解析ケース・出典・マヌーバの名前の変更（`RenameNet`・`RenameHarness`・`RenameCase`・`RenameSource`・
//! `RenameManeuver`）。fixtures/demo-6u に対して計画を立てる。変更後に旧い ID を指すものが索引に残らないこと
//! （`Design::references(旧い ID)` が空）を 5 つのエンティティで確かめる。

use fune_model::design::{RefSite, Referent};
use fune_model::value::{Provenance, Scalar, Value};
use fune_model::{Design, check};
use fune_ops::{Op, OpError, ViewNodePos, plan, to_cli};
use fune_store::format::format_text;
use fune_store::read::design_of;
use fune_store::{FileSet, Snapshot, apply_changes, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

/// 計画の変更を当てた後のスナップショット（次の計画の元にする）。
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

/// 2 つのファイルの集合で本文の違うファイルのパス。
fn plan_between(a: &FileSet, b: &FileSet) -> Vec<String> {
    let paths: std::collections::BTreeSet<&str> = a.paths().chain(b.paths()).collect();
    paths
        .into_iter()
        .filter(|p| a.get(p) != b.get(p))
        .map(str::to_owned)
        .collect()
}

fn no_errors(d: &Design) {
    assert!(check(d).iter().all(|x| !x.is_error()), "{:?}", check(d));
}

fn rename_ops(entity: &str, id: &str, to: &str) -> Op {
    let (id, to) = (id.to_owned(), to.to_owned());
    match entity {
        "net" => Op::RenameNet { id, to },
        "harness" => Op::RenameHarness { id, to },
        "case" => Op::RenameCase { id, to },
        "source" => Op::RenameSource { id, to },
        "maneuver" => Op::RenameManeuver { id, to },
        other => panic!("{other}"),
    }
}

fn referent<'a>(entity: &str, id: &'a str) -> Referent<'a> {
    match entity {
        "net" => Referent::Net(id),
        "harness" => Referent::Harness(id),
        "case" => Referent::Case(id),
        "source" => Referent::Source(id),
        "maneuver" => Referent::Maneuver(id),
        other => panic!("{other}"),
    }
}

/// 名前を変えると、旧い ID を指す参照は索引に残らず、新しい ID を指す参照が（元の数だけ）できる。
fn assert_renamed(snap: &Snapshot, entity: &str, id: &str, to: &str, expected_sites: usize) {
    let before = snap.design.references(referent(entity, id));
    assert_eq!(before.len(), expected_sites, "{entity} {id}: {before:?}");
    let p = plan(snap, &[rename_ops(entity, id, to)]).unwrap();
    let d = &p.design;
    assert!(d.references(referent(entity, id)).is_empty(), "{entity}");
    assert_eq!(d.references(referent(entity, to)).len(), expected_sites);
    no_errors(d);
    // 戻すと元のバイト列になる（コメントも並びも保つ）
    let back = applied(
        &applied(snap, &[rename_ops(entity, id, to)]),
        &[rename_ops(entity, to, id)],
    );
    let changed: Vec<_> = plan_between(&snap.files, &back.files);
    assert!(
        changed.is_empty(),
        "{entity}: rename and back changed {changed:?}"
    );
}

#[test]
fn rename_net_rewrites_ports_harnesses_and_view_nodes() {
    let snap = applied(
        &snapshot(),
        &[Op::SetViewLayout {
            view: "electrical".into(),
            nodes: vec![ViewNodePos {
                id: "P5V".into(),
                x: 10,
                y: 20,
                w: None,
                h: None,
            }],
            remove: vec![],
        }],
    );
    let sites = snap.design.references(Referent::Net("P5V"));
    assert!(sites.iter().any(|s| matches!(s, RefSite::PortNet { .. })));
    assert!(sites.contains(&RefSite::ViewNode {
        view: "electrical".into()
    }));
    assert_renamed(&snap, "net", "P5V", "P5V_MAIN", sites.len());
    // ハーネスの `nets` も書き換わる
    let harness_net = snap
        .design
        .harnesses
        .values()
        .find(|h| h.nets.iter().any(|n| n.as_str() == "P12V"))
        .map(|h| h.id.to_string())
        .unwrap();
    let p = plan(&snap, &[rename_ops("net", "P12V", "P12V_MAIN")]).unwrap();
    assert!(
        p.design.harnesses[harness_net.as_str()]
            .nets
            .iter()
            .any(|n| n.as_str() == "P12V_MAIN")
    );
    assert!(
        p.design.nets.contains_key("P12V_MAIN") && !p.design.nets.contains_key("P12V"),
        "the net file moves"
    );
    let renamed = applied(&snap, &[rename_ops("net", "P5V", "P5V_MAIN")]);
    let view = &renamed.design.views["electrical"];
    assert!(view.nodes.contains_key("P5V_MAIN") && !view.nodes.contains_key("P5V"));
    // 揃えた行のキーを長くしても、正規形（`fune fmt`）のまま
    let text = renamed.files.get("view/electrical.toml").unwrap();
    assert_eq!(format_text("view/electrical.toml", text).unwrap(), text);
}

#[test]
fn rename_harness_moves_the_file_only() {
    let snap = snapshot();
    assert_renamed(&snap, "harness", "H_OBC_CAM", "H_OBC_CAMERA", 0);
    let p = plan(&snap, &[rename_ops("harness", "H_OBC_CAM", "H_OBC_CAMERA")]).unwrap();
    let paths: Vec<&str> = p.changes.iter().map(|c| c.path.as_str()).collect();
    assert_eq!(
        paths,
        ["harness/H_OBC_CAM.toml", "harness/H_OBC_CAMERA.toml"]
    );
    assert_eq!(
        p.design.harnesses["H_OBC_CAMERA"].nets,
        snap.design.harnesses["H_OBC_CAM"].nets
    );
}

#[test]
fn rename_case_rewrites_the_default_case_and_requirement_quantification() {
    let snap = snapshot();
    let sites = snap.design.references(Referent::Case("hot_imaging"));
    assert!(sites.contains(&RefSite::CommonDefaultCase));
    assert!(
        sites
            .iter()
            .any(|s| matches!(s, RefSite::RequirementCase { .. }))
    );
    assert_renamed(&snap, "case", "hot_imaging", "hot_img", sites.len());
    let d = plan(&snap, &[rename_ops("case", "hot_imaging", "hot_img")])
        .unwrap()
        .design;
    assert_eq!(
        d.analysis.common.as_ref().unwrap().default_case.as_deref(),
        Some("hot_img")
    );
    assert!(d.cases.contains_key("hot_img") && !d.cases.contains_key("hot_imaging"));
}

#[test]
fn rename_maneuver_rewrites_the_after_of_later_maneuvers() {
    let snap = snapshot();
    let sites = snap.design.references(Referent::Maneuver("RAISE"));
    assert_eq!(
        sites,
        [RefSite::ManeuverAfter {
            maneuver: "DEORBIT".into()
        }]
    );
    assert_renamed(&snap, "maneuver", "RAISE", "M_RAISE", 1);
    let d = plan(&snap, &[rename_ops("maneuver", "RAISE", "M_RAISE")])
        .unwrap()
        .design;
    assert_eq!(d.maneuvers["DEORBIT"].after.as_deref(), Some("M_RAISE"));
}

/// 出典: 部品の値（状態ごとのセルも）・カタログ品の `source`・`datasheet`・値の出典・要求の `source`。
#[test]
fn rename_source_rewrites_values_catalog_items_and_requirements() {
    let from = Provenance {
        source: Some("ds_vendor".into()),
        at: Some("p.3".into()),
        note: None,
    };
    let snap = applied(
        &snapshot(),
        &[
            Op::SetCatalogField {
                item: "local/cam_ms_5m".into(),
                field: fune_model::design::CatalogField::Datasheet(Some("ds_vendor".into())),
            },
            Op::SetCatalogValue {
                item: "local/cam_ms_5m".into(),
                attr: "mass".into(),
                state: None,
                value: Value::Scalar(Scalar::new(900.0, "g")),
                from: Some(from.clone()),
            },
            Op::SetValue {
                part: "CAM".into(),
                attr: "mass".into(),
                state: None,
                value: Value::Scalar(Scalar::new(910.0, "g")),
                from: Some(from),
            },
        ],
    );
    let sites = snap.design.references(Referent::Source("sys_judgement"));
    assert!(
        sites
            .iter()
            .any(|s| matches!(s, RefSite::PartValueSource { .. })),
        "by-state cells count: {sites:?}"
    );
    assert!(
        sites
            .iter()
            .any(|s| matches!(s, RefSite::RequirementSource { .. }))
    );
    assert_renamed(&snap, "source", "sys_judgement", "sys_judge", sites.len());
    let ds = snap.design.references(Referent::Source("ds_vendor"));
    for kind in [
        |s: &RefSite| matches!(s, RefSite::CatalogItemSource { .. }),
        |s: &RefSite| matches!(s, RefSite::CatalogItemDatasheet { .. }),
        |s: &RefSite| matches!(s, RefSite::CatalogValueSource { .. }),
        |s: &RefSite| matches!(s, RefSite::PartValueSource { .. }),
        |s: &RefSite| matches!(s, RefSite::RequirementSource { .. }),
    ] {
        assert!(ds.iter().any(kind), "{ds:?}");
    }
    assert_renamed(&snap, "source", "ds_vendor", "ds_vend", ds.len());
    // 値・セルの出典の中身（`at` など）は残り、出典の ID だけが替わる
    let d = plan(&snap, &[rename_ops("source", "ds_vendor", "ds_vend")])
        .unwrap()
        .design;
    let from = d.parts["CAM"].attr["mass"].from.as_ref().unwrap();
    assert_eq!(from.source.as_deref(), Some("ds_vend"));
    assert_eq!(from.at.as_deref(), Some("p.3"));
    let cell = &d.parts["HTR_CAM"].attr["power"];
    let Value::ByState(cells) = &cell.value else {
        panic!("{cell:?}")
    };
    assert_eq!(
        cells["on"].from.as_ref().and_then(|f| f.source.as_deref()),
        Some("sys_judgement")
    );
    let d = plan(&snap, &[rename_ops("source", "sys_judgement", "sys_judge")])
        .unwrap()
        .design;
    let Value::ByState(cells) = &d.parts["HTR_CAM"].attr["power"].value else {
        panic!()
    };
    assert_eq!(
        cells["on"].from.as_ref().and_then(|f| f.source.as_deref()),
        Some("sys_judge")
    );
}

#[test]
fn renames_keep_a_source_in_its_directory() {
    let snap = applied(
        &snapshot(),
        &[Op::MoveSource {
            id: "sys_judgement".into(),
            dir: "tier/one".into(),
        }],
    );
    let p = plan(&snap, &[rename_ops("source", "sys_judgement", "sys_judge")]).unwrap();
    assert!(
        p.changes
            .iter()
            .any(|c| c.path == "sources/tier/one/sys_judge.toml" && c.before.is_none())
    );
    no_errors(&p.design);
}

#[test]
fn renames_refuse_taken_invalid_and_missing_ids() {
    let snap = snapshot();
    for (entity, id, taken) in [
        ("net", "P5V", "P12V"),
        ("harness", "H_OBC_CAM", "H_PCDU_RWX"),
        ("case", "hot_imaging", "cold_safe"),
        ("source", "sys_judgement", "ds_vendor"),
        ("maneuver", "RAISE", "DEORBIT"),
    ] {
        let clash = |to: &str| plan(&snap, &[rename_ops(entity, id, to)]);
        // 同じ綴りも、大文字小文字だけが違うものも、既にある ID と重なる
        assert!(
            matches!(clash(taken), Err(OpError::AlreadyExists { .. })),
            "{entity}"
        );
        assert!(
            matches!(
                clash(&taken.to_lowercase()),
                Err(OpError::AlreadyExists { .. })
            ),
            "{entity}"
        );
        assert!(
            matches!(clash("bad id"), Err(OpError::Invalid { .. })),
            "{entity}"
        );
        assert!(
            matches!(clash("_x"), Err(OpError::Invalid { .. })),
            "{entity}"
        );
        assert!(
            matches!(clash(""), Err(OpError::Invalid { .. })),
            "{entity}"
        );
        let missing = plan(&snap, &[rename_ops(entity, "NOPE", "FRESH")]);
        assert!(matches!(missing, Err(OpError::NotFound { .. })), "{entity}");
        assert_eq!(
            to_cli(&rename_ops(entity, id, "fresh")),
            [format!("fune {entity} rename {id} fresh")]
        );
    }
}

/// 軌道の `maneuver`（設計の軌道とフェーズの `[orbit]`）はマヌーバの参照元になる: 名前の変更で両方の `maneuver` が書き換わり、
/// 戻すと元のバイト列になる。参照されているマヌーバを消す Op は、適用後の検査（`ref.maneuver`）が拒む。
#[test]
fn rename_maneuver_rewrites_the_orbit_maneuver_of_the_design_and_of_a_phase() {
    use fune_model::design::{Maneuver, ManeuverTransfer};
    use fune_model::value::SettingValue;
    let mut maneuver: Maneuver = snapshot().design.maneuvers["RAISE"].clone();
    maneuver.id = "MV_MARS".into();
    maneuver.after = None;
    maneuver.thruster = None;
    maneuver.method = fune_model::design::ManeuverMethod::Impulsive;
    maneuver.target.altitude = None;
    maneuver.constraints = Default::default();
    maneuver.target.body = Some(fune_model::design::CentralBodyKind::Mars);
    maneuver.target.perigee_altitude = Some(fune_model::value::Quantity::new(300.0, "km"));
    maneuver.transfer = Some(ManeuverTransfer {
        departure: Some("2028-12-10T00:00:00Z".to_owned()),
        arrival: Some("2029-09-25T00:00:00Z".to_owned()),
        departure_by: None,
        flybys: Vec::new(),
    });
    let text = |v: &str| Some(SettingValue::Text(v.to_owned()));
    let snap = applied(
        &snapshot(),
        &[
            Op::SetManeuver { maneuver },
            Op::SetSetting {
                path: "conditions.orbit.maneuver".into(),
                value: text("MV_MARS"),
            },
            Op::SetPhase {
                phase: fune_model::design::Phase {
                    id: "CRUISE".into(),
                    ..Default::default()
                },
            },
            Op::SetPhaseField {
                phase: "CRUISE".into(),
                path: "orbit.maneuver".into(),
                value: text("MV_MARS"),
            },
        ],
    );
    let sites = snap.design.references(Referent::Maneuver("MV_MARS"));
    assert_eq!(
        sites,
        [
            RefSite::OrbitManeuver,
            RefSite::PhaseOrbitManeuver {
                phase: "CRUISE".into()
            }
        ]
    );
    let renamed = plan(&snap, &[rename_ops("maneuver", "MV_MARS", "MV_X")])
        .unwrap()
        .design;
    assert_eq!(
        renamed
            .conditions
            .orbit
            .as_ref()
            .unwrap()
            .maneuver
            .as_deref(),
        Some("MV_X")
    );
    assert_eq!(
        renamed.phases["CRUISE"]
            .orbit
            .as_ref()
            .unwrap()
            .maneuver
            .as_deref(),
        Some("MV_X")
    );
    assert!(renamed.references(Referent::Maneuver("MV_MARS")).is_empty());
    // 名前を戻すと元のバイト列
    let back = applied(
        &applied(&snap, &[rename_ops("maneuver", "MV_MARS", "MV_X")]),
        &[rename_ops("maneuver", "MV_X", "MV_MARS")],
    );
    assert!(plan_between(&snap.files, &back.files).is_empty());
    // 参照されているマヌーバは消せない（`ref.maneuver` が新しい誤りになる）
    let removed = plan(
        &snap,
        &[Op::RemoveManeuver {
            id: "MV_MARS".into(),
        }],
    );
    match removed {
        Err(OpError::Invalid { message }) => {
            assert!(
                message.starts_with("Maneuver `MV_MARS` cannot be removed: referred to by "),
                "{message}"
            );
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}
