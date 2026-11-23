//! フェーズの検査（`ref.mode`・`ref.phase`・`unit.dimension`・`case.mode_not_in_phase`・`id.invalid`・軌道の検査）。
use fune_model::design::{AttitudeLawKind, AttitudeLawSetting, Case, Mode, OrbitSettings, Phase};
use fune_model::value::Quantity;
use fune_model::{Design, Diagnostic, Severity, Target};

fn mode(id: &str) -> Mode {
    Mode {
        id: id.into(),
        ..Mode::default()
    }
}

fn case(id: &str, mode: Option<&str>, phase: Option<&str>) -> Case {
    Case {
        id: id.into(),
        label: None,
        description: None,
        mode: mode.map(Into::into),
        phase: phase.map(Into::into),
        side: None,
        thermal_case: None,
        beta: None,
        orbits: None,
        sun_distance: None,
    }
}

/// モード SAFE・NOMINAL と、フェーズ 1 つ（`edit` で書き換える）・ケース `edit_case` を持つ設計の診断（info を除く）。
fn diagnostics(edit: impl FnOnce(&mut Phase), cases: Vec<Case>) -> Vec<Diagnostic> {
    let mut d = Design::default();
    for m in ["SAFE", "NOMINAL"] {
        d.modes.insert(m.into(), mode(m));
    }
    let mut p = Phase {
        id: "LEOP".into(),
        modes: vec!["SAFE".into(), "NOMINAL".into()],
        duration: Some(Quantity::new(30.0, "d")),
        ..Phase::default()
    };
    edit(&mut p);
    d.phases.insert(p.id.clone(), p);
    for c in cases {
        d.cases.insert(c.id.clone(), c);
    }
    fune_model::check(&d)
        .into_iter()
        .filter(|d| d.severity != Severity::Info)
        .collect()
}

fn find<'a>(ds: &'a [Diagnostic], code: &str) -> Option<&'a Diagnostic> {
    ds.iter().find(|d| d.code == code)
}

#[test]
fn a_well_formed_phase_has_no_problems() {
    let ds = diagnostics(
        |_| {},
        vec![
            case("c1", Some("NOMINAL"), Some("LEOP")),
            case("c2", None, None),
        ],
    );
    assert!(ds.is_empty(), "{ds:?}");
}

#[test]
fn modes_and_laws_must_exist() {
    let ds = diagnostics(
        |p| {
            p.modes.push("GHOST".into());
            p.law.insert(
                "GHOST2".into(),
                AttitudeLawSetting {
                    law: AttitudeLawKind::Inertial,
                    axis: None,
                    secondary: None,
                    along_track: None,
                    orientation: None,
                },
            );
        },
        vec![],
    );
    let targets: Vec<String> = ds
        .iter()
        .filter(|d| d.code == "ref.mode")
        .filter_map(|d| d.target.as_ref().map(ToString::to_string))
        .collect();
    assert_eq!(targets, ["phases.LEOP.modes", "phases.LEOP.law.GHOST2"]);
    assert!(ds.iter().all(|d| d.severity == Severity::Error));
}

#[test]
fn duration_has_a_time_dimension() {
    let ds = diagnostics(|p| p.duration = Some(Quantity::new(30.0, "kg")), vec![]);
    let d = find(&ds, "unit.dimension").expect("dimension error");
    assert_eq!(d.target, Some(Target::phase("LEOP", Some("duration"))));
}

#[test]
fn the_case_phase_must_exist_and_its_mode_should_be_listed() {
    let ds = diagnostics(|_| {}, vec![case("c1", None, Some("GHOST"))]);
    let d = find(&ds, "ref.phase").expect("ref.phase");
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.target, Some(Target::case("c1", Some("phase"))));

    // フェーズが挙げるモードに無いモードは警告（`modes` が空なら何も言わない）
    let listed = |p: &mut Phase| p.modes = vec!["SAFE".into()];
    let ds = diagnostics(listed, vec![case("c1", Some("NOMINAL"), Some("LEOP"))]);
    let d = find(&ds, "case.mode_not_in_phase").expect("warning");
    assert_eq!(d.severity, Severity::Warning);
    let ds = diagnostics(
        |p| p.modes.clear(),
        vec![case("c1", Some("NOMINAL"), Some("LEOP"))],
    );
    assert!(ds.is_empty(), "{ds:?}");
}

#[test]
fn the_phase_id_follows_the_id_rules() {
    let mut d = Design::default();
    d.phases.insert(
        "bad id".into(),
        Phase {
            id: "bad id".into(),
            ..Phase::default()
        },
    );
    d.phases.insert(
        "A".into(),
        Phase {
            id: "A".into(),
            ..Phase::default()
        },
    );
    d.phases.insert(
        "a".into(),
        Phase {
            id: "a".into(),
            ..Phase::default()
        },
    );
    let ds = fune_model::check(&d);
    assert!(find(&ds, "id.invalid").is_some(), "{ds:?}");
    assert!(find(&ds, "id.duplicate").is_some(), "{ds:?}");
}

/// `[orbit]` は設計の軌道と同じ検査（次元・離心率の範囲・地方時の書き方）で、対象と位置はフェーズのファイル。
#[test]
fn the_phase_orbit_gets_the_orbit_checks() {
    let orbit = OrbitSettings {
        altitude: Some(Quantity::new(550.0, "W")),
        eccentricity: Some(1.5),
        ltdn: Some("10.5".into()),
        ..OrbitSettings::default()
    };
    let ds = diagnostics(|p| p.orbit = Some(orbit), vec![]);
    let at = |code: &str| {
        find(&ds, code)
            .and_then(|d| d.target.as_ref().map(ToString::to_string))
            .unwrap_or_default()
    };
    assert_eq!(at("unit.dimension"), "phases.LEOP.orbit.altitude");
    assert_eq!(at("setting.range"), "phases.LEOP.orbit.eccentricity");
    assert_eq!(at("setting.format"), "phases.LEOP.orbit.ltdn");
    assert!(ds.iter().all(|d| {
        d.target
            .as_ref()
            .is_some_and(|t| t.to_string().starts_with("phases.LEOP"))
    }));
}
