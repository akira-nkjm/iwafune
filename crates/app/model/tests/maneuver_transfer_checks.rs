//! 日付つきの惑星間遷移（マヌーバの `[transfer]`）の検査: 両方の日付が要る・目標の天体が要る・数値伝搬は不可。
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
