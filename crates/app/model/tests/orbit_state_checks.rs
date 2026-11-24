//! 軌道の出どころ `state`（状態ベクトル）の検査: 欄が正しく書けて読めること、状態の欄を別の出どころに添えると
//! `setting.not_applied` の警告、状態の軌道に添えた要素の欄も警告、座標系の欄・非有限・非正の期間は誤り、次元が違う単位は
//! 設定の次元の表で誤り。足りない欄は検査の誤りにしない（射影が足りない入力として知らせる）。
//! 設計の軌道（`conditions/orbit.toml`）とフェーズの `[orbit]` で同じ検査。
use fune_model::design::{CentralBodyKind, OrbitSettings, OrbitSource, Phase};
use fune_model::value::{Quantity, Vector3};
use fune_model::{Design, Diagnostic, Severity, check};

fn vector(x: f64, y: f64, z: f64, unit: &str) -> Vector3 {
    Vector3 {
        x,
        y,
        z,
        unit: Some(unit.to_owned()),
        frame: None,
    }
}

/// 1 au の円軌道（太陽中心）の状態ベクトル。
fn state() -> OrbitSettings {
    OrbitSettings {
        source: Some(OrbitSource::State),
        epoch: Some("2030-01-01T00:00:00Z".to_owned()),
        central: Some(CentralBodyKind::Sun),
        position: Some(vector(1.495_978_707e8, 0.0, 0.0, "km")),
        velocity: Some(vector(0.0, 29.78, 0.0, "km/s")),
        span: Some(Quantity::new(365.0, "d")),
        ..OrbitSettings::default()
    }
}

fn design_orbit(o: OrbitSettings) -> Design {
    let mut d = Design::default();
    d.conditions.orbit = Some(o);
    d
}

fn phase_orbit(o: OrbitSettings) -> Design {
    let mut d = Design::default();
    d.phases.insert(
        "CRUISE".into(),
        Phase {
            id: "CRUISE".into(),
            orbit: Some(o),
            ..Phase::default()
        },
    );
    d
}

/// 軌道の欄に関わる診断（コード・重大度・対象）。
fn found(d: &Design) -> Vec<(String, Severity, String)> {
    check(d)
        .into_iter()
        .filter(|x: &Diagnostic| {
            x.target
                .as_ref()
                .is_some_and(|t| t.to_string().contains("orbit"))
        })
        .map(|x| {
            (
                x.code,
                x.severity,
                x.target.map(|t| t.to_string()).unwrap_or_default(),
            )
        })
        .collect()
}

/// 書けた状態の軌道は診断が無い（設計の軌道でもフェーズの軌道でも）。欄が足りなくても検査の誤りにはしない。
#[test]
fn a_complete_state_orbit_is_clean_and_missing_keys_are_not_check_errors() {
    assert!(found(&design_orbit(state())).is_empty());
    assert!(found(&phase_orbit(state())).is_empty());
    let only_source = OrbitSettings {
        source: Some(OrbitSource::State),
        ..OrbitSettings::default()
    };
    assert!(found(&design_orbit(only_source)).is_empty());
}

/// 状態の欄（`position`・`velocity`・`span`）を `elements`（書かない）や `transfer` に添えると使われない（警告）。
#[test]
fn state_keys_beside_another_source_are_warned() {
    let mut elements = state();
    elements.source = None;
    elements.central = None;
    elements.epoch = None;
    let ds = found(&design_orbit(elements.clone()));
    let targets: Vec<&str> = ds.iter().map(|d| d.2.as_str()).collect();
    assert_eq!(ds.len(), 3, "{ds:?}");
    for key in ["position", "velocity", "span"] {
        assert!(
            targets.contains(&format!("conditions.orbit.{key}").as_str()),
            "{targets:?}"
        );
    }
    assert!(
        ds.iter()
            .all(|(c, s, _)| c == "setting.not_applied" && *s == Severity::Warning)
    );
    // フェーズの軌道は対象がフェーズのファイル
    let ds = found(&phase_orbit(elements));
    assert!(ds.iter().all(|d| d.2.starts_with("phases.CRUISE.orbit.")));
    // `transfer` では `epoch`・`central` も使わない
    let mut transfer = state();
    transfer.source = Some(OrbitSource::Transfer);
    assert_eq!(found(&design_orbit(transfer)).len(), 5);
}

/// 状態の軌道に添えた要素の欄は使われない（警告）。`epoch`・`central`・`label` は使う。
#[test]
fn element_keys_beside_a_state_source_are_warned_but_epoch_and_central_are_used() {
    let mut o = state();
    o.label = Some("Cruise".to_owned());
    o.altitude = Some(Quantity::new(550.0, "km"));
    o.maneuver = Some("X".to_owned());
    let ds = found(&design_orbit(o));
    let targets: Vec<&str> = ds
        .iter()
        .filter(|d| d.0 == "setting.not_applied")
        .map(|d| d.2.as_str())
        .collect();
    assert_eq!(
        targets.len(),
        2,
        "{ds:?}: altitude and maneuver only (epoch, central, label are used)"
    );
    assert!(targets.contains(&"conditions.orbit.altitude"));
    assert!(targets.contains(&"conditions.orbit.maneuver"));
}

/// 座標系（`frame`）は書けない（ECI の 1 つだけ）。非有限の成分と、正でない期間も誤り。
#[test]
fn a_frame_a_non_finite_component_and_a_non_positive_span_are_errors() {
    let mut o = state();
    o.position.as_mut().unwrap().frame = Some("ecliptic".to_owned());
    o.velocity.as_mut().unwrap().z = f64::NAN;
    o.span = Some(Quantity::new(0.0, "d"));
    let ds = found(&design_orbit(o));
    let has = |code: &str, key: &str| {
        ds.iter().any(|(c, s, t)| {
            c == code && *s == Severity::Error && t == &format!("conditions.orbit.{key}")
        })
    };
    assert!(has("orbit.state_frame", "position"), "{ds:?}");
    assert!(has("setting.range", "velocity"), "{ds:?}");
    assert!(has("setting.range", "span"), "{ds:?}");
}

/// 位置は長さ・速度は速度・期間は時間の次元（設定の次元の表）。別の次元の単位は誤り。
#[test]
fn units_of_the_wrong_dimension_are_errors() {
    let mut o = state();
    o.position = Some(vector(1.0, 0.0, 0.0, "kg"));
    o.velocity = Some(vector(0.0, 1.0, 0.0, "km"));
    o.span = Some(Quantity::new(1.0, "km"));
    let ds = found(&design_orbit(o));
    for key in ["position", "velocity", "span"] {
        assert!(
            ds.iter()
                .any(|(_, s, t)| *s == Severity::Error && t == &format!("conditions.orbit.{key}")),
            "{key}: {ds:?}"
        );
    }
}

/// 閉じた語彙: `state` は読める。書き出すとき、状態の欄が無ければ何も出ない。
#[test]
fn the_source_reads_state_and_unused_keys_are_never_written() {
    let ok: OrbitSettings = toml::from_str(
        "source = \"state\"\nposition = { x = 1.0, y = 2.0, z = 3.0, unit = \"km\" }\nspan = { value = 2.0, unit = \"d\" }",
    )
    .expect("toml");
    assert_eq!(ok.source, Some(OrbitSource::State));
    assert_eq!(ok.position, Some(vector(1.0, 2.0, 3.0, "km")));
    let text = toml::to_string(&OrbitSettings::default()).expect("toml");
    assert!(
        !text.contains("position") && !text.contains("velocity") && !text.contains("span"),
        "{text}"
    );
}
