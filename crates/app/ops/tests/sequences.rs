//! モードの並び（名前付き・従来の `[[sequence]]`）とモードの時刻の指定の編集（Op → ファイルの差分）。
//! fixtures/demo-6u に対して計画を立て、書かれた本文・読み戻した設計・正規形・参照の書き換えを確かめる。

use fune_model::check;
use fune_model::design::{ModeSegment, ModeWhen, StartFrom, StartRef};
use fune_model::value::{Quantity, SettingValue};
use fune_ops::{Op, OpError, plan};
use fune_store::format::format_text;
use fune_store::{Snapshot, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

fn min(v: f64) -> Quantity {
    Quantity::new(v, "min")
}

fn seg(mode: &str, minutes: f64) -> ModeSegment {
    ModeSegment {
        mode: mode.into(),
        duration: min(minutes),
    }
}

fn imaging_day() -> Op {
    Op::SetSequence {
        name: "extra_day".into(),
        anchor: Some(StartRef {
            from: StartFrom::EclipseExit,
            offset: Some(min(5.0)),
        }),
        segments: vec![
            seg("NOMINAL", 76.0),
            seg("MISSION", 5.0),
            seg("DOWNLINK", 9.6),
        ],
    }
}

fn text_of(p: &fune_ops::Plan, path: &str) -> String {
    p.changes
        .iter()
        .find(|c| c.path == path)
        .and_then(|c| c.after.clone())
        .unwrap_or_else(|| panic!("{path} not changed"))
}

/// 書かれた本文が正規形（`fune fmt --check` に通る）で、設計の検査に誤りが無い。
fn assert_clean(p: &fune_ops::Plan) {
    for c in &p.changes {
        if let Some(after) = &c.after {
            assert_eq!(&format_text(&c.path, after).unwrap(), after, "{}", c.path);
        }
    }
    assert!(check(&p.design).iter().all(|d| !d.is_error()));
}

#[test]
fn set_sequence_writes_a_named_table_next_to_the_legacy_one() {
    let p = plan(&snapshot(), &[imaging_day()]).unwrap();
    let text = text_of(&p, "analysis/power.toml");
    assert!(text.contains("[sequences.extra_day]"), "{text}");
    assert!(
        text.contains(
            r#"anchor = { from = "eclipse_exit", offset = { value = 5.0, unit = "min" } }"#
        ),
        "{text}"
    );
    assert!(text.contains("[[sequences.extra_day.segments]]"), "{text}");
    // 従来の `[[sequence]]` はそのまま（デモの名前付きの並びとも並ぶ）
    assert_eq!(text.matches("[[sequence]]").count(), 4);
    assert!(text.contains("[sequences.imaging]"));
    assert_clean(&p);
    let power = p.design.analysis.power.as_ref().unwrap();
    let view = power.sequence("extra_day").unwrap();
    assert_eq!(view.segments.len(), 3);
    assert_eq!(view.anchor.unwrap().from, StartFrom::EclipseExit);
    // 並びの名前は default（従来の並び）が最初、使う並びは default のまま
    assert_eq!(
        power.sequence_names(),
        ["default", "downlink", "extra_day", "imaging"]
    );
    assert_eq!(power.active_sequence_name(), Some("default"));
}

#[test]
fn writing_default_migrates_the_legacy_sequence() {
    let op = Op::SetSequence {
        name: "default".into(),
        anchor: None,
        segments: vec![seg("NOMINAL", 90.0), seg("DOWNLINK", 5.6)],
    };
    let p = plan(&snapshot(), &[op]).unwrap();
    let text = text_of(&p, "analysis/power.toml");
    assert!(!text.contains("[[sequence]]"), "{text}");
    assert!(text.contains("[[sequences.default.segments]]"), "{text}");
    assert_clean(&p);
    let power = p.design.analysis.power.as_ref().unwrap();
    assert!(power.sequence.is_empty());
    assert!(!power.sequence("default").unwrap().legacy);
}

#[test]
fn the_active_sequence_is_chosen_by_a_setting_and_removal_clears_it() {
    let snap = snapshot();
    let ops = [
        imaging_day(),
        Op::SetSetting {
            path: "analysis.power.active_sequence".into(),
            value: Some(SettingValue::Text("extra_day".into())),
        },
    ];
    let p = plan(&snap, &ops).unwrap();
    assert_clean(&p);
    let power = p.design.analysis.power.as_ref().unwrap();
    assert_eq!(power.active_sequence_name(), Some("extra_day"));
    // 使う並びを消せば選択も外れる（残すと検査の誤りになる）
    let mut ops = ops.to_vec();
    ops.push(Op::RemoveSequence {
        name: "extra_day".into(),
    });
    let p = plan(&snap, &ops).unwrap();
    assert_clean(&p);
    let power = p.design.analysis.power.as_ref().unwrap();
    assert_eq!(power.active_sequence, None);
    assert_eq!(power.active_sequence_name(), Some("default"));
    // 足して消したので、ファイルは元に戻る
    assert!(p.changes.iter().all(|c| c.path != "analysis/power.toml"));
}

#[test]
fn a_missing_active_sequence_is_a_check_error() {
    let op = Op::SetSetting {
        path: "analysis.power.active_sequence".into(),
        value: Some(SettingValue::Text("nope".into())),
    };
    let OpError::Rejected { diagnostics } = plan(&snapshot(), &[op]).unwrap_err() else {
        panic!("expected a rejection");
    };
    assert!(
        diagnostics.iter().any(|d| d.code == "ref.sequence"),
        "{diagnostics:?}"
    );
}

#[test]
fn renaming_a_mode_rewrites_named_sequences() {
    let ops = [
        imaging_day(),
        Op::RenameMode {
            id: "MISSION".into(),
            to: "IMAGING".into(),
        },
    ];
    let p = plan(&snapshot(), &ops).unwrap();
    let text = text_of(&p, "analysis/power.toml");
    let named = text.split("[sequences.extra_day]").nth(1).unwrap();
    assert!(
        named.contains(r#""IMAGING""#) && !named.contains("MISSION"),
        "{text}"
    );
    // 従来の並びも書き換わる
    let power = p.design.analysis.power.as_ref().unwrap();
    assert!(power.sequence.iter().any(|s| s.mode == "IMAGING"));
    assert!(
        power.sequences["extra_day"]
            .segments
            .iter()
            .any(|s| s.mode == "IMAGING")
    );
    assert_clean(&p);
}

#[test]
fn invalid_sequences_are_rejected() {
    let snap = snapshot();
    let bad = |name: &str, segments: Vec<ModeSegment>| {
        plan(
            &snap,
            &[Op::SetSequence {
                name: name.into(),
                anchor: None,
                segments,
            }],
        )
        .unwrap_err()
    };
    assert!(matches!(bad("x", vec![]), OpError::Invalid { .. }));
    assert!(matches!(
        bad("x", vec![seg("NOPE", 5.0)]),
        OpError::NotFound { .. }
    ));
    assert!(matches!(
        bad("not a name", vec![seg("NOMINAL", 5.0)]),
        OpError::Invalid { .. }
    ));
    // 長さの次元は検査が見る
    let err = bad(
        "x",
        vec![ModeSegment {
            mode: "NOMINAL".into(),
            duration: Quantity::new(5.0, "kg"),
        }],
    );
    assert!(matches!(err, OpError::Rejected { .. }), "{err}");
    let err = plan(
        &snap,
        &[Op::RemoveSequence {
            name: "nope".into(),
        }],
    )
    .unwrap_err();
    assert!(matches!(err, OpError::NotFound { .. }));
}

#[test]
fn mode_timing_is_written_and_cleared() {
    let snap = snapshot();
    let set = Op::SetModeTiming {
        mode: "DOWNLINK".into(),
        when: Some(ModeWhen::Sunlit),
        start: Some(StartRef {
            from: StartFrom::EclipseExit,
            offset: Some(min(5.0)),
        }),
    };
    let p = plan(&snap, std::slice::from_ref(&set)).unwrap();
    let text = text_of(&p, "modes/DOWNLINK.toml");
    assert!(
        text.contains(r#"when         = "sunlit""#) || text.contains(r#"when = "sunlit""#),
        "{text}"
    );
    assert!(
        text.contains(
            r#"start = { from = "eclipse_exit", offset = { value = 5.0, unit = "min" } }"#
        ) || text.contains("eclipse_exit"),
        "{text}"
    );
    assert_clean(&p);
    let m = &p.design.modes["DOWNLINK"];
    assert_eq!(m.when, Some(ModeWhen::Sunlit));
    assert_eq!(m.start.as_ref().unwrap().from, StartFrom::EclipseExit);
    // `any` は書かない（指定なしと同じ）。両方 None なら消える
    let any = Op::SetModeTiming {
        mode: "DOWNLINK".into(),
        when: Some(ModeWhen::Any),
        start: None,
    };
    let p2 = plan(&snap, &[set.clone(), any]).unwrap();
    assert!(!p2.design.modes["DOWNLINK"].when.is_some());
    assert!(p2.design.modes["DOWNLINK"].start.is_none());
    let clear = Op::SetModeTiming {
        mode: "DOWNLINK".into(),
        when: None,
        start: None,
    };
    let p3 = plan(&snap, &[set, clear]).unwrap();
    assert!(p3.changes.iter().all(|c| c.path != "modes/DOWNLINK.toml"));
    assert!(matches!(
        plan(
            &snap,
            &[Op::SetModeTiming {
                mode: "NOPE".into(),
                when: None,
                start: None
            }]
        )
        .unwrap_err(),
        OpError::NotFound { .. }
    ));
}

#[test]
fn copying_a_mode_copies_its_timing() {
    let ops = [
        Op::SetModeTiming {
            mode: "DOWNLINK".into(),
            when: Some(ModeWhen::Eclipse),
            start: None,
        },
        Op::CreateMode {
            id: "DOWNLINK2".into(),
            label: None,
            description: None,
            copy_from: Some("DOWNLINK".into()),
        },
    ];
    let p = plan(&snapshot(), &ops).unwrap();
    assert_eq!(p.design.modes["DOWNLINK2"].when, Some(ModeWhen::Eclipse));
    assert_clean(&p);
}
