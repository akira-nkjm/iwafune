//! 面の日照割合のテスト。幾何の手計算（面積の比と、高さ `h` の縁が `h·tanθ` だけずれる影）で確かめる。値は丸めず、相対 1e-12。

use super::*;
use crate::structure::interference::InterferenceBox;

/// 受ける面は板（1 m × 1 m、厚さ 10 mm）の上面（z = 0.005、法線 +Z）。
fn plate() -> PartBox {
    part("PLATE", [0.0, 0.0, 0.0], [1.0, 1.0, 0.01])
}

const TOP: f64 = 0.005;

/// 部品 0（板）の上面の日照割合。
fn lit(others: Vec<PartBox>, d: Vec3) -> f64 {
    let mut parts = vec![plate()];
    parts.extend(others);
    let g = ExternalGeometry::new(parts);
    let face = g.face_toward(0, Vec3::Z).unwrap();
    g.lit_fraction(0, &face, d, None)
}

#[test]
fn a_box_over_half_of_the_face_leaves_half_lit() {
    // x ∈ [0, 0.5]、y ∈ [-0.5, 0.5]、高さ 0.1〜0.2 の箱。真上の光は面の半分を隠す
    let half = part("A", [0.25, 0.0, TOP + 0.15], [0.5, 1.0, 0.1]);
    assert!(close(lit(vec![half], Vec3::Z), 0.5));
}

#[test]
fn a_box_covering_the_whole_face_leaves_nothing_lit() {
    let cover = part("A", [0.0, 0.0, TOP + 0.15], [3.0, 3.0, 0.1]);
    assert_eq!(lit(vec![cover], Vec3::Z), 0.0);
}

#[test]
fn a_box_behind_the_face_does_not_shadow() {
    // 面の下（板の裏）の大きな箱
    let behind = part("A", [0.0, 0.0, -0.5], [3.0, 3.0, 0.5]);
    assert_eq!(lit(vec![behind], Vec3::Z), 1.0);
    assert_eq!(lit(vec![], Vec3::Z), 1.0);
}

#[test]
fn overlapping_shadows_are_counted_once() {
    let a = part("A", [0.25, 0.0, TOP + 0.15], [0.5, 1.0, 0.1]);
    let b = part("B", [0.25, 0.0, TOP + 0.35], [0.5, 1.0, 0.1]);
    assert!(close(lit(vec![a, b], Vec3::Z), 0.5));
}

#[test]
fn the_back_side_is_not_lit() {
    assert_eq!(lit(vec![], -Vec3::Z), 0.0);
    // 面に平行な光（cos = 0）も受けない
    assert_eq!(lit(vec![], Vec3::X), 0.0);
}

/// 面の外（x ∈ [0.5, 1.5]）にある箱の底が面の平面から 0.3、上が 0.4 の高さ。+X 側 45° の光（面から光源へ向かう `d = (1, 0, 1)/√2`）では、
/// 影は −x 側へ高さの分だけずれる。いちばん外へ（−x へ）出る影は箱の上の縁（高さ 0.4）で、x = 0.5 − 0.4 = 0.1。
/// 面（x ∈ [-0.5, 0.5]）のうち x ∈ [0.1, 0.5] の幅 0.4 が影で、日照割合は 1 − 0.4 = 0.6。真上の光では影は面に届かない。
#[test]
fn an_oblique_shadow_is_displaced_by_the_height_times_the_tangent() {
    let side = part("A", [1.0, 0.0, TOP + 0.35], [1.0, 3.0, 0.1]);
    let d = unit(1.0, 0.0, 1.0);
    assert!(close(lit(vec![side.clone()], d), 0.6));
    assert_eq!(lit(vec![side.clone()], Vec3::Z), 1.0);
    // 光が −X 側から来れば、影は +x 側（面の外）へ落ちる
    assert_eq!(lit(vec![side], unit(-1.0, 0.0, 1.0)), 1.0);
}

/// 底が面と同じ平面にある、面の外の壁（x ∈ [0.5, 0.6]、高さ 0.2）。45° の光では、壁の上の内側の縁（x = 0.5、高さ 0.2）の影が
/// x = 0.5 − 0.2 = 0.3 に落ちるので、面の x ∈ [0.3, 0.5] の幅 0.2 が影で、日照割合は 0.8。
#[test]
fn a_wall_beside_the_face_casts_a_shadow_of_its_height() {
    let wall = part("W", [0.55, 0.0, TOP + 0.1], [0.1, 3.0, 0.2]);
    assert!(close(lit(vec![wall], unit(1.0, 0.0, 1.0)), 0.8));
}

#[test]
fn a_cell_flush_on_a_panel_is_not_shadowed_by_the_panel() {
    // 受ける面は薄いセルの上面。セルは大きなパネルの上面（z = 0）に載る（セルは 0〜2 mm の高さ）
    let panel = part("PANEL", [0.0, 0.0, -0.01], [2.0, 2.0, 0.02]);
    let cell = part("CELL", [0.0, 0.0, 0.001], [0.3, 0.3, 0.002]);
    let g = ExternalGeometry::new(vec![cell, panel]);
    let face = g.face_toward(0, Vec3::Z).unwrap();
    for d in [Vec3::Z, unit(1.0, 0.0, 1.0), unit(0.3, -0.7, 0.2)] {
        assert_eq!(g.lit_fraction(0, &face, d, None), 1.0);
    }
    // パネルの上面とセルの面が同じ平面（セルがパネルの窪みに納まる）でも同じ
    let pocket = part("CELL", [0.0, 0.0, -0.001], [0.3, 0.3, 0.002]);
    let panel = part("PANEL", [0.0, 0.0, -0.01], [2.0, 2.0, 0.02]);
    let g = ExternalGeometry::new(vec![pocket, panel]);
    let face = g.face_toward(0, Vec3::Z).unwrap();
    assert_eq!(g.lit_fraction(0, &face, unit(1.0, 0.0, 1.0), None), 1.0);
}

#[test]
fn a_box_rising_only_within_the_tolerance_is_flush() {
    // 面より 0.05 mm（許容 0.1 mm の内）だけ高いパネル: 面合わせの許容として遮らない。0.5 mm 高ければ（埋まったセル）遮る
    let cell = |z: f64| part("CELL", [0.0, 0.0, z], [0.3, 0.3, 0.002]);
    let panel = |top: f64| part("PANEL", [0.0, 0.0, top - 0.01], [2.0, 2.0, 0.02]);
    let g = ExternalGeometry::new(vec![cell(0.0), panel(0.001 + 0.00005)]);
    let face = g.face_toward(0, Vec3::Z).unwrap();
    assert_eq!(g.lit_fraction(0, &face, Vec3::Z, None), 1.0);
    let g = ExternalGeometry::new(vec![cell(0.0), panel(0.001 + 0.0005)]);
    let face = g.face_toward(0, Vec3::Z).unwrap();
    assert_eq!(g.lit_fraction(0, &face, Vec3::Z, None), 0.0);
}

#[test]
fn coplanar_neighbours_do_not_shadow_each_other() {
    let a = part("A", [0.0, 0.0, 0.0], [0.5, 0.5, 0.002]);
    let b = part("B", [0.5, 0.0, 0.0], [0.5, 0.5, 0.002]);
    let g = ExternalGeometry::new(vec![a, b]);
    let (fa, fb) = (
        g.face_toward(0, Vec3::Z).unwrap(),
        g.face_toward(1, Vec3::Z).unwrap(),
    );
    for d in [Vec3::Z, unit(1.0, 0.0, 1.0), unit(-1.0, 0.0, 1.0)] {
        assert_eq!(g.lit_fraction(0, &fa, d, None), 1.0);
        assert_eq!(g.lit_fraction(1, &fb, d, None), 1.0);
    }
}

#[test]
fn a_box_does_not_shadow_its_own_face_and_a_rotated_box_is_handled() {
    // 45° 回した板の上面（法線は (0, -sin, cos)）。真上から −45° 傾いた向き（面の法線）の光なら、手前の箱が無い限り 1
    let q = Quat::from_axis_angle(Vec3::X, 45f64.to_radians());
    let tilted = rotated("T", [0.0, 0.0, 0.0], q, [1.0, 1.0, 0.01]);
    let g = ExternalGeometry::new(vec![tilted]);
    let n = q.rotate(Vec3::Z);
    let face = g.face_toward(0, n).unwrap();
    assert_eq!(g.lit_fraction(0, &face, n, None), 1.0);
    // 書かれた法線が箱の軸から 10° 外れていれば、合う面が無い
    let off = Quat::from_axis_angle(Vec3::X, 10f64.to_radians()).rotate(n);
    assert!(g.face_toward(0, off).is_none());
}

fn boxed(id: &str, c: [f64; 3], size: [f64; 3]) -> InterferenceBox {
    InterferenceBox {
        id: id.to_owned(),
        count: 1,
        position: Vec3::new(c[0], c[1], c[2]),
        orientation: Quat::IDENTITY,
        size: Vec3::new(size[0], size[1], size[2]),
        mount_ancestors: vec![],
    }
}

#[test]
fn the_surface_list_gives_the_fractions_a_summary_and_the_unshadowed() {
    let boxes = vec![
        boxed("PLATE", [0.0, 0.0, 0.0], [1.0, 1.0, 0.01]),
        boxed("HALF", [0.25, 0.0, TOP + 0.15], [0.5, 1.0, 0.1]),
    ];
    let surfaces = [("PLATE", Vec3::Z), ("NOBOX", Vec3::Z), ("HALF", Vec3::X)];
    let input = ShadowingInput {
        boxes,
        enabled: None,
    };
    let lit = LitSurfaces::new(&input, Footprint::Blocks, surfaces).unwrap();
    // 真上: 板は半分。箱の +X の面は光を受けない
    assert!(close(lit.lit(0, Vec3::Z), 0.5));
    assert_eq!(lit.lit(1, Vec3::Z), 1.0);
    assert_eq!(lit.lit(2, Vec3::Z), 0.0);
    // 同じ向きを繰り返しても同じ値（使い回し）
    assert_eq!(lit.lit(0, Vec3::Z), lit.lit(0, Vec3::new(0.0, 0.0, 2.0)));
    // −X 側 45° の光では、箱（x ∈ [0, 0.5]、高さ 0.1〜0.2）の影は +x へ 0.1〜0.2 ずれて x ∈ [0.1, 0.7]、面との共通部分は幅 0.4 で日照割合 0.6。
    // 真上は 0.5。最小 0.5、平均 0.55。裏の向きは数えない
    let dirs = [Vec3::Z, unit(-1.0, 0.0, 1.0), -Vec3::Z];
    let report = lit.summary(&dirs);
    assert_eq!(report.unshadowed, vec!["NOBOX".to_owned()]);
    let plate = &report.parts[0];
    assert_eq!((plate.id.as_str(), plate.samples), ("PLATE", 2));
    assert!(close(plate.min.value(), 0.5) && close(plate.mean.value(), 0.55));
    // 箱が無い・影を切ってある: 影を数えない
    let none = ShadowingInput {
        boxes: vec![],
        enabled: None,
    };
    assert!(LitSurfaces::new(&none, Footprint::Blocks, surfaces).is_none());
    let off = ShadowingInput {
        enabled: Some(false),
        ..input
    };
    assert!(LitSurfaces::new(&off, Footprint::Blocks, surfaces).is_none());
}

/// 面に載る箱（x ∈ [0, 0.5]、面の平面から高さ 0.1。接地面は面の半分）。真上の光では、遮る扱い（太陽電池）なら接地面の下が影で 0.5、
/// 面の一部として数える扱い（熱の外面）なら影は接地面の中だけなので 1。+X 側 45° の光では影が −x へ高さ 0.1 だけ伸びて
/// x ∈ [-0.1, 0.5]（幅 0.6）: 遮る扱いは 1 − 0.6 = 0.4、数える扱いは接地面（幅 0.5）の外の 0.1 だけを引いて 0.9。
#[test]
fn a_part_resting_on_the_face_is_credited_or_blocks_by_the_footprint_rule() {
    let resting = part("R", [0.25, 0.0, TOP + 0.05], [0.5, 1.0, 0.1]);
    let g = ExternalGeometry::new(vec![plate(), resting]);
    let face = g.face_toward(0, Vec3::Z).unwrap();
    let at = |d: Vec3, credited: Option<&[bool]>| g.lit_fraction(0, &face, d, credited);
    // 取り付けの印が無くても、面の平面に接する箱は接地面として数える
    let not_mounted = [false, false];
    let oblique = unit(1.0, 0.0, 1.0);
    assert!(close(at(Vec3::Z, None), 0.5));
    assert!(close(at(Vec3::Z, Some(&not_mounted)), 1.0));
    assert!(close(at(oblique, None), 0.4));
    assert!(close(at(oblique, Some(&not_mounted)), 0.9));
    // 面から浮いた箱（高さ 0.1〜0.2）には、どちらの扱いでも差が無い
    let hover = part("H", [0.25, 0.0, TOP + 0.15], [0.5, 1.0, 0.1]);
    let g = ExternalGeometry::new(vec![plate(), hover]);
    assert!(close(g.lit_fraction(0, &face, Vec3::Z, None), 0.5));
    assert!(close(
        g.lit_fraction(0, &face, Vec3::Z, Some(&not_mounted)),
        0.5
    ));
}

/// 面の部品に取り付けられた部品が、取付の隙間（0.45 mm。許容 0.1 mm より大きい）で面から浮いていても、真下は面の一部として数える
/// （取付の連鎖で決める。許容に頼らない）。印が無ければ浮いた箱は遮る。+X 側 45° では、箱の底（高さ 0.00045）と上（0.10045）の影が
/// x ∈ [-0.10045, 0.49955]、接地面 [0, 0.5] との和は幅 0.60045 で、接地面を引いた 0.10045 が影: 日照割合 0.89955。
#[test]
fn a_mounted_part_floating_by_the_mount_gap_is_credited_by_the_mount_chain() {
    let floating = part("F", [0.25, 0.0, TOP + 0.00045 + 0.05], [0.5, 1.0, 0.1]);
    let g = ExternalGeometry::new(vec![plate(), floating]);
    let face = g.face_toward(0, Vec3::Z).unwrap();
    let (mounted, loose) = ([false, true], [false, false]);
    assert!(close(
        g.lit_fraction(0, &face, Vec3::Z, Some(&mounted)),
        1.0
    ));
    assert!(close(g.lit_fraction(0, &face, Vec3::Z, Some(&loose)), 0.5));
    let oblique = unit(1.0, 0.0, 1.0);
    assert!(close(
        g.lit_fraction(0, &face, oblique, Some(&mounted)),
        0.89955
    ));
}

/// 取付の連鎖（`mount_ancestors`）が面の部品を含む箱だけが、熱の外面の「面に載る部品」になる。
#[test]
fn the_mount_chain_of_the_input_boxes_decides_which_parts_sit_on_the_face() {
    let build = |ancestors: Vec<String>, footprint: Footprint| {
        let mut cell = boxed("CELL", [0.25, 0.0, TOP + 0.00045 + 0.05], [0.5, 1.0, 0.1]);
        cell.mount_ancestors = ancestors;
        let input = ShadowingInput {
            boxes: vec![boxed("PLATE", [0.0, 0.0, 0.0], [1.0, 1.0, 0.01]), cell],
            enabled: None,
        };
        LitSurfaces::new(&input, footprint, [("PLATE", Vec3::Z)]).unwrap()
    };
    let chain = vec!["PLATE".to_owned()];
    assert!(close(
        build(chain.clone(), Footprint::Credited).lit(0, Vec3::Z),
        1.0
    ));
    assert!(close(build(chain, Footprint::Blocks).lit(0, Vec3::Z), 0.5));
    assert!(close(
        build(vec![], Footprint::Credited).lit(0, Vec3::Z),
        0.5
    ));
}
