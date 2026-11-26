//! 凸多角形の和集合のテスト。手計算（包除原理・台形・図形の分割）と比べる。
//! 厳密な幾何なので、許容は面積に対して相対 1e-12（丸め誤差の積み重ねの上限。固有の近似は無い）。

use super::*;

const REL: f64 = 1e-12;

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> ConvexPolygon {
    ConvexPolygon::rectangle([x0, y0], [x1, y1])
}

fn poly(points: &[Point2]) -> ConvexPolygon {
    ConvexPolygon::hull(points)
}

fn assert_close(got: f64, want: f64, what: &str) {
    assert!(
        (got - want).abs() <= REL * want.abs().max(1.0),
        "{what}: got {got}, want {want}"
    );
}

fn assert_measure(polys: &[ConvexPolygon], area: f64, centroid: Point2) {
    let m = union_measure(polys);
    assert_close(m.area, area, "area");
    let c = m.centroid.expect("a non-empty union has a centroid");
    assert_close(c[0], centroid[0], "centroid x");
    assert_close(c[1], centroid[1], "centroid y");
}

/// 長方形 1 つ: 2 × 3、図心は中心。
#[test]
fn one_rectangle() {
    assert_measure(&[rect(1.0, 2.0, 3.0, 5.0)], 6.0, [2.0, 3.5]);
}

/// 離れた 2 つ: 面積は和、図心は面積で重みを付けた平均。1×1（中心 0.5, 0.5）と 2×1（中心 4, 0.5）→ x = (0.5 + 8) / 3。
#[test]
fn two_disjoint_rectangles() {
    assert_measure(
        &[rect(0.0, 0.0, 1.0, 1.0), rect(3.0, 0.0, 5.0, 1.0)],
        3.0,
        [8.5 / 3.0, 0.5],
    );
}

/// 重なる 2 つ: [0,2]² と [1,3]×[1,3]。重なりは [1,2]²（面積 1）。面積 4 + 4 − 1 = 7。
/// モーメント: x は 4·1 + 4·2 − 1·1.5 = 10.5（y も同じ）→ 図心 1.5。
#[test]
fn two_overlapping_rectangles() {
    assert_measure(
        &[rect(0.0, 0.0, 2.0, 2.0), rect(1.0, 1.0, 3.0, 3.0)],
        7.0,
        [10.5 / 7.0, 10.5 / 7.0],
    );
}

/// 片方が他方の中: 和集合は大きいほうだけ。
#[test]
fn one_inside_another() {
    assert_measure(
        &[rect(0.0, 0.0, 4.0, 2.0), rect(1.0, 0.5, 2.0, 1.5)],
        8.0,
        [2.0, 1.0],
    );
}

/// 辺を共有する 2 つ（x = 1 で接する）: 重なりは無く、面積は和。
#[test]
fn two_sharing_an_edge() {
    assert_measure(
        &[rect(0.0, 0.0, 1.0, 1.0), rect(1.0, 0.0, 2.0, 1.0)],
        2.0,
        [1.0, 0.5],
    );
}

/// 同じ 2 つ（座標も同じ）: 1 つぶん。
#[test]
fn two_identical() {
    assert_measure(
        &[rect(0.0, 0.0, 3.0, 2.0), rect(0.0, 0.0, 3.0, 2.0)],
        6.0,
        [1.5, 1.0],
    );
}

/// L 字（[0,2]×[0,1] と [0,1]×[1,2]）: 面積 3。モーメント x: 2·1 + 1·0.5 = 2.5、y: 2·0.5 + 1·1.5 = 2.5。
#[test]
fn an_l_shape_from_two_rectangles() {
    assert_measure(
        &[rect(0.0, 0.0, 2.0, 1.0), rect(0.0, 1.0, 1.0, 2.0)],
        3.0,
        [2.5 / 3.0, 2.5 / 3.0],
    );
}

/// 正方形 [0,2]² の上に三角形 (1,1)-(3,1)-(1,2)（面積 1、斜辺 y = 1 + (3 − x)/2）。
/// 重なり = {1 ≤ x ≤ 2, 1 ≤ y ≤ 1 + (3 − x)/2}: 面積 ∫₁² (3 − x)/2 dx = 0.75、
/// モーメント x = ∫ x (3 − x)/2 dx = 13/12、y = 25/24（手積分）。
/// 和集合: 面積 4 + 1 − 0.75、x のモーメント 4 + 5/3 − 13/12、y のモーメント 4 + 4/3 − 25/24。
#[test]
fn a_triangle_over_a_square() {
    let square = rect(0.0, 0.0, 2.0, 2.0);
    let triangle = poly(&[[1.0, 1.0], [3.0, 1.0], [1.0, 2.0]]);
    assert_close(triangle.area(), 1.0, "triangle area");
    let area = 4.25;
    let mx = 4.0 + 5.0 / 3.0 - 13.0 / 12.0;
    let my = 4.0 + 4.0 / 3.0 - 25.0 / 24.0;
    assert_measure(&[square, triangle], area, [mx / area, my / area]);
}

/// 面積が 0 の多角形（線分・点）は何も足さない。縦に長い線分が長方形の外に突き出していても同じ。
#[test]
fn a_zero_area_polygon_contributes_nothing() {
    let base = [rect(0.0, 0.0, 2.0, 1.0)];
    let with = [
        rect(0.0, 0.0, 2.0, 1.0),
        poly(&[[1.0, -5.0], [1.0, 9.0]]),
        poly(&[[7.0, 7.0]]),
    ];
    let (a, b) = (union_measure(&base), union_measure(&with));
    assert_eq!(a.area, b.area);
    assert_eq!(a.centroid, b.centroid);
    assert_eq!(union_measure(&[poly(&[[0.0, 0.0], [1.0, 1.0]])]).area, 0.0);
    assert_eq!(union_measure(&[]).centroid, None);
}

/// 3 つが互いに重なる: [0,2]²、[1,3]×[0,2]、[0,2]×[1,3]。包除原理:
/// 面積 4 + 4 + 4 − (2 + 2 + 1) + 1 = 8。モーメント x: (4·1 + 4·2 + 4·1) − (2·1.5 + 2·1 + 1·1.5) + 1·1.5 = 11（y は対称で 11）。
#[test]
fn three_mutually_overlapping_by_inclusion_exclusion() {
    assert_measure(
        &[
            rect(0.0, 0.0, 2.0, 2.0),
            rect(1.0, 0.0, 3.0, 2.0),
            rect(0.0, 1.0, 2.0, 3.0),
        ],
        8.0,
        [11.0 / 8.0, 11.0 / 8.0],
    );
}

/// 菱形の中の長方形: 原点が中心で半対角が 2 と 1 の菱形 (2,0),(0,1),(-2,0),(0,-1)（面積 4）と
/// 長方形 [-1,1]×[-0.25,0.25]（面積 1）。長方形が菱形の中に入るのは |x|/2 + |y| ≤ 1 のとき、角 (1, 0.25) は 0.5 + 0.25 ≤ 1 で内側。
/// よって長方形は菱形の中で、和集合は菱形だけ（面積 4、図心は原点）。
#[test]
fn a_rectangle_inside_a_rhombus() {
    let rhombus = poly(&[[2.0, 0.0], [0.0, 1.0], [-2.0, 0.0], [0.0, -1.0]]);
    assert_close(rhombus.area(), 4.0, "rhombus area");
    let m = union_measure(&[rhombus, rect(-1.0, -0.25, 1.0, 0.25)]);
    assert_close(m.area, 4.0, "area");
    let c = m.centroid.unwrap();
    assert!(c[0].abs() < 1e-12 && c[1].abs() < 1e-12);
}

/// 斜辺が同じ直線の 2 つの三角形（頂点が互いの辺の上にある）: T1 = (0,0),(4,0),(0,4)（x + y ≤ 4）と T2 = (0,4),(4,4),(4,0)（x + y ≥ 4）。
/// 斜辺が同じ直線で、和集合は正方形 [0,4]²（面積 16、図心 (2,2)）。重なりは線だけ。
#[test]
fn two_triangles_sharing_a_diagonal_make_a_square() {
    let t1 = poly(&[[0.0, 0.0], [4.0, 0.0], [0.0, 4.0]]);
    let t2 = poly(&[[0.0, 4.0], [4.0, 4.0], [4.0, 0.0]]);
    assert_measure(&[t1, t2], 16.0, [2.0, 2.0]);
}

/// 辺が斜めに交わる: 正方形 [0,2]² と、それを 45° 回して中心を (2,1) に置いた正方形（対角線 2。頂点 (2,0),(3,1),(2,2),(1,1)）。
/// 重なりは、回した正方形の x ≤ 2 の半分 = 三角形 (2,0),(2,2),(1,1)（面積 1）。和集合は 4 + 2 − 1 = 5。
/// 回した正方形の面積 2。モーメント x: 4·1 + 2·2 − 1·(5/3) = 8 − 5/3、y: 4·1 + 2·1 − 1·1 = 5。
#[test]
fn diagonal_edges_crossing_give_the_hand_area() {
    let square = rect(0.0, 0.0, 2.0, 2.0);
    let diamond = poly(&[[2.0, 0.0], [3.0, 1.0], [2.0, 2.0], [1.0, 1.0]]);
    assert_close(diamond.area(), 2.0, "diamond area");
    let mx = 8.0 - 5.0 / 3.0;
    assert_measure(&[square, diamond], 5.0, [mx / 5.0, 1.0]);
}

/// 辺や頂点が他の多角形の辺の上にある（接する）形でも落ちない: 同じ辺の上に頂点がある 3 つの長方形（中段の幅が上下より広い T 字の逆）。
/// [0,3]×[0,1]、[1,2]×[1,2]（下の長方形の上辺に載る）、[1,2]×[2,3]（その上に載る）。面積 3 + 1 + 1。
#[test]
fn stacked_rectangles_touching_along_edges() {
    assert_measure(
        &[
            rect(0.0, 0.0, 3.0, 1.0),
            rect(1.0, 1.0, 2.0, 2.0),
            rect(1.0, 2.0, 2.0, 3.0),
        ],
        5.0,
        [1.5, (1.5 + 1.5 + 2.5) / 5.0],
    );
}

/// ほぼ平行な辺（傾きが 1e-9 違う 2 本の辺）でも、面積は手計算（台形）とほぼ一致する。
/// 長方形 [0,1]×[0,1] と、右辺が x = 1 − 1e-9·y から始まる台形。差は 1e-9 の程度で、和集合は長方形の面積 + はみ出し（無い）。
#[test]
fn nearly_parallel_edges_are_stable() {
    let eps = 1e-9;
    let a = rect(0.0, 0.0, 1.0, 1.0);
    let b = poly(&[[0.0, 0.0], [1.0 - eps, 0.0], [1.0 + eps, 1.0], [0.0, 1.0]]);
    // b の面積は 1 + 0 = 1（台形 (1 − eps + 1 + eps)/2 · 1）。和集合 = a ∪ b。はみ出しは y > 0.5 側の三角形 (1,0.5)-(1+eps,1)-(1,1) = eps/4。
    let m = union_measure(&[a, b]);
    assert!((m.area - (1.0 + eps / 4.0)).abs() < 1e-12, "{}", m.area);
}

/// 入力の順に依らない（丸め誤差の違いだけ）。
#[test]
fn order_independent() {
    let polys = [
        rect(0.0, 0.0, 2.0, 2.0),
        poly(&[[1.0, 1.0], [3.0, 1.0], [1.0, 2.0]]),
        poly(&[[2.0, 0.0], [3.0, 1.0], [2.0, 2.0], [1.0, 1.0]]),
        rect(0.5, 1.5, 2.5, 2.5),
    ];
    let base = union_measure(&polys);
    let mut order = [3, 1, 0, 2];
    for _ in 0..4 {
        let shuffled: Vec<ConvexPolygon> = order.iter().map(|&i| polys[i].clone()).collect();
        let m = union_measure(&shuffled);
        assert_close(m.area, base.area, "area");
        let (c, b) = (m.centroid.unwrap(), base.centroid.unwrap());
        assert_close(c[0], b[0], "cx");
        assert_close(c[1], b[1], "cy");
        order.rotate_left(1);
    }
}

/// 全体を回転しても面積は変わらない（図心は同じ回転で動く）。30° と 17° で確かめる。
#[test]
fn rotation_invariance() {
    let polys = [
        rect(0.0, 0.0, 2.0, 2.0),
        poly(&[[1.0, 1.0], [3.0, 1.0], [1.0, 2.0]]),
        poly(&[[2.0, 0.0], [3.0, 1.0], [2.0, 2.0], [1.0, 1.0]]),
    ];
    let base = union_measure(&polys);
    for deg in [30.0_f64, 17.0, 90.0] {
        let (s, c) = deg.to_radians().sin_cos();
        let turn = |p: &Point2| [c * p[0] - s * p[1], s * p[0] + c * p[1]];
        let turned: Vec<ConvexPolygon> = polys
            .iter()
            .map(|p| poly(&p.vertices().iter().map(turn).collect::<Vec<_>>()))
            .collect();
        let m = union_measure(&turned);
        assert_close(m.area, base.area, "area");
        let want = turn(&base.centroid.unwrap());
        let got = m.centroid.unwrap();
        assert_close(got[0], want[0], "cx");
        assert_close(got[1], want[1], "cy");
    }
}

/// 凸包: 内側の点と一直線上の点は落ち、反時計回りになる。
#[test]
fn hull_drops_interior_and_collinear_points() {
    let h = poly(&[
        [0.0, 0.0],
        [1.0, 0.0],
        [2.0, 0.0],
        [2.0, 2.0],
        [1.0, 1.0],
        [0.0, 2.0],
    ]);
    assert_eq!(h.vertices().len(), 4);
    assert_close(h.area(), 4.0, "area");
}

/// 共通部分（凸どうし）: [0,2]² と [1,3]² → [1,2]²。離れていれば空。
#[test]
fn intersection_of_convex_polygons() {
    let i = rect(0.0, 0.0, 2.0, 2.0).intersection(&rect(1.0, 1.0, 3.0, 3.0));
    assert_close(i.area(), 1.0, "overlap");
    let none = rect(0.0, 0.0, 1.0, 1.0).intersection(&rect(2.0, 2.0, 3.0, 3.0));
    assert!(none.vertices().is_empty());
}

/// 乱数（固定の種）の軸平行な長方形 6 つの和集合を、座標圧縮した格子（軸平行なら厳密）の面積・モーメントと比べる。
/// 格子の座標は 0.25 の倍数にして、辺や頂点がぴったり重なる縮退も含める。
#[test]
fn random_rectangles_match_the_compressed_grid() {
    let mut state = 0x2545_F491_4F6C_DD1D_u64;
    let mut next = |n: u64| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) % n
    };
    for _ in 0..200 {
        let rects: Vec<[f64; 4]> = (0..6)
            .map(|_| {
                let (x, y) = (next(12) as f64 * 0.25, next(12) as f64 * 0.25);
                [
                    x,
                    y,
                    x + (1 + next(8)) as f64 * 0.25,
                    y + (1 + next(8)) as f64 * 0.25,
                ]
            })
            .collect();
        // 0.25 刻みの全セルについて、どれかの長方形に入っていれば数える
        let (mut area, mut mx, mut my) = (0.0, 0.0, 0.0);
        for i in 0..24 {
            for j in 0..24 {
                let (cx, cy) = (i as f64 * 0.25 + 0.125, j as f64 * 0.25 + 0.125);
                if rects
                    .iter()
                    .any(|r| r[0] < cx && cx < r[2] && r[1] < cy && cy < r[3])
                {
                    area += 0.0625;
                    mx += 0.0625 * cx;
                    my += 0.0625 * cy;
                }
            }
        }
        let polys: Vec<ConvexPolygon> =
            rects.iter().map(|r| rect(r[0], r[1], r[2], r[3])).collect();
        assert_measure(&polys, area, [mx / area, my / area]);
    }
}

/// `StaticUnion` は、固定の多角形を前処理しても `union_measure`（並び `[先頭, 前の固定, 間, 後ろの固定]`）と同じ値（ビットまで）を返す。
#[test]
fn static_union_matches_union_measure_bit_for_bit() {
    let head = vec![rect(0.1, 0.1, 0.4, 0.3), rect(0.5, 0.2, 0.9, 0.6)];
    let tail = vec![
        rect(0.0, 0.5, 0.35, 0.95),
        poly(&[[0.6, 0.0], [1.0, 0.3], [0.7, 0.9]]),
    ];
    let fixed = StaticUnion::new(&head, &tail);
    let shadows = [
        poly(&[[0.05, 0.0], [0.45, 0.1], [0.3, 0.55]]),
        rect(0.3, 0.3, 0.8, 0.45),
        rect(2.0, 2.0, 3.0, 3.0), // 離れていて重ならない
        ConvexPolygon::hull(&[]),
    ];
    let betweens = [vec![], vec![poly(&[[0.2, 0.0], [0.9, 0.5], [0.4, 0.8]])]];
    for lead in [&shadows[..0], &shadows[..1], &shadows[..], &shadows[1..3]] {
        for between in &betweens {
            let all: Vec<ConvexPolygon> = lead
                .iter()
                .chain(&head)
                .chain(between)
                .chain(&tail)
                .cloned()
                .collect();
            let (got, want) = (fixed.measure(lead, between), union_measure(&all));
            assert_eq!(got.area.to_bits(), want.area.to_bits());
            assert_eq!(got.centroid, want.centroid);
        }
    }
}
