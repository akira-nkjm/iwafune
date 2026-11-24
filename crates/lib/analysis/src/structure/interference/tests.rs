//! `structure.interference` のテスト（手計算との比較）。

use super::*;
use crate::structure::acknowledged::Acknowledgement;

fn unit_box(id: &str, center: [f64; 3]) -> InterferenceBox {
    InterferenceBox {
        id: id.to_owned(),
        count: 1,
        position: Vec3::new(center[0], center[1], center[2]),
        orientation: Quat::IDENTITY,
        size: Vec3::new(1.0, 1.0, 1.0),
        mount_ancestors: Vec::new(),
    }
}

fn input(boxes: Vec<InterferenceBox>) -> InterferenceInput {
    InterferenceInput {
        boxes,
        unsized_parts: Vec::new(),
        tolerance: None,
        acknowledged: Vec::new(),
    }
}

fn run(input: &InterferenceInput) -> InterferenceOutput {
    Interference::run(input).unwrap()
}

/// 一辺 1 m の立方体 2 つ。中心の距離 0.9 → 深さ 0.1 m（x 方向）で警告。
#[test]
fn overlapping_cubes_are_a_warning_with_depth_and_axis() {
    let out = run(&input(vec![
        unit_box("A", [0.0; 3]),
        unit_box("B", [0.9, 0.0, 0.0]),
    ]));
    assert_eq!(out.overlaps.len(), 1);
    let p = &out.overlaps[0];
    assert_eq!((p.first.as_str(), p.second.as_str()), ("A", "B"));
    assert!((p.depth.value() - 0.1).abs() < 1e-12);
    assert!((p.separation_axis.x - 1.0).abs() < 1e-12 && p.separation_axis.y.abs() < 1e-12);
    assert_eq!((out.parts_judged, out.pairs_judged), (2, 1));
    assert_eq!(out.checks.len(), 1);
    assert_eq!(out.checks[0].verdict, Verdict::Warning);
    assert_eq!(out.checks[0].key, "overlap:A:B");
    assert!(out.checks[0].message.contains("box approximation"));
    assert!(out.min_clearance.is_none());
}

/// 許容差 0.1 mm（既定）: 中心の距離 1.0（接触）と 1.0 − 0.05 mm（深さ 0.05 mm）は報告しない。
/// 1.0 − 0.2 mm は深さ 0.2 mm で報告する。ちょうど許容差（深さ 0.1 mm）は報告しない（超えたときだけ）。
#[test]
fn tolerance_separates_contact_from_interference() {
    let apart = |distance: f64| {
        run(&input(vec![
            unit_box("A", [0.0; 3]),
            unit_box("B", [distance, 0.0, 0.0]),
        ]))
    };
    assert!(apart(1.0).overlaps.is_empty());
    assert!(apart(1.0 - 5e-5).overlaps.is_empty());
    assert!(apart(1.0 - 1e-4 + 1e-9).overlaps.is_empty());
    let out = apart(1.0 - 2e-4);
    assert_eq!(out.overlaps.len(), 1);
    assert!((out.overlaps[0].depth.value() - 2e-4).abs() < 1e-12);
    // 接していて報告しなかった組は、最小の隙間 0 として残る
    let touching = apart(1.0);
    let c = touching.min_clearance.unwrap();
    assert_eq!(c.gap.value(), 0.0);
    assert_eq!(touching.checks[0].verdict, Verdict::Ok);
}

/// 許容差は入力で変えられる（1 cm なら、深さ 5 mm は報告しない。0 なら 1e-9 でも報告する）。
#[test]
fn tolerance_is_an_input() {
    let mut i = input(vec![
        unit_box("A", [0.0; 3]),
        unit_box("B", [0.995, 0.0, 0.0]),
    ]);
    assert_eq!(run(&i).overlaps.len(), 1);
    i.tolerance = Some(Qty::from_raw(0.01));
    let out = run(&i);
    assert!(out.overlaps.is_empty());
    assert_eq!(out.tolerance.value(), 0.01);
    i.tolerance = Some(Qty::from_raw(-1.0));
    assert!(Interference::run(&i).is_err());
}

/// 隙間は、報告しなかった組のうち最小のもの（A–B は 0.3、B–C は 0.1 → B–C）。
#[test]
fn smallest_clearance_among_non_overlapping_pairs() {
    let out = run(&input(vec![
        unit_box("A", [0.0; 3]),
        unit_box("B", [1.3, 0.0, 0.0]),
        unit_box("C", [2.4, 0.0, 0.0]),
    ]));
    assert!(out.overlaps.is_empty());
    let c = out.min_clearance.unwrap();
    assert_eq!((c.first.as_str(), c.second.as_str()), ("B", "C"));
    assert!((c.gap.value() - 0.1).abs() < 1e-12);
    assert_eq!(out.pairs_judged, 3);
}

/// 取付の祖先・子孫の組は判定しない（間の部品が寸法を持たなくてもよい）。兄弟と別の枝の組は判定する。
///
/// FRAME（大きな箱）の中に A・B が収まる。A の子 C（A の取付先は FRAME。C の取付先は A）。
/// A と B は兄弟で重なっている → 報告する。FRAME との組・A と C の組は判定しない。
#[test]
fn mount_ancestors_are_not_judged_but_siblings_are() {
    let mut frame = unit_box("FRAME", [0.0; 3]);
    frame.size = Vec3::new(10.0, 10.0, 10.0);
    let mut a = unit_box("A", [0.0; 3]);
    a.mount_ancestors = vec!["FRAME".to_owned()];
    let mut b = unit_box("B", [0.5, 0.0, 0.0]);
    b.mount_ancestors = vec!["FRAME".to_owned()];
    // C は寸法の無い中間部品 MID を介して A に取り付く（祖先の列は近いほうから）
    let mut c = unit_box("C", [0.0, 0.0, 0.0]);
    c.mount_ancestors = vec!["MID".to_owned(), "A".to_owned(), "FRAME".to_owned()];
    let out = run(&input(vec![frame, a, b, c]));
    let pairs: Vec<(&str, &str)> = out
        .overlaps
        .iter()
        .map(|p| (p.first.as_str(), p.second.as_str()))
        .collect();
    // C は B と別の枝なので、B との組は判定される。A–B、B–C はどちらも深さ 0.5（x 方向の中心の距離 0.5）
    assert_eq!(pairs, vec![("A", "B"), ("B", "C")]);
    // 判定しない組: FRAME–A、FRAME–B、FRAME–C、A–C の 4 つ。判定する組: A–B、B–C の 2 つ
    assert_eq!(out.pairs_in_mount_chain, 4);
    assert_eq!(out.pairs_judged, 2);
}

/// 取付の連鎖に無い箱どうしは、箱の中に完全に入っていても判定する（親子ではなく収容を決めるのは設計）。
#[test]
fn contained_non_ancestor_is_reported() {
    let mut big = unit_box("BIG", [0.0; 3]);
    big.size = Vec3::new(4.0, 4.0, 4.0);
    let out = run(&input(vec![big, unit_box("IN", [0.2, 0.0, 0.0])]));
    assert_eq!(out.overlaps.len(), 1);
    // 小さな箱（辺 1、中心 +0.2）が大きな箱（半幅 2）から +x へ出るのに要る移動: 2 + 0.5 − 0.2 = 2.3
    assert!((out.overlaps[0].depth.value() - 2.3).abs() < 1e-12);
}

/// 員数が 2 以上の部品は 1 つの箱として判定する（自分自身とは比べない）。一覧に出す。
#[test]
fn repeated_part_is_one_box_and_not_compared_with_itself() {
    let mut a = unit_box("A", [0.0; 3]);
    a.count = 4;
    let out = run(&input(vec![a, unit_box("B", [3.0, 0.0, 0.0])]));
    assert!(out.overlaps.is_empty());
    assert_eq!(out.pairs_judged, 1);
    assert_eq!(out.repeated.len(), 1);
    assert_eq!(
        (out.repeated[0].id.as_str(), out.repeated[0].count),
        ("A", 4)
    );
}

/// 寸法の無い部品は一覧に残す。寸法が正でない箱も判定せず一覧に残す。箱が 2 つに満たなければ重なり無しの成功（ok）。
#[test]
fn parts_without_a_size_are_listed_and_fewer_than_two_boxes_succeed() {
    let mut zero = unit_box("Z", [0.0; 3]);
    zero.size = Vec3::new(1.0, 0.0, 1.0);
    let mut i = input(vec![unit_box("A", [0.0; 3]), zero]);
    i.unsized_parts = vec!["WIRE".to_owned(), "BATTERY".to_owned()];
    let out = run(&i);
    assert!(out.overlaps.is_empty());
    assert_eq!((out.parts_judged, out.pairs_judged), (1, 0));
    let ids: Vec<(&str, InterferenceSkipReason)> = out
        .skipped
        .iter()
        .map(|s| (s.id.as_str(), s.reason))
        .collect();
    assert_eq!(
        ids,
        vec![
            ("BATTERY", InterferenceSkipReason::NoSize),
            ("WIRE", InterferenceSkipReason::NoSize),
            ("Z", InterferenceSkipReason::InvalidSize)
        ]
    );
    assert_eq!(out.checks.len(), 1);
    assert_eq!(out.checks[0].verdict, Verdict::Ok);
    assert!(out.min_clearance.is_none());
    // 箱が 1 つも無くても失敗しない
    let empty = run(&input(Vec::new()));
    assert_eq!((empty.parts_judged, empty.pairs_judged), (0, 0));
    assert_eq!(empty.checks[0].verdict, Verdict::Ok);
}

/// 入力の順に依らない: 順を入れ替えても出力は同じ。深い順に並び、どの重なりも警告で、違反は出さない。
#[test]
fn result_does_not_depend_on_input_order_and_is_never_a_violation() {
    let boxes = vec![
        unit_box("A", [0.0; 3]),
        unit_box("B", [0.9, 0.0, 0.0]),
        unit_box("C", [0.0, 0.5, 0.0]),
        unit_box("D", [5.0, 5.0, 5.0]),
    ];
    let forward = run(&input(boxes.clone()));
    let mut reversed_boxes = boxes;
    reversed_boxes.reverse();
    let reversed = run(&input(reversed_boxes));
    let key = |o: &InterferenceOutput| {
        o.overlaps
            .iter()
            .map(|p| (p.first.clone(), p.second.clone(), p.depth.value().to_bits()))
            .collect::<Vec<_>>()
    };
    assert_eq!(key(&forward), key(&reversed));
    // A–C は深さ 0.5（y）、A–B は 0.1（x）、B–C は B(0.9,0,0) と C(0,0.5,0): x の重なり 0.1・y の重なり 0.5 → 0.1
    let depths: Vec<f64> = forward.overlaps.iter().map(|p| p.depth.value()).collect();
    assert_eq!(forward.overlaps.len(), 3);
    assert!(depths.windows(2).all(|w| w[0] >= w[1]));
    assert!((depths[0] - 0.5).abs() < 1e-12);
    assert!(forward.checks.iter().all(|c| c.verdict == Verdict::Warning));
}

/// 出力は JSON に書いて読み戻せる（`fune analyze` の入力に使う形）。
#[test]
fn input_round_trips_through_json() {
    let i = input(vec![unit_box("A", [0.0; 3])]);
    let text = serde_json::to_string(&i).unwrap();
    let back: InterferenceInput = serde_json::from_str(&text).unwrap();
    assert_eq!(back.boxes.len(), 1);
    assert_eq!(back.boxes[0].count, 1);
}

fn ack(first: &str, second: &str, reason: &str) -> Acknowledgement {
    Acknowledgement {
        first: first.to_owned(),
        second: second.to_owned(),
        reason: reason.to_owned(),
        configuration: None,
    }
}

/// 承知済みの組（どちらの並びで書いても同じ）は、重なりとして残り（フラグと理由つき）、判定は ok。承知していない組は警告のまま。
#[test]
fn an_acknowledged_pair_is_listed_with_its_reason_and_does_not_warn() {
    let mut i = input(vec![
        unit_box("A", [0.0; 3]),
        unit_box("B", [0.9, 0.0, 0.0]),
        unit_box("C", [-0.9, 0.0, 0.0]),
    ]);
    // A–B と A–C が重なる。A–B だけを（並びを逆にして）承知する
    i.acknowledged = vec![ack("B", "A", "hinge edge")];
    let out = run(&i);
    assert_eq!(out.overlaps.len(), 2);
    let ab = out.overlaps.iter().find(|p| p.second == "B").unwrap();
    assert!(ab.acknowledged);
    assert_eq!(ab.reason.as_deref(), Some("hinge edge"));
    let ac = out.overlaps.iter().find(|p| p.second == "C").unwrap();
    assert!(!ac.acknowledged && ac.reason.is_none());
    let verdict = |key: &str| out.checks.iter().find(|c| c.key == key).unwrap();
    assert_eq!(verdict("overlap:A:B").verdict, Verdict::Ok);
    assert!(
        verdict("overlap:A:B")
            .message
            .starts_with("Acknowledged: hinge edge. ")
    );
    assert_eq!(verdict("overlap:A:C").verdict, Verdict::Warning);
}

/// すべて承知済みなら、警告は無い（判定は組ごとの ok。重なりは出力に残る）。
#[test]
fn when_every_overlap_is_acknowledged_nothing_warns() {
    let mut i = input(vec![
        unit_box("A", [0.0; 3]),
        unit_box("B", [0.9, 0.0, 0.0]),
    ]);
    i.acknowledged = vec![ack("A", "B", "known")];
    let out = run(&i);
    assert_eq!(out.overlaps.len(), 1);
    assert!(out.checks.iter().all(|c| c.verdict == Verdict::Ok));
    assert_eq!(
        Verdict::worst(out.checks.iter().map(|c| c.verdict)),
        Some(Verdict::Ok)
    );
}

fn sized(id: &str, center: [f64; 3], size: [f64; 3]) -> InterferenceBox {
    InterferenceBox {
        size: Vec3::new(size[0], size[1], size[2]),
        ..unit_box(id, center)
    }
}

/// 隣り合う 2 枚のパネル（厚さ 2 mm）が縁で半厚ずつ重なる（一辺 1 m の面に中心を置く: x ∈ [0.499, 0.501] と y ∈ [0.499, 0.501]）。
/// 重なりは 1 mm × 1 mm × 1 m で、縁の接触として別に残り、干渉の警告にはならない。深さは 1 mm（x にも y にも 1 mm）。
/// 縁の接触があっても組は「判定した組」に数える。
#[test]
fn panels_meeting_at_a_corner_are_listed_as_an_edge_contact_not_a_warning() {
    let out = run(&input(vec![
        sized("PX", [0.5, 0.0, 0.0], [0.002, 1.0, 1.0]),
        sized("PY", [0.0, 0.5, 0.0], [1.0, 0.002, 1.0]),
    ]));
    assert!(out.overlaps.is_empty());
    assert_eq!(out.edge_contacts.len(), 1);
    let e = &out.edge_contacts[0];
    assert_eq!((e.first.as_str(), e.second.as_str()), ("PX", "PY"));
    assert!((e.depth.value() - 1e-3).abs() < 1e-12);
    assert!((e.thickness.value() - 2e-3).abs() < 1e-15);
    assert_eq!(out.pairs_judged, 1);
    assert_eq!(out.checks.len(), 1);
    assert_eq!(out.checks[0].verdict, Verdict::Ok);
    assert!(out.checks[0].message.contains("1 edge contacts"));
}

/// 同じ板に、一辺 40 mm のセンサが 5 mm 食い込む（板 z ∈ [-0.001, 0.001]、センサ z ∈ [-0.004, 0.036]）: 縁の接触ではなく、干渉の警告。
#[test]
fn a_sensor_sunk_into_a_panel_is_still_an_overlap() {
    let out = run(&input(vec![
        sized("PANEL", [0.0; 3], [1.0, 1.0, 0.002]),
        sized("SENSOR", [0.0, 0.0, 0.016], [0.04, 0.04, 0.04]),
    ]));
    assert!(out.edge_contacts.is_empty());
    assert_eq!(out.overlaps.len(), 1);
    assert_eq!(out.checks[0].verdict, Verdict::Warning);
}

/// 縁の接触の組を承知していても、承知は効かない（所見ではないので `overlaps` に出ない。古い承知として別に知らせる）。
#[test]
fn an_acknowledgement_of_an_edge_contact_matches_nothing() {
    let mut i = input(vec![
        sized("PX", [0.5, 0.0, 0.0], [0.002, 1.0, 1.0]),
        sized("PY", [0.0, 0.5, 0.0], [1.0, 0.002, 1.0]),
    ]);
    i.acknowledged = vec![Acknowledgement {
        first: "PX".to_owned(),
        second: "PY".to_owned(),
        reason: "r".to_owned(),
        configuration: None,
    }];
    let out = run(&i);
    assert!(out.overlaps.is_empty());
    assert_eq!(out.edge_contacts.len(), 1);
}
