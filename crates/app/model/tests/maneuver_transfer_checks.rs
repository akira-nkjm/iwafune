//! 日付つきの惑星間遷移（マヌーバの `[transfer]`）の検査: 両方の日付が要る・目標の天体が要る・数値伝播は不可。
//! 日付の読み方・暦の範囲・順序・天体の組は暦を持つ fune-projection の側（`maneuver_dates` のテスト）。
use fune_model::design::{
    CentralBodyKind, Maneuver, ManeuverCategory, ManeuverConstraints, ManeuverMethod,
    ManeuverTarget, ManeuverTransfer,
};
use fune_model::{Design, check};

fn design(
    body: Option<CentralBodyKind>,
    tr: Option<ManeuverTransfer>,
    m: ManeuverMethod,
) -> Design {
    let mut d = Design::default();
    d.maneuvers.insert(
        "MARS".into(),
        Maneuver {
            id: "MARS".into(),
            label: None,
            category: ManeuverCategory::Insertion,
            method: m,
            thruster: None,
            after: None,
            via_altitude: None,
            delta_v: None,
            target: ManeuverTarget {
                body,
                perigee_altitude: Some(fune_model::value::Quantity::new(300.0, "km")),
                ..ManeuverTarget::default()
            },
            constraints: ManeuverConstraints::default(),
            transfer: tr,
        },
    );
    d
}

fn dates(a: Option<&str>, b: Option<&str>) -> Option<ManeuverTransfer> {
    Some(ManeuverTransfer {
        departure: a.map(str::to_owned),
        arrival: b.map(str::to_owned),
        departure_by: None,
        flybys: Vec::new(),
    })
}

fn codes(d: &Design) -> Vec<String> {
    check(d)
        .into_iter()
        .filter(|x| x.code.starts_with("maneuver.transfer"))
        .map(|x| x.code)
        .collect()
}

#[test]
fn a_complete_transfer_with_a_body_has_no_diagnostics() {
    let d = design(
        Some(CentralBodyKind::Mars),
        dates(Some("2028-12-10T00:00:00Z"), Some("2029-09-25T00:00:00Z")),
        ManeuverMethod::Impulsive,
    );
    assert!(codes(&d).is_empty());
    // `[transfer]` を書かなければ何も言わない（従来のホーマンの見積もり）
    assert!(
        codes(&design(
            Some(CentralBodyKind::Mars),
            None,
            ManeuverMethod::Impulsive
        ))
        .is_empty()
    );
}

#[test]
fn both_dates_are_required() {
    let d = design(
        Some(CentralBodyKind::Mars),
        dates(Some("2028-12-10T00:00:00Z"), None),
        ManeuverMethod::Impulsive,
    );
    assert_eq!(codes(&d), ["maneuver.transfer_dates"]);
    let d = design(
        Some(CentralBodyKind::Mars),
        dates(None, Some(" ")),
        ManeuverMethod::Impulsive,
    );
    assert_eq!(
        codes(&d),
        ["maneuver.transfer_dates", "maneuver.transfer_dates"]
    );
}

#[test]
fn a_target_body_is_required() {
    let d = design(
        None,
        dates(Some("2028-12-10T00:00:00Z"), Some("2029-09-25T00:00:00Z")),
        ManeuverMethod::Impulsive,
    );
    assert_eq!(codes(&d), ["maneuver.transfer_body"]);
}

#[test]
fn propagated_and_low_thrust_are_rejected_for_dated_transfers() {
    let tr = dates(Some("2028-12-10T00:00:00Z"), Some("2029-09-25T00:00:00Z"));
    let d = design(
        Some(CentralBodyKind::Mars),
        tr.clone(),
        ManeuverMethod::Propagated,
    );
    assert_eq!(codes(&d), ["maneuver.transfer_method"]);
    // 低推力は従来どおり `maneuver.body_method`
    let d = design(Some(CentralBodyKind::Mars), tr, ManeuverMethod::LowThrust);
    assert!(check(&d).iter().any(|x| x.code == "maneuver.body_method"));
}

/// `departure_by = "launcher"` は、前のマヌーバ（`after` がマヌーバ）があると警告（先に軌道を変える噴射は打ち上げ機の送り先と合わない）。
/// 最初のマヌーバなら何も言わず、`spacecraft`（既定）なら前のマヌーバがあっても言わない。
#[test]
fn departure_by_the_launcher_after_another_maneuver_is_a_warning() {
    use fune_model::design::DepartureBy;
    let launcher = || {
        let mut tr = dates(Some("2028-12-10T00:00:00Z"), Some("2029-09-25T00:00:00Z")).unwrap();
        tr.departure_by = Some(DepartureBy::Launcher);
        tr
    };
    let warns = |d: &Design| {
        check(d)
            .into_iter()
            .filter(|x| x.code == "maneuver.departure_by_launcher_after")
            .count()
    };
    let mut d = design(
        Some(CentralBodyKind::Mars),
        Some(launcher()),
        ManeuverMethod::Impulsive,
    );
    assert_eq!(warns(&d), 0);
    let mut raise = d.maneuvers["MARS"].clone();
    raise.id = "RAISE".into();
    raise.transfer = None;
    raise.target = ManeuverTarget::default();
    d.maneuvers.insert("RAISE".into(), raise);
    d.maneuvers.get_mut("MARS").unwrap().after = Some("RAISE".into());
    assert_eq!(warns(&d), 1);
    d.maneuvers
        .get_mut("MARS")
        .unwrap()
        .transfer
        .as_mut()
        .unwrap()
        .departure_by = Some(DepartureBy::Spacecraft);
    assert_eq!(warns(&d), 0);
}

fn flyby(date: &str, altitude: Option<(f64, &str)>) -> fune_model::design::ManeuverFlyby {
    fune_model::design::ManeuverFlyby {
        body: CentralBodyKind::Venus,
        date: date.to_owned(),
        min_altitude: altitude.map(|(v, u)| fune_model::value::Quantity::new(v, u)),
        powered: None,
    }
}

fn flyby_codes(f: fune_model::design::ManeuverFlyby) -> Vec<String> {
    let mut tr = dates(Some("2028-12-10T00:00:00Z"), Some("2029-09-25T00:00:00Z")).unwrap();
    tr.flybys.push(f);
    let d = design(
        Some(CentralBodyKind::Mars),
        Some(tr),
        ManeuverMethod::Impulsive,
    );
    check(&d)
        .into_iter()
        .filter(|x| x.code.starts_with("maneuver.flyby") || x.code.starts_with("unit."))
        .map(|x| x.code)
        .collect()
}

/// スイングバイの形: 日付が要り、近点の高度の下限は長さの正の値（0 や負は表面以下を通ることになるので受け付けない）。
#[test]
fn flybys_need_a_date_and_a_positive_length_altitude() {
    assert!(flyby_codes(flyby("2029-03-01T00:00:00Z", None)).is_empty());
    assert!(flyby_codes(flyby("2029-03-01T00:00:00Z", Some((300.0, "km")))).is_empty());
    assert_eq!(flyby_codes(flyby(" ", None)), ["maneuver.flyby_date"]);
    assert_eq!(
        flyby_codes(flyby("2029-03-01T00:00:00Z", Some((0.0, "km")))),
        ["maneuver.flyby_altitude"]
    );
    assert_eq!(
        flyby_codes(flyby("2029-03-01T00:00:00Z", Some((-5.0, "km")))),
        ["maneuver.flyby_altitude"]
    );
    assert_eq!(
        flyby_codes(flyby("2029-03-01T00:00:00Z", Some((300.0, "kg")))),
        ["unit.dimension"]
    );
}
