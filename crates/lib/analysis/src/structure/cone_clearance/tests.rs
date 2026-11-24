//! `structure.cone_clearance` のテスト（手計算との比較）。担い手の表の規則（頂点・自身の箱・取付の連鎖・半頂角が無い担い手・距離の上限）を確かめる。

use super::*;
use sat_astro::math::Quat;

const DEG: f64 = std::f64::consts::PI / 180.0;

fn cube(id: &str, center: [f64; 3], side: f64) -> InterferenceBox {
    InterferenceBox {
        id: id.to_owned(),
        count: 1,
        position: Vec3::new(center[0], center[1], center[2]),
        orientation: Quat::IDENTITY,
        size: Vec3::new(side, side, side),
        mount_ancestors: Vec::new(),
    }
}

/// 原点にある担い手。軸は +x、半頂角 30°。
fn carrier(id: &str, kind: ConeKind) -> ConeCarrier {
    ConeCarrier {
        id: id.to_owned(),
        kind,
        position: Vec3::ZERO,
        axis: Some(Vec3::X),
        half_angle: Some(Qty::from_raw(30.0 * DEG)),
        mount_ancestors: Vec::new(),
    }
}

fn input(boxes: Vec<InterferenceBox>, carriers: Vec<ConeCarrier>) -> ConeClearanceInput {
    ConeClearanceInput {
        boxes,
        unsized_parts: Vec::new(),
        carriers,
        range: None,
    }
}

fn run(input: &ConeClearanceInput) -> ConeClearanceOutput {
    ConeClearance::run(input).unwrap()
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

/// 担い手が箱を持たなければ頂点は部品の原点。一辺 1 m の立方体が軸の上 (3, 0, 0) にあれば、角度 0・距離 2.5 m の警告 1 つ。
/// 判定文は箱での近似に触れ、違反にはならない。
#[test]
fn a_part_in_the_cone_is_a_warning_with_angle_and_distance() {
    let out = run(&input(
        vec![cube("PADDLE", [3.0, 0.0, 0.0], 1.0)],
        vec![carrier("EPT", ConeKind::Plume)],
    ));
    assert_eq!((out.cones_judged, out.cones_obstructed), (1, 1));
    let o = &out.cones[0].obstructions[0];
    assert_eq!(o.part, "PADDLE");
    close(o.min_angle.value(), 0.0);
    close(o.min_distance.value(), 2.5);
    assert_eq!(out.checks.len(), 1);
    assert_eq!(out.checks[0].verdict, Verdict::Warning);
    assert_eq!(out.checks[0].key, "obstruction:EPT:PADDLE");
    assert!(out.checks[0].message.contains("box approximation"));
    assert!(out.checks.iter().all(|c| c.verdict != Verdict::Violation));
}

/// 何も入らなければ ok が 1 つ。担い手が 1 つも判定できなくても成功（ok）。
#[test]
fn clear_cones_and_no_carriers_are_ok() {
    let clear = run(&input(
        vec![cube("A", [-3.0, 0.0, 0.0], 1.0)],
        vec![carrier("EPT", ConeKind::Plume)],
    ));
    assert_eq!(clear.cones_obstructed, 0);
    assert_eq!(clear.checks.len(), 1);
    assert_eq!(clear.checks[0].verdict, Verdict::Ok);
    let empty = run(&input(Vec::new(), Vec::new()));
    assert_eq!(
        (empty.cones_judged, empty.checks[0].verdict),
        (0, Verdict::Ok)
    );
}

/// 担い手自身の箱は判定しない。箱を持つ担い手の頂点は、軸が自身の箱から出る点: 一辺 0.2 の箱が原点にあり軸が +x なら頂点は x = 0.1。
/// 距離 2.5 の立方体までの距離は、頂点から 2.5 − 0.1 = 2.4 になる（自身の箱が中心から 0.1 だけ張り出すため）。
#[test]
fn the_apex_is_where_the_axis_leaves_the_carriers_own_box() {
    let own = cube("EPT", [0.0, 0.0, 0.0], 0.2);
    let out = run(&input(
        vec![own, cube("PADDLE", [3.0, 0.0, 0.0], 1.0)],
        vec![carrier("EPT", ConeKind::Plume)],
    ));
    let c = &out.cones[0];
    close(c.apex.x, 0.1);
    assert_eq!(
        c.obstructions.len(),
        1,
        "the own box is never an obstruction"
    );
    close(c.obstructions[0].min_distance.value(), 2.4);
    assert_eq!(c.parts_judged, 1);
    // 軸が斜めなら支持量は Σ h_i |axis·u_i|: 軸 (1,1,0)/√2・箱 0.2 なら 0.1·(1/√2 + 1/√2) = 0.1·√2 で、頂点は (0.1, 0.1, 0)
    let mut diagonal = carrier("EPT", ConeKind::Plume);
    diagonal.axis = Some(Vec3::new(1.0, 1.0, 0.0));
    let out = run(&input(vec![cube("EPT", [0.0; 3], 0.2)], vec![diagonal]));
    close(out.cones[0].apex.x, 0.1);
    close(out.cones[0].apex.y, 0.1);
}

/// 取付の連鎖: 担い手の取付先の祖先（フレームの箱は担い手を含む）と、担い手を取付先とする子孫は判定せず、数える。
/// 兄弟（同じ取付先の別の部品）は判定する。
#[test]
fn mount_ancestors_and_descendants_are_not_judged_but_siblings_are() {
    let mut frame = cube("FRAME", [1.0, 0.0, 0.0], 4.0); // 担い手の頂点を含む大きな箱
    frame.mount_ancestors = Vec::new();
    let mut sibling = cube("SIBLING", [3.0, 0.0, 0.0], 1.0);
    sibling.mount_ancestors = vec!["FRAME".to_owned()];
    let mut child = cube("CHILD", [2.0, 0.0, 0.0], 0.5);
    child.mount_ancestors = vec!["EPT".to_owned(), "FRAME".to_owned()];
    let mut c = carrier("EPT", ConeKind::Plume);
    c.mount_ancestors = vec!["FRAME".to_owned()];
    let out = run(&input(vec![frame, sibling, child], vec![c]));
    let cone = &out.cones[0];
    let hit: Vec<&str> = cone.obstructions.iter().map(|o| o.part.as_str()).collect();
    assert_eq!(hit, ["SIBLING"]);
    assert_eq!((cone.parts_judged, cone.parts_in_mount_chain), (1, 2));
}

/// 半頂角が無い担い手は、判定せず理由つきで一覧に残す（既定を仮定しない）。軸が無い・値が不正なものも同じ。他の担い手の判定は続く。
#[test]
fn carriers_without_a_usable_cone_are_listed_not_assumed() {
    let mut no_half = carrier("A", ConeKind::Plume);
    no_half.half_angle = None;
    let mut no_axis = carrier("B", ConeKind::FieldOfView);
    no_axis.axis = None;
    let mut zero_axis = carrier("C", ConeKind::FieldOfView);
    zero_axis.axis = Some(Vec3::ZERO);
    let mut wide = carrier("D", ConeKind::Plume);
    wide.half_angle = Some(Qty::from_raw(90.0 * DEG));
    let mut flat = carrier("E", ConeKind::Plume);
    flat.half_angle = Some(Qty::from_raw(0.0));
    let out = run(&input(
        vec![cube("X", [3.0, 0.0, 0.0], 1.0)],
        vec![
            no_half,
            no_axis,
            zero_axis,
            wide,
            flat,
            carrier("OK", ConeKind::Plume),
        ],
    ));
    let reasons: Vec<(&str, ConeSkipReason)> = out
        .skipped_carriers
        .iter()
        .map(|s| (s.id.as_str(), s.reason))
        .collect();
    assert_eq!(
        reasons,
        [
            ("A", ConeSkipReason::NoHalfAngle),
            ("B", ConeSkipReason::NoAxis),
            ("C", ConeSkipReason::InvalidAxis),
            ("D", ConeSkipReason::InvalidHalfAngle),
            ("E", ConeSkipReason::InvalidHalfAngle),
        ]
    );
    assert_eq!((out.cones_judged, out.cones_obstructed), (1, 1));
}

/// 距離の上限: 既定は全部品の箱と担い手の位置を含む軸平行な箱の対角線。
/// 担い手 (0,0,0) と立方体 (3,0,0)（x ∈ [2.5, 3.5]、y・z ∈ [−0.5, 0.5]）なら、範囲は x ∈ [0, 3.5]・y,z ∈ [−0.5, 0.5] で、対角線は √(3.5² + 1 + 1)。
/// 上限を 2.4 に指定すれば、最短距離 2.5 の部品は入らない。
#[test]
fn range_defaults_to_the_bounding_diagonal_and_can_be_set() {
    let mut i = input(
        vec![cube("PADDLE", [3.0, 0.0, 0.0], 1.0)],
        vec![carrier("EPT", ConeKind::Plume)],
    );
    let out = run(&i);
    assert!(out.range_is_default);
    close(out.range.value(), (3.5_f64 * 3.5 + 2.0).sqrt());
    i.range = Some(Qty::from_raw(2.4));
    let short = run(&i);
    assert!(!short.range_is_default);
    assert_eq!(short.cones_obstructed, 0);
    i.range = Some(Qty::from_raw(-1.0));
    assert!(ConeClearance::run(&i).is_err());
}

/// 寸法が無い部品は一覧に残る（干渉の確認と同じ理由の型）。判定は箱を持つ部品だけ。
#[test]
fn parts_without_a_size_are_listed() {
    let mut i = input(
        vec![cube("A", [3.0, 0.0, 0.0], 1.0)],
        vec![carrier("EPT", ConeKind::Plume)],
    );
    i.unsized_parts = vec!["Z".to_owned()];
    let out = run(&i);
    assert_eq!(out.skipped.len(), 1);
    assert_eq!(out.skipped[0].id, "Z");
    assert_eq!(out.parts_with_box, 1);
}

/// 噴流と視野は同じ判定: 種類だけが違い、同じ配置なら同じ角度・距離になる。
#[test]
fn plume_and_field_of_view_share_one_judgement() {
    let boxes = || vec![cube("P", [3.0, 2.5, 0.0], 1.0)];
    let a = run(&input(boxes(), vec![carrier("X", ConeKind::Plume)]));
    let b = run(&input(boxes(), vec![carrier("X", ConeKind::FieldOfView)]));
    let (x, y) = (&a.cones[0].obstructions[0], &b.cones[0].obstructions[0]);
    close(x.min_angle.value(), y.min_angle.value());
    close(x.min_distance.value(), y.min_distance.value());
    assert_eq!(a.cones[0].kind, ConeKind::Plume);
    assert_eq!(b.cones[0].kind, ConeKind::FieldOfView);
}
