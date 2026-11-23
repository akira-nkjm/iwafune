//! `structure.launch_fit` のテスト（手計算との比較）。

use super::*;

fn mass_limit(origin: &str, kg: f64) -> LaunchFitMassLimit {
    LaunchFitMassLimit {
        origin: origin.to_owned(),
        value: Qty::from_raw(kg),
    }
}

fn freq_limit(origin: &str, hz: f64) -> LaunchFitFrequencyLimit {
    LaunchFitFrequencyLimit {
        origin: origin.to_owned(),
        value: Qty::from_raw(hz),
    }
}

fn mode(kind: ModalEstimateKind, hz: f64) -> LaunchFitMode {
    LaunchFitMode {
        id: "BODY".to_owned(),
        kind,
        frequency: Qty::from_raw(hz),
    }
}

/// 6U 相当の収納外形 0.1 × 0.2263 × 0.366 m。部品 4 kg × 1（成長 20 %）+ 2 kg × 2（成長 0）= 乾燥 8 kg（成長込み 8.8 kg）、
/// 推薬 1.5 kg（成長 0）。素の質量 9.5 kg、成長込み 10.3 kg。
fn base() -> LaunchFitInput {
    LaunchFitInput {
        parts: vec![
            BudgetItem {
                id: "A".to_owned(),
                group: String::new(),
                value: 4.0,
                count: 1,
                growth: 0.2,
            },
            BudgetItem::counted("B", 2.0, 2),
        ],
        propellant_mass: Some(Qty::from_raw(1.5)),
        size: Some(Vec3::new(0.1, 0.2263, 0.366)),
        max_mass: vec![mass_limit("launcher L", 11.0)],
        envelope: Some(LaunchFitEnvelope {
            origin: "dispenser D".to_owned(),
            size: Vec3::new(0.1, 0.2263, 0.366),
        }),
        min_lateral: vec![freq_limit("dispenser D", 40.0)],
        min_axial: vec![freq_limit("dispenser D", 80.0)],
        modes: vec![
            mode(ModalEstimateKind::BeamLateral, 60.0),
            mode(ModalEstimateKind::BeamAxial, 120.0),
        ],
    }
}

fn check<'a>(out: &'a LaunchFitOutput, key: &str) -> &'a Check {
    out.checks.iter().find(|c| c.key == key).unwrap()
}

/// 手計算: 成長込みの質量 = 4 × 1.2 + 2 × 2 + 1.5 = 10.3 kg（素は 9.5 kg）、質量の余裕 = 11 / 10.3 − 1 = 0.06796。外形は許容と同じ寸法なので余裕 0（収まる）。
/// 固有振動数の余裕は横 60 / 40 − 1 = 0.5、機軸 120 / 80 − 1 = 0.5。
#[test]
fn fits_by_hand() {
    let out = LaunchFit::run(&base()).unwrap();
    let mass = out.mass.as_ref().unwrap();
    assert!((mass.launch_mass.value() - 10.3).abs() < 1e-12);
    assert!((mass.bare_mass.value() - 9.5).abs() < 1e-12);
    assert!((mass.margin.unwrap().value() - (11.0 / 10.3 - 1.0)).abs() < 1e-12);
    assert!(out.envelope.iter().all(|a| a.verdict == Verdict::Ok));
    assert!(
        out.envelope
            .iter()
            .all(|a| a.margin.unwrap().value().abs() < 1e-12)
    );
    for f in &out.frequencies {
        assert!((f.margin.unwrap().value() - 0.5).abs() < 1e-12);
    }
    assert!(
        out.checks.iter().all(|c| c.verdict == Verdict::Ok),
        "{:?}",
        out.checks
    );
    assert_eq!(out.checks.len(), 6);
}

/// 質量 10.3 kg が上限 9 kg を超える（余裕 = 9 / 10.3 − 1 < 0）。外形は y が 1 mm 足りない。軸の入れ替えでは救わない。
#[test]
fn does_not_fit() {
    let mut input = base();
    input.max_mass = vec![mass_limit("launcher L", 9.0)];
    input.envelope = Some(LaunchFitEnvelope {
        origin: "dispenser D".to_owned(),
        size: Vec3::new(0.1, 0.2253, 0.366),
    });
    let out = LaunchFit::run(&input).unwrap();
    assert_eq!(check(&out, "mass").verdict, Verdict::Violation);
    assert_eq!(check(&out, "envelope:x").verdict, Verdict::Ok);
    assert_eq!(check(&out, "envelope:y").verdict, Verdict::Violation);
    assert_eq!(check(&out, "envelope:z").verdict, Verdict::Ok);
    // 余裕は y だけが負（許容 0.2253 / 外形 0.2263 − 1）。軸を入れ替えて救うことはしない
    assert!(out.envelope[1].margin.unwrap().value() < 0.0);
}

/// 要求と打ち上げ機の質量の上限は、厳しいほう（小さいほう）が効き、出所が文に出る。
#[test]
fn tighter_mass_limit_wins() {
    let mut input = base();
    input.max_mass = vec![
        mass_limit("launcher L", 12.0),
        mass_limit("requirement R_MASS", 9.0),
    ];
    let out = LaunchFit::run(&input).unwrap();
    let mass = out.mass.unwrap();
    assert_eq!(mass.origin, "requirement R_MASS");
    assert_eq!(mass.verdict, Verdict::Violation);
    assert!(
        check(&LaunchFit::run(&input).unwrap(), "mass")
            .message
            .contains("requirement R_MASS")
    );
    // 緩い要求は効かない
    input.max_mass = vec![
        mass_limit("launcher L", 10.0),
        mass_limit("requirement R_MASS", 12.0),
    ];
    assert_eq!(
        LaunchFit::run(&input).unwrap().mass.unwrap().origin,
        "launcher L"
    );
}

/// 固有振動数の下限は大きいほうが効く。要求が 70 Hz なら横 60 Hz の見積もりは足りない。
#[test]
fn tighter_frequency_limit_wins() {
    let mut input = base();
    input.min_lateral = vec![
        freq_limit("dispenser D", 40.0),
        freq_limit("requirement R_FREQ", 70.0),
    ];
    let out = LaunchFit::run(&input).unwrap();
    let f = &out.frequencies[0];
    assert_eq!(f.origin, "requirement R_FREQ");
    assert_eq!(f.verdict, Verdict::Violation);
    assert!((f.margin.unwrap().value() - (60.0 / 70.0 - 1.0)).abs() < 1e-12);
}

/// パネルの局所モードは機体全体の条件とは比べない。機体全体の見積もりが無ければ警告（確かめていない）。
#[test]
fn panel_modes_are_not_whole_body() {
    let mut input = base();
    input.modes = vec![mode(ModalEstimateKind::Panel, 10.0)];
    let out = LaunchFit::run(&input).unwrap();
    for f in &out.frequencies {
        assert_eq!(f.verdict, Verdict::Warning);
        assert!(f.estimated.is_none());
    }
    assert!(
        check(&out, "frequency:lateral")
            .message
            .starts_with("Not verified")
    );
}

#[test]
fn nothing_to_compare_is_an_error() {
    let mut input = base();
    input.max_mass.clear();
    input.envelope = None;
    input.min_lateral.clear();
    input.min_axial.clear();
    assert!(LaunchFit::run(&input).is_err());
}

/// 素の質量 9.5 kg は上限 10 kg に収まるが、成長余裕込みの 10.3 kg は超える。判定は成長込みで、文は両方の質量を示す。
#[test]
fn growth_allowance_is_judged() {
    let mut input = base();
    input.max_mass = vec![mass_limit("launcher L", 10.0)];
    let out = LaunchFit::run(&input).unwrap();
    let mass = out.mass.as_ref().unwrap();
    assert_eq!(mass.verdict, Verdict::Violation);
    assert!((mass.margin.unwrap().value() - (10.0 / 10.3 - 1.0)).abs() < 1e-12);
    let message = &check(&out, "mass").message;
    assert!(
        message.starts_with("Launch mass with growth allowance 10.3 kg (bare 9.50 kg)"),
        "{message}"
    );
}

/// 推薬には成長余裕を掛けない（質量バジェットの推薬込みの合計と同じ）。推薬を 0 にすると成長込みは 8.8 kg。
#[test]
fn propellant_carries_no_growth() {
    let mut input = base();
    input.propellant_mass = None;
    let mass = LaunchFit::run(&input).unwrap().mass.unwrap();
    assert!((mass.launch_mass.value() - 8.8).abs() < 1e-12);
    assert!((mass.bare_mass.value() - 8.0).abs() < 1e-12);
}
