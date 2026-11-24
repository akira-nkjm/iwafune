//! フェーズ（`phases/<ID>.toml`）: ケースが `phase` を指すと、射影はフェーズの `[orbit]` を設計の軌道の代わりに使い（丸ごと置き換え）、
//! フェーズの `[law.<モード>]` をそのモードの姿勢則に使う。フェーズの無い設計・フェーズの無いケースは今までと同じ。
//! 設計はコードで書き換える（デモのデータは変えない）。

use fune_model::Design;
use fune_model::Target;
use fune_model::design::{
    AttitudeLawKind, AttitudeLawSetting, Case, OrbitSettings, Phase, UncertaintySide,
};
use fune_model::id::Id;
use fune_model::value::Quantity;
use fune_projection::{Context, MissingInputs};
use sat_astro::attitude::AttitudeLaw;
use std::path::Path;

fn demo() -> Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root)
        .expect("the demo should load")
        .design
}

/// デモに、同じ軌道に別の高度を書いたフェーズと、姿勢則を 1 つ上書きしたフェーズ、それぞれを指すケースを足す。
fn with_phases() -> Design {
    let mut d = demo();
    let mut orbit: OrbitSettings = d.conditions.orbit.clone().expect("orbit");
    orbit.altitude = Some(Quantity::new(250.0, "km"));
    d.phases.insert(
        Id::new("LOW"),
        Phase {
            id: Id::new("LOW"),
            orbit: Some(orbit),
            ..Phase::default()
        },
    );
    d.phases.insert(
        Id::new("INERTIAL"),
        Phase {
            id: Id::new("INERTIAL"),
            law: [(
                "NOMINAL".to_owned(),
                AttitudeLawSetting {
                    law: AttitudeLawKind::Inertial,
                    axis: None,
                    secondary: None,
                    along_track: None,
                    orientation: None,
                },
            )]
            .into(),
            ..Phase::default()
        },
    );
    for (id, phase) in [
        ("in_low", "LOW"),
        ("in_inertial", "INERTIAL"),
        ("plain", ""),
    ] {
        d.cases.insert(
            Id::new(id),
            Case {
                id: Id::new(id),
                label: None,
                description: None,
                mode: Some(Id::new("NOMINAL")),
                phase: (!phase.is_empty()).then(|| Id::new(phase)),
                side: Some(UncertaintySide::Nominal),
                thermal_case: None,
                beta: None,
                orbits: None,
                sun_distance: None,
                epoch: None,
            },
        );
    }
    d
}

fn orbit_of(d: &Design, case: Option<&str>) -> sat_astro::orbit::OrbitSpec {
    let resolved = fune_model::resolve(d);
    let ctx = Context::new(d, &resolved, case).unwrap();
    ctx.orbit(&mut MissingInputs::new()).expect("orbit")
}

/// フェーズの軌道が設計の軌道の代わりになる（高度が違えば長半径も周期も違う）。フェーズを指さないケースは設計の軌道のまま。
#[test]
fn the_phase_orbit_replaces_the_design_orbit() {
    let d = with_phases();
    let design = orbit_of(&d, None);
    assert_eq!(
        orbit_of(&d, Some("plain")).elements.semi_major_axis.value(),
        design.elements.semi_major_axis.value()
    );
    let low = orbit_of(&d, Some("in_low"));
    let dropped = design.elements.semi_major_axis.value() - low.elements.semi_major_axis.value();
    assert!((dropped - 300e3).abs() < 1.0, "{dropped}");
    assert!(low.period_s() < design.period_s() - 100.0);
}

/// フェーズの軌道は丸ごとの置き換えで、設計の軌道のキーは合成しない（高度だけ書いたフェーズの軌道は、傾斜角などが足りない）。
/// 足りない入力の対象はフェーズのファイル（`phases.<ID>.orbit.<キー>`）を指す。
#[test]
fn a_phase_orbit_is_not_merged_with_the_design_orbit() {
    let mut d = with_phases();
    d.phases.get_mut("LOW").unwrap().orbit = Some(OrbitSettings {
        altitude: Some(Quantity::new(250.0, "km")),
        ..OrbitSettings::default()
    });
    let resolved = fune_model::resolve(&d);
    let ctx = Context::new(&d, &resolved, Some("in_low")).unwrap();
    let mut m = MissingInputs::new();
    assert!(ctx.orbit(&mut m).is_none());
    let paths: Vec<String> = m.into_vec().into_iter().map(|i| i.path).collect();
    assert!(
        paths.contains(&"phases.LOW.orbit.epoch".to_owned()),
        "{paths:?}"
    );
    assert!(paths.iter().all(|p| !p.starts_with("conditions.")));
    // フェーズの軌道が無いフェーズでは、設計の軌道の対象のまま
    let mut d = with_phases();
    d.conditions.orbit = None;
    let resolved = fune_model::resolve(&d);
    let ctx = Context::new(&d, &resolved, Some("in_inertial")).unwrap();
    let mut m = MissingInputs::new();
    assert!(ctx.orbit(&mut m).is_none());
    assert_eq!(
        m.into_vec()[0].target,
        Some(Target::from(fune_model::path::SettingPath::conditions(&[
            "orbit"
        ])))
    );
}

/// フェーズの `[law.<モード>]` はそのモードの姿勢則を置き換える。ほかのモードと、フェーズを指さないケースは設計の姿勢則のまま。
#[test]
fn the_phase_law_overrides_that_mode_only() {
    let d = with_phases();
    let resolved = fune_model::resolve(&d);
    let law = |case: &str, mode: &str| {
        let ctx = Context::new(&d, &resolved, Some(case)).unwrap();
        ctx.attitude_law(Some(mode), &mut MissingInputs::new())
            .expect("law")
    };
    assert!(matches!(
        law("in_inertial", "NOMINAL"),
        AttitudeLaw::Inertial { .. }
    ));
    assert!(matches!(
        law("in_inertial", "MISSION"),
        AttitudeLaw::NadirPointing { .. }
    ));
    assert!(matches!(
        law("plain", "NOMINAL"),
        AttitudeLaw::SunPointing { .. }
    ));
    let ctx = Context::new(&d, &resolved, Some("in_inertial")).unwrap();
    assert_eq!(
        ctx.law_target("NOMINAL").to_string(),
        "phases.INERTIAL.law.NOMINAL"
    );
    assert_eq!(
        ctx.law_target("MISSION").to_string(),
        "analysis.aocs.law.MISSION"
    );
}

/// 存在しないフェーズを指すケースは設計の軌道のまま動く（検査が `ref.phase` で知らせる）。
#[test]
fn an_unknown_phase_falls_back_to_the_design() {
    let mut d = with_phases();
    d.cases.get_mut("plain").unwrap().phase = Some(Id::new("NOPE"));
    assert_eq!(
        orbit_of(&d, Some("plain")).elements.semi_major_axis.value(),
        orbit_of(&d, None).elements.semi_major_axis.value()
    );
}
