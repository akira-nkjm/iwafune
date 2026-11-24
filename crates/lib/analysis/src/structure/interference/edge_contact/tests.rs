//! 縁の接触の規則のテスト（手計算との比較）。数値は m。許容差は既定の 0.1 mm。

use super::*;
use sat_astro::math::{Quat, Vec3};

const TOL: f64 = 1e-4;

fn plate(center: [f64; 3], size: [f64; 3]) -> OrientedBox {
    OrientedBox::from_size(
        Vec3::new(center[0], center[1], center[2]),
        Quat::IDENTITY,
        Vec3::new(size[0], size[1], size[2]),
    )
}

/// 一辺 1 m の立方体の +x の面にある、厚さ `t` の板（面に中心を置く）。
fn x_plate(t: f64, centre_x: f64) -> OrientedBox {
    plate([centre_x, 0.0, 0.0], [t, 1.0, 1.0])
}

/// 同じく +y の面にある板。
fn y_plate(t: f64, centre_y: f64) -> OrientedBox {
    plate([0.0, centre_y, 0.0], [1.0, t, 1.0])
}

/// 隣り合う 2 枚のパネル（厚さ 2 mm、面に中心を置く）が縁で重なる。
/// A: x ∈ [0.499, 0.501]、B: y ∈ [0.499, 0.501]。重なりは x ∈ [0.499, 0.5]・y ∈ [0.499, 0.5]・z 全体（1 m）= 1 mm × 1 mm × 1 m。
/// 板厚 t = 2 mm。A の軸 x（長さ 1 mm、範囲の下端に接する）・軸 y（1 mm、上端に接する）の 2 本が細く、B も同じ。
/// 体積の上限 1e-3 · 1e-3 · 1 = 1e-6 m³ ≤ 2 %（板の体積 2e-3 m³ の 2 % = 4e-5 m³）。
#[test]
fn neighbouring_panels_meeting_at_a_corner_are_an_edge_contact() {
    let basis = classify(&x_plate(0.002, 0.5), &y_plate(0.002, 0.5), TOL).unwrap();
    assert!((basis.thickness - 0.002).abs() < 1e-15);
    assert!((basis.volume_bound - 1e-6).abs() < 1e-12);
    // 組の順を入れ替えても同じ
    assert!(classify(&y_plate(0.002, 0.5), &x_plate(0.002, 0.5), TOL).is_some());
}

/// 向きを変えても同じ（2 枚とも z 軸まわりに 30° 回す。重なりの区間は箱の軸への射影なので、軸が合ったまま）。
#[test]
fn the_rule_does_not_depend_on_the_orientation_of_the_pair() {
    let q = Quat::from_axis_angle(Vec3::Z, 30.0_f64.to_radians());
    let turn = |b: OrientedBox| OrientedBox {
        center: q.rotate(b.center),
        orientation: q.mul(b.orientation),
        half_extent: b.half_extent,
    };
    assert!(classify(&turn(x_plate(0.002, 0.5)), &turn(y_plate(0.002, 0.5)), TOL).is_some());
    // 食い込みのほうも、回しても縁の接触にならない（下の `sunk_*` と同じ配置）
    let sensor = plate([0.0, 0.0, 0.016], [0.04, 0.04, 0.04]);
    let panel = plate([0.0, 0.0, 0.0], [1.0, 1.0, 0.002]);
    assert!(classify(&turn(panel), &turn(sensor), TOL).is_none());
}

/// 厚さ以内の重なり方の境界。A（2 mm）は x ∈ [0.498, 0.5]（全体が B の x の範囲に入り、上端に接する）、B（2 mm）は y ∈ [0.498, 0.5]。
/// 重なりは 2 mm × 2 mm × 1 m で、断面の各軸の長さ 2 mm = 板厚 t = 2 mm（ちょうど。「以下」）なので縁の接触。
/// B を 4 mm（y ∈ [0.496, 0.5]、中心 0.498）にすると、A の軸 y の区間が 4 mm > t = 2 mm（2 つの箱の最短の辺の小さいほう）になり、
/// 断面が薄くない → 干渉。
#[test]
fn the_overlap_may_be_as_deep_as_the_thickness_but_not_deeper() {
    let a = x_plate(0.002, 0.499);
    assert!(classify(&a, &y_plate(0.002, 0.499), TOL).is_some());
    let b = plate([0.0, 0.498, 0.0], [1.0, 0.004, 1.0]);
    assert!(classify(&a, &b, TOL).is_none());
}

/// 面に食い込む部品: 厚さ 2 mm の板（z ∈ [-0.001, 0.001]）に、一辺 40 mm の立方体のセンサが 5 mm 食い込む
/// （センサの z ∈ [-0.004, 0.036]、板を貫く）。板の軸 z の重なりは 2 mm（細い）だが、軸 x・y の重なりは 40 mm > t = 2 mm
/// （t は板の 2 mm とセンサの 40 mm の小さいほう）。細い軸は 1 本だけ → 干渉。
/// 0.5 mm だけ食い込む場合（センサの z ∈ [0.0005, 0.0405]）も、面内は 40 mm のままで干渉。
#[test]
fn a_part_sunk_into_a_panel_face_is_not_an_edge_contact() {
    let panel = plate([0.0, 0.0, 0.0], [1.0, 1.0, 0.002]);
    let through = plate([0.0, 0.0, 0.016], [0.04, 0.04, 0.04]);
    assert!(classify(&panel, &through, TOL).is_none());
    let shallow = plate([0.0, 0.0, 0.0205], [0.04, 0.04, 0.04]);
    assert!(classify(&panel, &shallow, TOL).is_none());
}

/// 面内も薄い小さな部品が面に食い込む（一辺 1 mm の箱が、板の中に全部入る）。重なりは箱そのもの（1 mm の立方体）。
/// 板の中ほど: 板の側は、どの軸の区間も板の範囲の端に接しない（厚さ方向は端まで 0.5 mm、面内は 0.5 m）ので断面が端に無い → 干渉。
/// 板の角（x = y = 0.5）に寄せても、重なりの 3 軸はどれも 1 mm（縁の方向が断面の 4 倍に満たない）で、
/// 体積の上限 1e-9 m³ も小さいほうの箱（部品の 1e-9 m³）の 2 % を超える（重なりが部品の全体）ので干渉。
#[test]
fn a_tiny_part_inside_a_panel_is_not_an_edge_contact() {
    let panel = plate([0.0, 0.0, 0.0], [1.0, 1.0, 0.002]);
    let tiny = plate([0.0, 0.0, 0.0], [0.001, 0.001, 0.001]);
    assert!(classify(&panel, &tiny, TOL).is_none());
    // 板の角（x = y = 0.5、z は板の厚さの中）にぴったり寄せる
    let at_corner = plate([0.4995, 0.4995, 0.0], [0.001, 0.001, 0.001]);
    assert!(classify(&panel, &at_corner, TOL).is_none());
}

/// 板の縁を、別の板の面の中ほどへ 2 mm 差し込む（T 字）。A は x ∈ [0.496, 0.498]（厚さ 2 mm）、B は y ∈ [0.498, 0.502]。
/// 重なりは x ∈ [0.496, 0.498]・y ∈ [0.498, 0.5]・z 全体 = 2 mm × 2 mm × 1 m。A 側は細い軸が 2 本（x は A の両端に接し、y は上端に接する）だが、
/// B 側は x の区間が B の範囲（±0.5）の端に接しない（上端まで 2 mm 離れる）ので細い軸が y の 1 本だけ → 干渉。
#[test]
fn a_plate_edge_stuck_into_the_middle_of_another_plate_is_not_an_edge_contact() {
    let a = x_plate(0.002, 0.497);
    let b = plate([0.0, 0.5, 0.0], [1.0, 0.004, 1.0]);
    assert!(classify(&a, &b, TOL).is_none());
}

/// 同じ板を重ねる（中心が同じ・半厚ずらす）: 重なりは厚さ方向だけが細く、面内の 2 方向は板全体（1 m）→ 干渉。
#[test]
fn stacked_plates_are_not_an_edge_contact() {
    let p = plate([0.0, 0.0, 0.0], [1.0, 1.0, 0.002]);
    assert!(classify(&p, &p, TOL).is_none());
    // 厚さの半分（1 mm）ずらして重ねる: 重なりは 1 mm × 1 m × 1 m。細いのは 1 本
    let shifted = plate([0.0, 0.0, 0.001], [1.0, 1.0, 0.002]);
    assert!(classify(&p, &shifted, TOL).is_none());
}

/// 広く重なる箱（一辺 1 m の立方体を 0.5 m ずらす）: 重なりは 0.5 m × 1 m × 1 m。t = 1 m なので 3 軸とも t 以下で範囲の端に接するが、
/// 縁の方向（1 m）が断面の長いほう（1 m）の 4 倍に満たない。体積の上限 0.5 m³ も 2 % × 1 m³ = 0.02 m³ を超えるので干渉。
#[test]
fn broadly_overlapping_boxes_are_not_an_edge_contact() {
    let a = plate([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let b = plate([0.5, 0.0, 0.0], [1.0, 1.0, 1.0]);
    assert!(classify(&a, &b, TOL).is_none());
}

/// 立方体どうしが面で少し食い込む（中心の距離 1 m − 0.2 mm。重なりは 0.2 mm × 1 m × 1 m）: 体積の上限 2e-4 m³ は 2 % 以内、
/// 3 軸とも t = 1 m 以下で範囲の端に接するが、縁の方向（1 m）が断面の長いほう（1 m）の 4 倍に満たない（1 倍）ので干渉。
#[test]
fn a_slight_face_overlap_of_fat_boxes_is_not_an_edge_contact() {
    let a = plate([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let b = plate([1.0 - 2e-4, 0.0, 0.0], [1.0, 1.0, 1.0]);
    assert!(classify(&a, &b, TOL).is_none());
}

/// 体積の割合の境界（一辺 1 m の立方体 2 つが縁で `e` ずつ食い込む。重なり = e × e × 1 m、体積の上限 e²、箱の体積 1 m³。
/// 縁の方向 1 m は断面 e の 4 倍以上）。e = 0.14 → 0.0196 ≤ 0.02 で縁の接触。e = 0.15 → 0.0225 > 0.02 で干渉。
#[test]
fn the_volume_fraction_threshold_separates_slivers_from_overlaps() {
    let cubes = |e: f64| {
        let a = plate([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        // B は (1 − e, 1 − e) へずらす: x ∈ [0.5 − e, 1.5 − e]、y 同様 → 重なりは x・y が e
        let b = plate([1.0 - e, 1.0 - e, 0.0], [1.0, 1.0, 1.0]);
        classify(&a, &b, TOL)
    };
    assert!(cubes(0.14).is_some());
    assert!(cubes(0.15).is_none());
}
