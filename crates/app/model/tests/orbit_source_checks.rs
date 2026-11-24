//! 軌道の出どころ（`source`・`maneuver`）の検査: `transfer` が指すマヌーバがあり日付つきの `[transfer]` を持つこと、
//! 出どころが使わない欄は `setting.not_applied` の警告、書かなければ今までと同じ（診断も無い）。
//! 設計の軌道（`conditions/orbit.toml`）とフェーズの `[orbit]` で同じ検査で、対象と位置だけが違う。
use fune_model::design::{
    CentralBodyKind, Maneuver, ManeuverCategory, ManeuverConstraints, ManeuverMethod,
    ManeuverTarget, ManeuverTransfer, OrbitSettings, OrbitSource, Phase,
};
use fune_model::value::Quantity;
use fune_model::{Design, Diagnostic, Severity, check};

fn maneuver(id: &str, transfer: bool) -> Maneuver {
    Maneuver {
        id: id.into(),
        label: None,
        category: ManeuverCategory::Insertion,
        method: ManeuverMethod::Impulsive,
        thruster: None,
        after: None,
        via_altitude: None,
        delta_v: None,
        target: ManeuverTarget {
            body: Some(CentralBodyKind::Mars),
            perigee_altitude: Some(Quantity::new(300.0, "km")),
            ..ManeuverTarget::default()
        },
        constraints: ManeuverConstraints::default(),
        transfer: transfer.then(|| ManeuverTransfer {
            departure: Some("2028-12-10T00:00:00Z".to_owned()),
            arrival: Some("2029-09-25T00:00:00Z".to_owned()),
            departure_by: None,
            flybys: Vec::new(),
        }),
    }
}

fn base() -> Design {
    let mut d = Design::default();
    for (id, transfer) in [("DATED", true), ("PLAIN", false)] {
        d.maneuvers.insert(id.into(), maneuver(id, transfer));
    }
    d
}

fn transfer(maneuver: Option<&str>) -> OrbitSettings {
    OrbitSettings {
        source: Some(OrbitSource::Transfer),
        maneuver: maneuver.map(str::to_owned),
        ..OrbitSettings::default()
    }
}

fn design_orbit(o: OrbitSettings) -> Design {
    let mut d = base();
    d.conditions.orbit = Some(o);
    d
}

fn phase_orbit(o: OrbitSettings) -> Design {
    let mut d = base();
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

/// 出どころに関わる診断（コード・重大度・対象）。
fn found(d: &Design) -> Vec<(String, Severity, String)> {
    check(d)
        .into_iter()
        .filter(|x: &Diagnostic| {
            x.code == "setting.not_applied"
                || x.code == "ref.maneuver"
                || x.code.starts_with("orbit.")
        })
        .map(|x| {
            (
                x.code,
                x.severity,
                x.target
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn an_orbit_without_a_source_is_unchanged() {
    let o = OrbitSettings {
        altitude: Some(Quantity::new(550.0, "km")),
        ..OrbitSettings::default()
    };
    assert_eq!(o.source_kind(), OrbitSource::Elements);
    assert!(found(&design_orbit(o.clone())).is_empty());
    assert!(found(&phase_orbit(o)).is_empty());
}

#[test]
fn a_transfer_orbit_with_a_dated_maneuver_is_clean() {
    assert!(found(&design_orbit(transfer(Some("DATED")))).is_empty());
    assert!(found(&phase_orbit(transfer(Some("DATED")))).is_empty());
}

/// `maneuver` が書かれていないのは足りない入力（射影が選べるマヌーバを添えて知らせる）で、検査の誤りではない。
#[test]
fn a_missing_maneuver_is_not_a_check_error() {
    assert!(found(&design_orbit(transfer(None))).is_empty());
}

#[test]
fn the_maneuver_must_exist_and_have_a_dated_transfer() {
    let ds = found(&design_orbit(transfer(Some("NOPE"))));
    assert_eq!(
        ds,
        [(
            "ref.maneuver".to_owned(),
            Severity::Error,
            "conditions.orbit.maneuver".to_owned()
        )]
    );
    let ds = found(&design_orbit(transfer(Some("PLAIN"))));
    assert_eq!(
        ds,
        [(
            "orbit.maneuver_transfer".to_owned(),
            Severity::Error,
            "conditions.orbit.maneuver".to_owned()
        )]
    );
    // フェーズの軌道は対象がフェーズのファイル
    let ds = found(&phase_orbit(transfer(Some("PLAIN"))));
    assert_eq!(ds[0].2, "phases.CRUISE.orbit.maneuver");
}

/// `transfer` に添えた要素は使われない（警告）。`elements` に添えた `maneuver` も同じ。
#[test]
fn keys_the_source_does_not_use_are_warned() {
    let mut o = transfer(Some("DATED"));
    o.altitude = Some(Quantity::new(550.0, "km"));
    o.epoch = Some("2027-04-01T00:00:00Z".to_owned());
    let ds = found(&design_orbit(o));
    assert_eq!(ds.len(), 2, "{ds:?}");
    assert!(
        ds.iter()
            .all(|(c, s, _)| c == "setting.not_applied" && *s == Severity::Warning)
    );
    let targets: Vec<&str> = ds.iter().map(|d| d.2.as_str()).collect();
    assert!(targets.contains(&"conditions.orbit.altitude"));
    assert!(targets.contains(&"conditions.orbit.epoch"));

    // 出どころを書かない（= elements）軌道の `maneuver`
    let o = OrbitSettings {
        maneuver: Some("DATED".to_owned()),
        altitude: Some(Quantity::new(550.0, "km")),
        ..OrbitSettings::default()
    };
    let ds = found(&phase_orbit(o));
    assert_eq!(
        ds,
        [(
            "setting.not_applied".to_owned(),
            Severity::Warning,
            "phases.CRUISE.orbit.maneuver".to_owned()
        )]
    );
}

/// `label` は出どころによらず使える。
#[test]
fn the_label_is_allowed_with_any_source() {
    let mut o = transfer(Some("DATED"));
    o.label = Some("Cruise".to_owned());
    assert!(found(&design_orbit(o)).is_empty());
}

#[test]
fn the_source_is_a_closed_vocabulary() {
    let ok: OrbitSettings =
        toml::from_str("source = \"transfer\"\nmaneuver = \"X\"").expect("toml");
    assert_eq!(ok.source, Some(OrbitSource::Transfer));
    let bad = toml::from_str::<OrbitSettings>("source = \"teleport\"");
    assert!(bad.is_err());
    // 書かなければ書き出さない（既存のファイルは変わらない）
    let text = toml::to_string(&OrbitSettings::default()).expect("toml");
    assert!(
        !text.contains("source") && !text.contains("maneuver"),
        "{text}"
    );
}
