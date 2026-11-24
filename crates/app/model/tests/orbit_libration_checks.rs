//! 軌道の出どころ `libration`（ラグランジュ点・ハロー軌道）の検査: 欄が正しく書けて読めること、別の出どころに添えた欄・
//! ラグランジュ点の軌道に添えた欄は `setting.not_applied` の警告、天体の組・振幅・枝の誤りは検査の誤り。
//! 足りない欄は検査の誤りにしない（射影が足りない入力として知らせる）。
use fune_model::design::{
    CentralBodyKind, HaloBranchKind, LibrationPointKind, OrbitSettings, OrbitSource,
};
use fune_model::value::Quantity;
use fune_model::{Design, Severity, check};

fn libration() -> OrbitSettings {
    OrbitSettings {
        source: Some(OrbitSource::Libration),
        epoch: Some("2030-01-01T00:00:00Z".to_owned()),
        primary: Some(CentralBodyKind::Sun),
        secondary: Some(CentralBodyKind::Earth),
        point: Some(LibrationPointKind::L2),
        ..OrbitSettings::default()
    }
}

fn halo() -> OrbitSettings {
    OrbitSettings {
        amplitude_z: Some(Quantity::new(120_000.0, "km")),
        branch: Some(HaloBranchKind::North),
        ..libration()
    }
}

fn found(o: OrbitSettings) -> Vec<(String, Severity, String)> {
    let mut d = Design::default();
    d.conditions.orbit = Some(o);
    check(&d)
        .into_iter()
        .filter(|x| {
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

fn has(ds: &[(String, Severity, String)], code: &str, severity: Severity, key: &str) -> bool {
    ds.iter()
        .any(|(c, s, t)| c == code && *s == severity && t == &format!("conditions.orbit.{key}"))
}

#[test]
fn a_complete_point_or_halo_is_clean_and_missing_keys_are_not_check_errors() {
    assert!(found(libration()).is_empty());
    assert!(found(halo()).is_empty());
    let only_source = OrbitSettings {
        source: Some(OrbitSource::Libration),
        ..OrbitSettings::default()
    };
    assert!(found(only_source).is_empty());
}

/// 主天体は副天体の親でなければならない（太陽–惑星、地球–月）。同じ天体・逆・無関係な組は誤り。
#[test]
fn the_pair_must_be_a_body_and_the_body_it_orbits() {
    for (p, s, ok) in [
        (CentralBodyKind::Sun, CentralBodyKind::Earth, true),
        (CentralBodyKind::Sun, CentralBodyKind::Mars, true),
        (CentralBodyKind::Earth, CentralBodyKind::Moon, true),
        (CentralBodyKind::Earth, CentralBodyKind::Sun, false),
        (CentralBodyKind::Earth, CentralBodyKind::Earth, false),
        (CentralBodyKind::Earth, CentralBodyKind::Mars, false),
        (CentralBodyKind::Sun, CentralBodyKind::Moon, false),
    ] {
        let mut o = libration();
        (o.primary, o.secondary) = (Some(p), Some(s));
        let ds = found(o);
        assert_eq!(
            has(&ds, "orbit.libration_pair", Severity::Error, "secondary"),
            !ok,
            "{p} {s}: {ds:?}"
        );
    }
}

/// ハローの振幅は正で L1・L2 だけ。枝は振幅と一緒に書く。
#[test]
fn amplitude_and_branch_rules() {
    let mut l4 = halo();
    l4.point = Some(LibrationPointKind::L4);
    assert!(has(
        &found(l4),
        "orbit.libration_halo_point",
        Severity::Error,
        "amplitude_z"
    ));
    let mut l1 = halo();
    l1.point = Some(LibrationPointKind::L1);
    assert!(found(l1).is_empty());
    let mut negative = halo();
    negative.amplitude_z = Some(Quantity::new(-1.0, "km"));
    assert!(has(
        &found(negative),
        "setting.range",
        Severity::Error,
        "amplitude_z"
    ));
    let mut branch_only = libration();
    branch_only.branch = Some(HaloBranchKind::South);
    assert!(has(
        &found(branch_only),
        "orbit.libration_branch",
        Severity::Warning,
        "branch"
    ));
    let mut wrong_unit = halo();
    wrong_unit.amplitude_z = Some(Quantity::new(1.0, "kg"));
    assert!(
        found(wrong_unit)
            .iter()
            .any(|(_, s, t)| *s == Severity::Error && t == "conditions.orbit.amplitude_z")
    );
}

/// ラグランジュ点の欄を `elements`（書かない）に添えると使われない（警告）。ラグランジュ点の軌道に添えた要素の欄と `central` も警告。
#[test]
fn unused_keys_are_warned() {
    let mut elements = halo();
    elements.source = None;
    elements.epoch = None;
    let ds = found(elements);
    for key in ["primary", "secondary", "point", "amplitude_z", "branch"] {
        assert!(
            has(&ds, "setting.not_applied", Severity::Warning, key),
            "{key}: {ds:?}"
        );
    }
    let mut o = halo();
    o.altitude = Some(Quantity::new(550.0, "km"));
    o.central = Some(CentralBodyKind::Earth);
    let ds = found(o);
    assert!(has(
        &ds,
        "setting.not_applied",
        Severity::Warning,
        "altitude"
    ));
    assert!(has(
        &ds,
        "setting.not_applied",
        Severity::Warning,
        "central"
    ));
    assert_eq!(ds.len(), 2, "{ds:?}");
}

#[test]
fn the_offered_keys_do_not_overlap_the_unused_keys_and_the_file_round_trips() {
    let source = OrbitSource::Libration;
    assert_eq!(
        source.offered_keys(),
        [
            "source",
            "epoch",
            "primary",
            "secondary",
            "point",
            "amplitude_z",
            "branch"
        ]
    );
    let unused = source.unused_keys();
    assert!(source.offered_keys().iter().all(|k| !unused.contains(k)));
    let o: OrbitSettings = toml::from_str(
        "source = \"libration\"\nprimary = \"sun\"\nsecondary = \"earth\"\npoint = \"l2\"\namplitude_z = { value = 120000.0, unit = \"km\" }\nbranch = \"south\"",
    )
    .expect("toml");
    assert_eq!(o.point, Some(LibrationPointKind::L2));
    assert_eq!(o.branch, Some(HaloBranchKind::South));
    assert_eq!(
        toml::from_str::<OrbitSettings>(&toml::to_string(&o).expect("w")).expect("r"),
        o
    );
    let text = toml::to_string(&OrbitSettings::default()).expect("toml");
    assert!(!text.contains("primary") && !text.contains("point") && !text.contains("branch"));
}
