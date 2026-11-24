//! 打ち上げ適合の外形（部品の箱から出す）のテスト。手計算との比較。

use super::super::{LaunchFit, LaunchFitOutput};
use super::*;
use crate::spec::Analysis;
use crate::structure::interference::InterferenceSkipReason;
use sat_astro::math::Quat;
use std::f64::consts::FRAC_PI_2;

fn part(id: &str, position: Vec3, orientation: Quat, size: Vec3) -> InterferenceBox {
    InterferenceBox {
        id: id.to_owned(),
        count: 1,
        position,
        orientation,
        size,
        mount_ancestors: Vec::new(),
    }
}

/// 外形だけを判定する入力（許容の外形は 0.11 × 0.24 × 0.38 m。根の寸法は 0.1 × 0.2 × 0.3 m）。
fn input(boxes: Option<Vec<InterferenceBox>>, unsized_parts: &[&str]) -> LaunchFitInput {
    LaunchFitInput {
        parts: Vec::new(),
        propellant_mass: None,
        size: Some(Vec3::new(0.1, 0.2, 0.3)),
        part_boxes: boxes.map(|boxes| LaunchFitPartBoxes {
            boxes,
            unsized_parts: unsized_parts.iter().map(|s| (*s).to_owned()).collect(),
        }),
        max_mass: Vec::new(),
        capability: None,
        envelope: Some(LaunchFitEnvelope {
            origin: "dispenser D".to_owned(),
            size: Vec3::new(0.11, 0.24, 0.38),
        }),
        min_lateral: Vec::new(),
        min_axial: Vec::new(),
        modes: Vec::new(),
    }
}

/// 箱 A（0.1 × 0.2 × 0.3 m、原点）。
fn body() -> InterferenceBox {
    part("A", Vec3::ZERO, Quat::IDENTITY, Vec3::new(0.1, 0.2, 0.3))
}

fn check<'a>(out: &'a LaunchFitOutput, key: &str) -> &'a Check {
    out.checks.iter().find(|c| c.key == key).unwrap()
}

/// 手計算: 板 B（0.002 × 0.2 × 0.3 m）を x = 0.051 に置く。x は A の −0.05 から B の 0.051 + 0.001 = 0.052 までで 0.102 m
/// （中心は +0.001 m）、y は 0.2 m、z は 0.3 m。x の最小は A、最大は B。許容 0.11 に対する余裕は 0.11 / 0.102 − 1。
#[test]
fn envelope_is_the_extent_of_all_boxes_and_names_the_extremes() {
    let plate = part(
        "B",
        Vec3::new(0.051, 0.0, 0.0),
        Quat::IDENTITY,
        Vec3::new(0.002, 0.2, 0.3),
    );
    let out = LaunchFit::run(&input(Some(vec![body(), plate]), &[])).unwrap();
    assert_eq!(out.envelope_source, LaunchFitEnvelopeSource::Parts);
    let sizes: Vec<f64> = out.envelope.iter().map(|a| a.size.value()).collect();
    for (got, want) in sizes.iter().zip([0.102, 0.2, 0.3]) {
        assert!((got - want).abs() < 1e-12, "{got} {want}");
    }
    let x = &out.envelope[0];
    assert_eq!(
        (x.at_min.as_slice(), x.at_max.as_slice()),
        (&["A".to_owned()][..], &["B".to_owned()][..])
    );
    assert!((x.centre_offset.unwrap().value() - 0.001).abs() < 1e-12);
    assert!((x.margin.unwrap().value() - (0.11 / 0.102 - 1.0)).abs() < 1e-12);
    // y・z は両方の箱が同じ範囲なので、2 つとも端
    assert_eq!(out.envelope[1].at_max, ["A", "B"]);
    assert!(out.envelope[1].centre_offset.unwrap().value().abs() < 1e-12);
    assert!(out.envelope.iter().all(|a| a.verdict == Verdict::Ok));
    let message = &check(&out, "envelope:x").message;
    assert!(
        message.contains("at B") && message.contains("at A"),
        "{message}"
    );
}

/// 部品の箱が無ければ、根の寸法のまま（0.1 × 0.2 × 0.3 m）で、出力に外形の出どころは書かない（形態を指さない設計の結果を変えない）。
#[test]
fn without_boxes_the_root_size_is_used_and_json_is_unchanged() {
    let out = LaunchFit::run(&input(None, &[])).unwrap();
    assert_eq!(out.envelope_source, LaunchFitEnvelopeSource::RootSize);
    assert!((out.envelope[0].size.value() - 0.1).abs() < 1e-12);
    assert!(out.envelope[0].at_min.is_empty() && out.envelope[0].centre_offset.is_none());
    let json = serde_json::to_value(&out).unwrap();
    assert!(json.get("envelope_source").is_none() && json.get("skipped").is_none());
    assert!(json["envelope"][0].get("at_min").is_none());
    assert!(json["envelope"][0].get("centre_offset").is_none());
}

/// 部品の箱があれば、根の寸法より優先する（根の寸法 0.1 の x が、箱の 0.102 に置き換わる）。
#[test]
fn boxes_take_precedence_over_the_root_size() {
    let plate = part(
        "B",
        Vec3::new(0.051, 0.0, 0.0),
        Quat::IDENTITY,
        Vec3::new(0.002, 0.2, 0.3),
    );
    let mut i = input(Some(vec![body(), plate]), &[]);
    i.size = Some(Vec3::new(0.1, 0.2, 0.3));
    let out = LaunchFit::run(&i).unwrap();
    assert!((out.envelope[0].size.value() - 0.102).abs() < 1e-12);
}

/// z まわりに 90° 回した箱は、x と y の長さが入れ替わる（0.1 × 0.2 → x 0.2、y 0.1）。許容は x 0.11 なので、x は違反。
#[test]
fn a_rotated_box_swaps_its_extents() {
    let turned = part(
        "A",
        Vec3::ZERO,
        Quat::from_axis_angle(Vec3::Z, FRAC_PI_2),
        Vec3::new(0.1, 0.2, 0.3),
    );
    let out = LaunchFit::run(&input(Some(vec![turned]), &[])).unwrap();
    let s: Vec<f64> = out.envelope.iter().map(|a| a.size.value()).collect();
    for (got, want) in s.iter().zip([0.2, 0.1, 0.3]) {
        assert!((got - want).abs() < 1e-9, "{got} {want}");
    }
    assert_eq!(out.envelope[0].verdict, Verdict::Violation);
    assert_eq!(out.envelope[1].verdict, Verdict::Ok);
}

/// 寸法の無い部品は測らず、一覧に残す。寸法が正でない箱も同じ。
#[test]
fn parts_without_a_size_are_listed_not_measured() {
    let mut bad = body();
    bad.id = "BAD".to_owned();
    bad.size = Vec3::new(0.0, 0.2, 0.3);
    bad.position = Vec3::new(1.0, 0.0, 0.0);
    let out = LaunchFit::run(&input(Some(vec![body(), bad]), &["NOSIZE"])).unwrap();
    let ids: Vec<(&str, InterferenceSkipReason)> = out
        .skipped
        .iter()
        .map(|s| (s.id.as_str(), s.reason))
        .collect();
    assert_eq!(
        ids,
        [
            ("BAD", InterferenceSkipReason::InvalidSize),
            ("NOSIZE", InterferenceSkipReason::NoSize)
        ]
    );
    // 測らないので、x の長さは A だけ（0.1）
    assert!((out.envelope[0].size.value() - 0.1).abs() < 1e-12);
}

/// 測れる箱が 1 つも無ければ、外形は出せない（入力の誤り）。
#[test]
fn no_measurable_box_is_an_error() {
    assert!(LaunchFit::run(&input(Some(Vec::new()), &["X"])).is_err());
}
