//! `spec` のテスト。

use super::*;
use sat_units::typed::{Decibel, Energy, Fraction, Length, TorqueKind};

#[test]
fn quantity_text_takes_the_name_from_the_type() {
    assert_eq!(quantity_text::<Fraction>(0.125), "12.5 %");
    assert_eq!(quantity_text::<Energy>(3600.0), "1.00 Wh");
    assert_eq!(quantity_text::<Length>(0.012), "12.0 mm");
    // 力のモーメントは TorqueKind（次元の型 Torque は Energy と同じ型）
    assert_eq!(quantity_text::<TorqueKind>(0.002), "2.00 mN·m");
    assert_eq!(quantity_text::<Decibel>(13.54), "13.5 dB");
}

#[test]
fn channels_take_the_dimension_from_the_type() {
    let c = Channel::of::<sat_units::typed::Power>("p", ChannelKind::Load, vec![1.0]);
    assert_eq!((c.dimension.as_str(), c.subject), ("Power", None));
    let w = Channel::of::<TorqueKind>("t", ChannelKind::AeroTorque, vec![]).about("RW");
    assert_eq!(
        (w.dimension.as_str(), w.subject.as_deref()),
        ("Torque", Some("RW"))
    );
}

#[test]
fn verdict_helpers() {
    assert_eq!(Verdict::worst([]), None);
    assert_eq!(
        Verdict::worst([Verdict::Warning, Verdict::Ok]),
        Some(Verdict::Warning)
    );
    assert_eq!(
        Verdict::worst([Verdict::Violation, Verdict::Warning]),
        Some(Verdict::Violation)
    );
    assert_eq!(Verdict::from_ok(true), Verdict::Ok);
    assert_eq!(Verdict::from_ok(false), Verdict::Violation);
    assert_eq!(margin_verdict(-0.01, 0.2), Verdict::Violation);
    assert_eq!(margin_verdict(0.1, 0.2), Verdict::Warning);
    assert_eq!(margin_verdict(0.2, 0.2), Verdict::Ok);
    assert_eq!(margin_verdict(f64::NAN, 0.2), Verdict::Violation);
    assert!(ensure_fraction("efficiency", 0.5).is_ok());
    assert!(ensure_fraction("efficiency", 1.5).is_err());
    assert!(ensure_positive("capacity", 0.0).is_err());
    assert!(ensure_positive("capacity", f64::INFINITY).is_err());
}

/// 比の余裕 = 能力 / 要求 − 1: 200 N を 1.25 × 68.65 N で受けると 200 / 85.81 − 1 = 1.331。
/// 要求が 0 なら無限大なので None（判定は ok）。
#[test]
fn ratio_margin_by_hand() {
    let m = ratio_margin(200.0, 1.25 * 68.65).unwrap();
    assert!((m.value() - (200.0 / 85.8125 - 1.0)).abs() < 1e-12);
    assert!((ratio_margin(1.0, 0.8).unwrap().value() - 0.25).abs() < 1e-12);
    assert_eq!(ratio_margin(1.0, 0.0), None);
    assert_eq!(ratio_margin(0.0, 0.0), None);
    let band = Fraction::new(0.25);
    assert_eq!(ratio_margin_verdict(None, band), Verdict::Ok);
    assert_eq!(
        ratio_margin_verdict(Some(Fraction::new(0.1)), band),
        Verdict::Warning
    );
    assert_eq!(
        ratio_margin_verdict(Some(Fraction::new(-0.1)), band),
        Verdict::Violation
    );
    assert_eq!(
        ratio_margin_verdict(Some(Fraction::new(0.25)), band),
        Verdict::Ok
    );
}

/// 判定の余裕: 添えなければ持たず（JSON にも出ない）、添えた値はそのまま持つ。非有限の値は添えない。
#[test]
fn a_check_carries_its_margin_only_when_given() {
    let plain = Check::new("k", "l", Verdict::Ok, "m");
    assert_eq!(plain.margin, None);
    assert!(!serde_json::to_string(&plain).unwrap().contains("margin"));
    let with = plain.clone().with_margin(0.25);
    assert_eq!(with.margin, Some(0.25));
    let json = serde_json::to_string(&with).unwrap();
    assert!(json.contains("\"margin\":0.25"));
    // 余裕の欄が無い JSON も読める
    let back: Check = serde_json::from_str(&serde_json::to_string(&plain).unwrap()).unwrap();
    assert_eq!(back.margin, None);
    assert_eq!(plain.clone().with_margin(f64::NAN).margin, None);
    assert_eq!(plain.clone().with_margin(f64::INFINITY).margin, None);
    assert_eq!(plain.clone().with_margin(None).margin, None);
    // 比の余裕 1 / 0.8 − 1 = 0.25。無限大（None）は添えない
    let ratio = plain.clone().with_ratio_margin(ratio_margin(1.0, 0.8));
    assert!((ratio.margin.unwrap() - 0.25).abs() < 1e-12);
    assert_eq!(plain.with_ratio_margin(ratio_margin(1.0, 0.0)).margin, None);
}

/// 厳しい（小さい）ほうが先。余裕の無い判定は持つ判定より後。どちらも無い・同じなら Equal。
#[test]
fn margin_order_puts_the_tightest_first_and_the_unmargined_last() {
    use std::cmp::Ordering::{Equal, Greater, Less};
    assert_eq!(margin_order(Some(0.2), Some(0.9)), Less);
    assert_eq!(margin_order(Some(0.9), Some(0.2)), Greater);
    assert_eq!(margin_order(Some(-0.1), Some(0.2)), Less);
    assert_eq!(margin_order(Some(0.2), Some(0.2)), Equal);
    assert_eq!(margin_order(Some(5.0), None), Less);
    assert_eq!(margin_order(None, Some(-5.0)), Greater);
    assert_eq!(margin_order(None, None), Equal);
}
