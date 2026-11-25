//! 面を覆う部品のテスト。幾何の手計算（面積の比と、高さ `h` の縁が `h·tanθ` だけずれる影）で確かめる。値は丸めず、相対 1e-12。
//! 受ける面は板（1 m × 1 m、厚さ 10 mm）の上面（z = 0.005、法線 +Z）。

use super::lit::FaceOccluders;
use super::*;

const TOP: f64 = 0.005;

fn plate() -> PartBox {
    part("PLATE", [0.0, 0.0, 0.0], [1.0, 1.0, 0.01])
}

/// 板の上面に覆いを調べて作る（`mounted` は箱ごとの「面の部品に取り付けられている」印。板に続く箱の並びと同じ順）。
fn occluders(others: Vec<PartBox>, mounted: &[bool]) -> FaceOccluders {
    let mut parts = vec![plate()];
    parts.extend(others);
    let g = ExternalGeometry::new(parts);
    let face = g.face_toward(0, Vec3::Z).unwrap();
    let mut flags = vec![false];
    flags.extend_from_slice(mounted);
    FaceOccluders::new(&g, 0, &face, Some(&flags)).with_covers(&g, 0, &face, &flags)
}

fn covered(others: Vec<PartBox>) -> f64 {
    let n = others.len();
    occluders(others, &vec![false; n]).covered_fraction()
}

/// x ∈ [0, 0.5]、全幅の箱が面に載る（底が面の平面。高さ 0.1）と、面の半分（0.5）が覆われる。
#[test]
fn a_box_resting_on_half_of_the_face_covers_half() {
    let resting = part("A", [0.25, 0.0, TOP + 0.05], [0.5, 1.0, 0.1]);
    assert!(close(covered(vec![resting]), 0.5));
}

/// 覆う面積は、面の外へはみ出した分を数えない（面の長方形で切る）。重なる覆いは 1 度だけ数える。
#[test]
fn covers_are_clipped_to_the_face_and_counted_once() {
    let wide = part("A", [0.75, 0.0, TOP + 0.05], [1.5, 3.0, 0.1]); // x ∈ [0, 1.5] → 面の x ∈ [0, 0.5]
    assert!(close(covered(vec![wide.clone()]), 0.5));
    let again = part("B", [0.35, 0.0, TOP + 0.05], [0.3, 1.0, 0.1]); // x ∈ [0.2, 0.5]（A に含まれる）
    assert!(close(covered(vec![wide, again]), 0.5));
}

/// 覆いは面に接する・横切る箱だけ: 面から 1 mm 浮く箱（許容 0.1 mm を超える）、面の裏から接する親の箱（面の平面より手前に出ない）、
/// 面と同じ平面に並ぶ隣の箱（辺で接するだけ）は覆わない。面を貫く箱（底が面の下 2 mm）は断面を覆う。
#[test]
fn only_boxes_touching_or_crossing_the_plane_cover() {
    let floating = part("A", [0.25, 0.0, TOP + 0.001 + 0.05], [0.5, 1.0, 0.1]);
    assert_eq!(covered(vec![floating]), 0.0);
    let parent = part("A", [0.0, 0.0, TOP - 0.5], [3.0, 3.0, 1.0]); // 上面が面の平面
    assert_eq!(covered(vec![parent]), 0.0);
    let neighbour = part("A", [1.0, 0.0, 0.0], [1.0, 1.0, 0.01]); // x ∈ [0.5, 1.5]、面と同じ高さ
    assert_eq!(covered(vec![neighbour]), 0.0);
    let piercing = part("A", [0.25, 0.0, TOP + 0.05 - 0.002], [0.5, 1.0, 0.104]);
    assert!(close(covered(vec![piercing]), 0.5));
}

/// 面の部品に取り付けられた部品（面の光学特性に含まれる）は、載っていても覆いに数えない。
#[test]
fn a_part_mounted_on_the_face_part_does_not_cover() {
    let resting = part("A", [0.25, 0.0, TOP + 0.05], [0.5, 1.0, 0.1]);
    assert_eq!(
        occluders(vec![resting.clone()], &[true]).covered_fraction(),
        0.0
    );
    assert!(close(
        occluders(vec![resting], &[false]).covered_fraction(),
        0.5
    ));
}

/// 覆われた範囲は日照割合にも入るが、影と二重に数えない。覆い（x ∈ [0, 0.5]、高さ 0.1）に +X 側 45° の光（`d = (1, 0, 1)/√2`）が当たると、
/// 影は −x 側へ高さの分（0.1）ずれて x ∈ [−0.1, 0.5]（幅 0.6）。覆いの範囲 x ∈ [0, 0.5] は影に含まれるので、日照割合は 1 − 0.6 = 0.4
/// （影と覆いの和集合）。真上の光では影が覆いと一致し 0.5。覆われた割合は光によらず 0.5。
/// 覆いを調べなければ、面に載る箱の接地面は日照のまま数える（`Footprint::Credited`）ので、影のうち接地面の外の幅 0.1 だけが暗く、日照割合は 0.9。
#[test]
fn the_lit_fraction_does_not_count_the_cover_twice() {
    let resting = part("A", [0.25, 0.0, TOP + 0.05], [0.5, 1.0, 0.1]);
    let with = occluders(vec![resting.clone()], &[false]);
    let d = unit(1.0, 0.0, 1.0);
    assert!(close(with.lit_fraction(d), 0.4));
    assert!(close(with.lit_fraction(Vec3::Z), 0.5));
    assert!(close(with.covered_fraction(), 0.5));
    let mut parts = vec![plate()];
    parts.push(resting);
    let g = ExternalGeometry::new(parts);
    let face = g.face_toward(0, Vec3::Z).unwrap();
    assert!(close(
        FaceOccluders::new(&g, 0, &face, Some(&[false, false])).lit_fraction(d),
        0.9
    ));
    // 覆う部品を無いものとすれば（`keep` が偽）、覆いも影も無い
    assert_eq!(with.lit_fraction_among(d, &|_| false), 1.0);
    assert_eq!(with.covered_fraction_among(&|_| false), 0.0);
}

/// 覆いを調べない（`with_covers` を呼ばない）と、覆われた割合は 0（太陽電池の発電の面）。
#[test]
fn without_with_covers_nothing_is_covered() {
    let resting = part("A", [0.25, 0.0, TOP + 0.05], [0.5, 1.0, 0.1]);
    let g = ExternalGeometry::new(vec![plate(), resting]);
    let face = g.face_toward(0, Vec3::Z).unwrap();
    assert_eq!(
        FaceOccluders::new(&g, 0, &face, None).covered_fraction(),
        0.0
    );
}

fn exposed(others: Vec<PartBox>) -> (f64, f64) {
    let n = others.len();
    let o = occluders(others, &vec![false; n]);
    (
        o.facing_fraction_among(&|_| true),
        o.exposed_fraction_among(&|_| true),
    )
}

/// 面（1 m × 1 m）と同じ大きさの板が距離 1 m で平行に向き合う（底が面の平面から 1 m）形態係数は、等しい正方形（辺 = 距離）の
/// 文献値（Howell の C-11）0.19982。隙間 1 mm ではほぼ 1、100 m ではほぼ 0（`A / (π r²)` ≈ 3.2e-5）。
/// 接していなければ覆いではないので、覆われた割合は 0。
#[test]
fn a_parallel_plate_in_front_blocks_by_the_view_factor() {
    let plate_at = |gap: f64| part("A", [0.0, 0.0, TOP + gap + 0.005], [1.0, 1.0, 0.01]);
    let (f, exposed_fraction) = exposed(vec![plate_at(1.0)]);
    assert!((f - 0.19982).abs() < 5e-4, "{f}");
    assert!((exposed_fraction - (1.0 - f)).abs() < 1e-15);
    assert_eq!(covered(vec![plate_at(1.0)]), 0.0);
    assert!(exposed(vec![plate_at(0.001)]).0 > 0.99);
    assert!(exposed(vec![plate_at(100.0)]).0 < 1e-4);
}

/// 向き合うのは平行な面だけ（傾いた箱・面に垂直に立つ板は数えない）。面に垂直に立つ板（展開したパドル）は 0。
/// 接して覆う箱は向き合いに数えず（接触の規則だけ）、覆いと向き合う板が重なる範囲は二重に引かない: 面の半分を箱が覆い、
/// その上 1 m に面と同じ大きさの板があれば、開いている割合は `(1 − 0.5)(1 − F)`。
#[test]
fn only_parallel_facing_faces_count_and_contact_is_not_counted_twice() {
    let standing = part("A", [0.0, 0.0, TOP + 0.5], [1.0, 0.01, 1.0]);
    assert_eq!(exposed(vec![standing]).0, 0.0);
    let resting = part("R", [0.25, 0.0, TOP + 0.05], [0.5, 1.0, 0.1]);
    let (f, e) = exposed(vec![resting.clone()]);
    assert_eq!((f, e), (0.0, 0.5));
    let above = part("A", [0.0, 0.0, TOP + 1.005], [1.0, 1.0, 0.01]);
    let (f, e) = exposed(vec![resting, above]);
    assert!((f - 0.19982).abs() < 5e-4);
    assert!(close(e, 0.5 * (1.0 - f)));
}

/// 面に載る部品（面の部品に取り付けられた部品）は向き合いにも数えない。
#[test]
fn a_mounted_part_is_not_a_facing_part() {
    let above = part("A", [0.0, 0.0, TOP + 1.005], [1.0, 1.0, 0.01]);
    let o = occluders(vec![above], &[true]);
    assert_eq!(o.facing_fraction_among(&|_| true), 0.0);
}

/// 面に載る部品の接地面（`Footprint::Credited`）は、載る部品が持ち主を遮らないという意味だけで、第三の部品の影は接地面の下にも落ちる:
/// 面（1 m × 1 m）の x ∈ [0, 0.5] に取り付けられたセルがあり、その真上 1 m の x ∈ [0, 0.5] に板がある。真上の光で、持ち主の日照は 0.5
/// （板の影が接地面の下を暗くする。以前は接地面が日照のまま数えられて 1）。板が接地面の外（x ∈ [−0.5, 0]）なら同じく 0.5。
#[test]
fn a_third_part_still_shades_the_area_under_a_mounted_cell() {
    let cell = part("CELL", [0.25, 0.0, TOP + 0.001], [0.5, 1.0, 0.002]);
    let lit = |plate_x: f64| {
        let cover = part("A", [plate_x, 0.0, TOP + 1.0], [0.5, 1.0, 0.01]);
        let g = ExternalGeometry::new(vec![plate(), cell.clone(), cover]);
        let face = g.face_toward(0, Vec3::Z).unwrap();
        // 箱ごとの印: セルは持ち主に取り付けられている
        FaceOccluders::new(&g, 0, &face, Some(&[false, true, false])).lit_fraction(Vec3::Z)
    };
    assert!(close(lit(0.25), 0.5));
    assert!(close(lit(-0.25), 0.5));
    // 板が無ければ、セルの接地面は日照のまま（持ち主を遮らない）
    let g = ExternalGeometry::new(vec![plate(), cell]);
    let face = g.face_toward(0, Vec3::Z).unwrap();
    assert_eq!(
        FaceOccluders::new(&g, 0, &face, Some(&[false, true])).lit_fraction(Vec3::Z),
        1.0
    );
}

/// `lit_fraction`（接地面と覆いの前処理を使い回し、面に届かない影の凸包を省く）は、`lit_fraction_among`（毎回すべての多角形を組む）と
/// 同じ値（ビットまで）を返す。覆い・面に載る部品・離れた箱・影を落とす箱が全部ある面を、いろいろな向きで比べる。
#[test]
fn the_lit_fraction_is_bit_identical_to_the_unprepared_one() {
    let others = vec![
        part("COVER", [0.25, 0.0, TOP + 0.05], [0.5, 1.0, 0.1]),
        part("MOUNTED", [-0.3, 0.2, TOP + 0.03], [0.2, 0.2, 0.06]),
        part("TALL", [0.4, -0.3, TOP + 0.5], [0.1, 0.1, 0.2]),
        part("FAR", [5.0, 5.0, TOP + 0.2], [0.3, 0.3, 0.3]),
    ];
    let occ = occluders(others, &[false, true, false, false]);
    for k in 0..40 {
        let a = 0.17 * f64::from(k);
        let d = unit(a.cos(), (1.3 * a).sin(), 0.3 + 0.02 * f64::from(k));
        assert_eq!(
            occ.lit_fraction(d).to_bits(),
            occ.lit_fraction_among(d, &|_| true).to_bits(),
            "direction {k}"
        );
    }
}
