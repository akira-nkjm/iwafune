//! フェーズの Op（fixtures/demo-6u に対して計画だけを立てる）: 足す・書く・消す・名前を変える・軌道と姿勢則の値・ケースのフェーズ。

use fune_model::design::{Phase, RefSite, Referent};
use fune_model::value::{Quantity, SettingValue};
use fune_ops::{Op, OpError, Plan, plan, to_cli};
use fune_store::{Snapshot, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

fn phase(id: &str) -> Phase {
    Phase {
        id: id.into(),
        label: Some("試験".into()),
        order: 5,
        start: Some("Start".into()),
        end: None,
        duration: Some(Quantity::new(10.0, "d")),
        modes: vec!["SAFE".into(), "NOMINAL".into()],
        configuration: None,
        orbit: None,
        law: Default::default(),
    }
}

fn after_of<'a>(p: &'a Plan, file: &str) -> &'a str {
    p.changes
        .iter()
        .find(|c| c.path == file)
        .and_then(|c| c.after.as_deref())
        .unwrap_or_default()
}

fn field(phase: &str, path: &str, value: Option<SettingValue>) -> Op {
    Op::SetPhaseField {
        phase: phase.into(),
        path: path.into(),
        value,
    }
}

#[test]
fn phases_are_created_once_and_written_in_canonical_form() {
    let snap = snapshot();
    let op = Op::CreatePhase { phase: phase("T1") };
    let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
    assert_eq!(
        after_of(&p, "phases/T1.toml"),
        "label    = \"試験\"\norder    = 5\nstart    = \"Start\"\nmodes    = [\"SAFE\", \"NOMINAL\"]\nduration = { value = 10.0, unit = \"d\" }\n"
    );
    assert_eq!(
        to_cli(&op),
        [
            "fune phase add T1 --label \"試験\" --order 5 --start Start --duration \"10 d\" --modes SAFE,NOMINAL"
        ]
    );
    let twice = plan(&snap, &[op.clone(), op]);
    assert!(
        matches!(twice, Err(OpError::AlreadyExists { .. })),
        "{twice:?}"
    );
    let set = Op::SetPhase { phase: phase("T1") };
    assert!(plan(&snap, &[set.clone(), set]).is_ok());
    // 名前の規則
    let bad = plan(
        &snap,
        &[Op::CreatePhase {
            phase: phase("bad id"),
        }],
    );
    assert!(matches!(bad, Err(OpError::Invalid { .. })), "{bad:?}");
}

/// `SetPhase` で軌道と姿勢則を空にして送っても、今の `[orbit]`・`[law]` は残る。
#[test]
fn set_phase_keeps_the_orbit_and_law() {
    let snap = snapshot();
    let ops = [
        Op::CreatePhase { phase: phase("T1") },
        field(
            "T1",
            "orbit.altitude",
            Some(SettingValue::Quantity(Quantity::new(400.0, "km"))),
        ),
        field(
            "T1",
            "law.NOMINAL.law",
            Some(SettingValue::Text("nadir_pointing".into())),
        ),
        Op::SetPhase {
            phase: Phase {
                label: Some("変更".into()),
                ..phase("T1")
            },
        },
    ];
    let p = plan(&snap, &ops).unwrap();
    let t1 = &p.design.phases["T1"];
    assert_eq!(t1.label.as_deref(), Some("変更"));
    assert_eq!(
        t1.orbit.as_ref().and_then(|o| o.altitude.clone()),
        Some(Quantity::new(400.0, "km"))
    );
    assert_eq!(t1.law.len(), 1);
    let text = after_of(&p, "phases/T1.toml");
    assert!(
        text.contains("[orbit]") && text.contains("[law.NOMINAL]"),
        "{text}"
    );
}

#[test]
fn phase_fields_write_nested_values_and_remove_empty_tables() {
    let snap = snapshot();
    let create = Op::CreatePhase { phase: phase("T1") };
    let set = field(
        "T1",
        "orbit.altitude",
        Some(SettingValue::Quantity(Quantity::new(400.0, "km"))),
    );
    let p = plan(&snap, &[create.clone(), set.clone()]).unwrap();
    assert!(
        after_of(&p, "phases/T1.toml")
            .ends_with("\n[orbit]\naltitude = { value = 400.0, unit = \"km\" }\n"),
        "{}",
        after_of(&p, "phases/T1.toml")
    );
    assert_eq!(
        to_cli(&set),
        ["fune phase set-field T1 orbit.altitude \"400 km\""]
    );
    // 値を消して `[orbit]` が空になれば、表ごと消える（作る前と同じ本文）
    let unset = field("T1", "orbit.altitude", None);
    let back = plan(&snap, &[create.clone(), set, unset]).unwrap();
    let plain = plan(&snap, &[create]).unwrap();
    assert_eq!(
        after_of(&back, "phases/T1.toml"),
        after_of(&plain, "phases/T1.toml")
    );
    // パスは `orbit.` か `law.<モード>.` で始まる。フェーズが無ければ拒む
    for path in ["label", "orbit", "law.NOMINAL", "modes.x"] {
        let bad = plan(
            &snapshot(),
            &[
                Op::CreatePhase { phase: phase("T1") },
                field("T1", path, Some(SettingValue::Text("x".into()))),
            ],
        );
        assert!(
            matches!(bad, Err(OpError::Invalid { .. })),
            "{path}: {bad:?}"
        );
    }
    let missing = plan(
        &snap,
        &[field(
            "NOPE",
            "orbit.altitude",
            Some(SettingValue::Number(1.0)),
        )],
    );
    assert!(
        matches!(missing, Err(OpError::NotFound { .. })),
        "{missing:?}"
    );
}

/// 参照の検査: 無いモードを挙げる・無いモードの姿勢則・無いフェーズを指すケースは誤り。
#[test]
fn unknown_references_are_rejected() {
    let snap = snapshot();
    let mut p = phase("T1");
    p.modes = vec!["NO_SUCH_MODE".into()];
    let bad = plan(&snap, &[Op::CreatePhase { phase: p }]);
    assert!(matches!(bad, Err(OpError::Rejected { .. })), "{bad:?}");
    let bad = plan(
        &snap,
        &[
            Op::CreatePhase { phase: phase("T1") },
            field(
                "T1",
                "law.NO_SUCH_MODE.law",
                Some(SettingValue::Text("inertial".into())),
            ),
        ],
    );
    assert!(matches!(bad, Err(OpError::Rejected { .. })), "{bad:?}");
    let bad = plan(
        &snap,
        &[Op::SetCasePhase {
            case: "hot_imaging".into(),
            phase: Some("NO_SUCH_PHASE".into()),
        }],
    );
    assert!(matches!(bad, Err(OpError::Rejected { .. })), "{bad:?}");
    let bad = plan(
        &snap,
        &[Op::SetCasePhase {
            case: "NO_SUCH_CASE".into(),
            phase: None,
        }],
    );
    assert!(matches!(bad, Err(OpError::NotFound { .. })), "{bad:?}");
}

#[test]
fn a_case_phase_is_set_and_cleared() {
    let snap = snapshot();
    let create = Op::CreatePhase { phase: phase("T1") };
    let set = Op::SetCasePhase {
        case: "hot_imaging".into(),
        phase: Some("T1".into()),
    };
    let p = plan(&snap, &[create.clone(), set.clone()]).unwrap();
    assert_eq!(
        p.design.cases["hot_imaging"]
            .phase
            .as_ref()
            .map(|x| x.as_str()),
        Some("T1")
    );
    assert!(
        p.design
            .references(Referent::Phase("T1"))
            .contains(&RefSite::CasePhase {
                case: "hot_imaging".into()
            })
    );
    assert_eq!(to_cli(&set), ["fune case set-phase hot_imaging T1"]);
    let clear = Op::SetCasePhase {
        case: "hot_imaging".into(),
        phase: None,
    };
    assert_eq!(to_cli(&clear), ["fune case unset-phase hot_imaging"]);
    let p = plan(&snap, &[create, set, clear]).unwrap();
    assert!(p.design.cases["hot_imaging"].phase.is_none());
}

/// 名前の変更はケースの `phase` を書き換え、モードの名前の変更はフェーズの `modes` と `[law]` のキーを書き換える。
/// フェーズを消すとき、ケースが指していれば拒む（モードを参照しているケースを消せないのと同じ）。
#[test]
fn renames_and_removals_follow_the_references() {
    let snap = snapshot();
    let mut t1 = phase("T1");
    t1.modes = vec!["NOMINAL".into()];
    let setup = vec![
        Op::CreatePhase { phase: t1 },
        field(
            "T1",
            "law.NOMINAL.law",
            Some(SettingValue::Text("inertial".into())),
        ),
        Op::SetCasePhase {
            case: "hot_imaging".into(),
            phase: Some("T1".into()),
        },
    ];
    let with = |op: Op| [setup.clone(), vec![op]].concat();
    // フェーズの名前
    let rename = Op::RenamePhase {
        id: "T1".into(),
        to: "T2".into(),
    };
    let p = plan(&snap, &with(rename.clone())).unwrap();
    assert!(p.design.phases.contains_key("T2") && !p.design.phases.contains_key("T1"));
    assert_eq!(
        p.design.cases["hot_imaging"]
            .phase
            .as_ref()
            .map(|x| x.as_str()),
        Some("T2")
    );
    assert_eq!(to_cli(&rename), ["fune phase rename T1 T2"]);
    let clash = plan(
        &snap,
        &with(Op::RenamePhase {
            id: "T1".into(),
            to: "t1".into(),
        }),
    );
    assert!(
        matches!(clash, Err(OpError::AlreadyExists { .. })),
        "{clash:?}"
    );
    let missing = plan(
        &snap,
        &[Op::RenamePhase {
            id: "NOPE".into(),
            to: "X".into(),
        }],
    );
    assert!(
        matches!(missing, Err(OpError::NotFound { .. })),
        "{missing:?}"
    );
    // モードの名前
    let p = plan(
        &snap,
        &with(Op::RenameMode {
            id: "NOMINAL".into(),
            to: "CRUISE".into(),
        }),
    )
    .unwrap();
    let t1 = &p.design.phases["T1"];
    assert_eq!(
        t1.modes.iter().map(|m| m.as_str()).collect::<Vec<_>>(),
        ["CRUISE"]
    );
    assert!(t1.law.contains_key("CRUISE") && !t1.law.contains_key("NOMINAL"));
    assert!(
        p.design
            .references(Referent::Mode("CRUISE"))
            .iter()
            .any(|s| matches!(s, RefSite::PhaseMode { .. }))
    );
    // 消す: ケースが指していれば拒み、外してからなら消せる
    let blocked = plan(&snap, &with(Op::RemovePhase { id: "T1".into() }));
    let Err(OpError::Rejected { diagnostics }) = blocked else {
        panic!("not rejected");
    };
    assert!(diagnostics.iter().any(|d| d.code == "ref.phase"));
    let p = plan(
        &snap,
        &with(Op::SetCasePhase {
            case: "hot_imaging".into(),
            phase: None,
        }),
    )
    .unwrap();
    assert!(p.design.phases.contains_key("T1"));
    let removed = [
        setup.clone(),
        vec![
            Op::SetCasePhase {
                case: "hot_imaging".into(),
                phase: None,
            },
            Op::RemovePhase { id: "T1".into() },
        ],
    ]
    .concat();
    let p = plan(&snap, &removed).unwrap();
    assert!(!p.design.phases.contains_key("T1"));
    assert_eq!(
        to_cli(&Op::RemovePhase { id: "T1".into() }),
        ["fune phase rm T1"]
    );
}

/// 設計の軌道を写し（1 手）、欄を書き換え、丸ごと消す。どの段階でも読み直せる（適用後の検査が通る）。
#[test]
fn the_design_orbit_is_copied_into_a_phase_and_removed_whole() {
    let snap = snapshot();
    let create = Op::CreatePhase { phase: phase("T1") };
    let copy = Op::CopyPhaseOrbit { phase: "T1".into() };
    let p = plan(&snap, &[create.clone(), copy.clone()]).unwrap();
    assert_eq!(p.design.phases["T1"].orbit, snap.design.conditions.orbit);
    let text = after_of(&p, "phases/T1.toml");
    assert!(
        text.contains("[orbit]") && text.contains("sun_synchronous = true"),
        "{text}"
    );
    assert_eq!(to_cli(&copy), ["fune phase copy-orbit T1"]);
    // 写したあとに欄を書き換えられる。もう一度写すと設計の軌道に戻る
    let edited = field(
        "T1",
        "orbit.altitude",
        Some(SettingValue::Quantity(Quantity::new(400.0, "km"))),
    );
    let ops = [create.clone(), copy.clone(), edited, copy];
    let reset = plan(&snap, &ops).unwrap();
    assert_eq!(
        reset.design.phases["T1"].orbit,
        snap.design.conditions.orbit
    );
    // 丸ごと消すと、軌道を持たない同じフェーズに戻る（ほかの欄は残る）
    let unset = Op::UnsetPhaseOrbit { phase: "T1".into() };
    let back = plan(&snap, &[create.clone(), ops[1].clone(), unset.clone()]).unwrap();
    let plain = plan(&snap, &[create]).unwrap();
    assert_eq!(
        after_of(&back, "phases/T1.toml"),
        after_of(&plain, "phases/T1.toml")
    );
    assert_eq!(to_cli(&unset), ["fune phase unset-orbit T1"]);
    // 無いフェーズ・設計に軌道が無いときは拒む
    let missing = plan(
        &snap,
        &[Op::CopyPhaseOrbit {
            phase: "NOPE".into(),
        }],
    );
    assert!(
        matches!(missing, Err(OpError::NotFound { .. })),
        "{missing:?}"
    );
    let mut bare = snapshot();
    bare.design.conditions.orbit = None;
    let none = plan(
        &bare,
        &[Op::CopyPhaseOrbit {
            phase: "DISPOSAL".into(),
        }],
    );
    assert!(matches!(none, Err(OpError::Invalid { .. })), "{none:?}");
}

/// 姿勢則の上書きは、種類を先に書けば作れ（どの段階でも読み直せる）、モードごとに丸ごと消せる。
#[test]
fn a_law_override_is_created_by_its_kind_and_removed_whole() {
    let snap = snapshot();
    let ops = [
        Op::CreatePhase { phase: phase("T1") },
        field(
            "T1",
            "law.NOMINAL.law",
            Some(SettingValue::Text("sun_pointing".into())),
        ),
        field(
            "T1",
            "law.NOMINAL.axis",
            Some(SettingValue::Vector(fune_model::value::Vector3::unitless(
                0.0, 0.0, 1.0,
            ))),
        ),
        field(
            "T1",
            "law.SAFE.law",
            Some(SettingValue::Text("inertial".into())),
        ),
    ];
    let p = plan(&snap, &ops).unwrap();
    assert_eq!(p.design.phases["T1"].law.len(), 2);
    let unset = Op::UnsetPhaseLaw {
        phase: "T1".into(),
        mode: "NOMINAL".into(),
    };
    let mut all = ops.to_vec();
    all.push(unset.clone());
    let p = plan(&snap, &all).unwrap();
    let law = &p.design.phases["T1"].law;
    assert_eq!(law.keys().collect::<Vec<_>>(), ["SAFE"]);
    assert_eq!(to_cli(&unset), ["fune phase unset-law T1 NOMINAL"]);
    // 上書きの無いモードは拒む
    let again = plan(&snap, &[ops[0].clone(), unset]);
    assert!(matches!(again, Err(OpError::NotFound { .. })), "{again:?}");
}
