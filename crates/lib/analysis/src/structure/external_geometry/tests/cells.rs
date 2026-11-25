//! 升に分けた板の面のテスト（#456）。面は板（1 m × 1 m、厚さ 10 mm）の上面（z = 0.005、法線 +Z）で、3 × 2 の升に分ける
//! （u = x、v = y。升の x の境は −1/6・1/6、y の境は 0）。箱の影・覆いは x に沿って変わるので、升は i（x の番号）で決まる。
//! 影と覆いは多角形の厳密な面積なので、升の値は手計算に相対 1e-12 で合う。向き合う面の形態係数だけは求積（誤差は各テストに）。

use super::lit::FaceOccluders;
use super::*;
use crate::structure::external_geometry::cells::{FaceCell, face_cells};

const TOP: f64 = 0.005;

fn plate() -> PartBox {
    part("PLATE", [0.0, 0.0, 0.0], [1.0, 1.0, 0.01])
}

/// 板の上面の遮る箱・覆い・向き合いと、3 × 2 の升。
fn setup(others: Vec<PartBox>) -> (FaceOccluders, Vec<FaceCell>) {
    let mut parts = vec![plate()];
    parts.extend(others);
    let g = ExternalGeometry::new(parts);
    let face = g.face_toward(0, Vec3::Z).unwrap();
    let flags = vec![false; g.parts().len()];
    let o = FaceOccluders::new(&g, 0, &face, Some(&flags)).with_covers(&g, 0, &face, &flags);
    let cells = face_cells(&g.parts()[0].shape, &face, [3, 2]).unwrap();
    (o, cells)
}

/// x ∈ [0, 0.5]、全幅の箱が面に載る（覆う）。覆いの割合は、x ∈ [−1/2, −1/6] の升 0 で 0、[−1/6, 1/6] の升 1 で（x ∈ [0, 1/6] の半分）0.5、
/// [1/6, 1/2] の升 2 で 1（同じ i の 2 つの升は同じ）。面全体は 0.5 で、升の平均（升は同じ面積）に等しい。
/// 日照割合（覆われた範囲は暗い）も 1・0.5・0。覆われていない割合 `exposed` は覆いの割合の残り。
#[test]
fn a_cover_over_half_the_plate_covers_those_cells_not_half_of_each() {
    let resting = part("A", [0.25, 0.0, TOP + 0.05], [0.5, 1.0, 0.1]);
    let (o, cells) = setup(vec![resting]);
    assert_eq!(cells.len(), 6);
    let by_i = [0.0, 0.5, 1.0];
    let covered: Vec<f64> = cells
        .iter()
        .map(|c| o.covered_fraction_in(&|_| true, Some(c)))
        .collect();
    let lit = o.lit_fractions_over(Vec3::Z, &|_| true, &cells);
    for (k, c) in covered.iter().enumerate() {
        assert!(close(*c, by_i[k / 2]), "cell {k}: {c}");
        assert!(close(lit[k], 1.0 - by_i[k / 2]), "cell {k}: {}", lit[k]);
        assert!(close(
            o.exposed_fraction_in(&|_| true, Some(&cells[k])),
            1.0 - by_i[k / 2]
        ));
    }
    // 保存: 升の平均は面全体の値（覆いも日照も）
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    assert!(close(mean(&covered), o.covered_fraction()));
    assert!(close(mean(&lit), o.lit_fraction(Vec3::Z)));
}

/// 手計算: 傾けた太陽 d = (1, 0, 2)/√5（tanθ = 0.5）で、箱 "A"（x ∈ [0, 0.5]、板の面からの高さ 0.1〜0.2 m）の影は、
/// 高さ h の点が x − h·tanθ に落ちるので x ∈ [−0.1, 0.45]。面全体の日照は 1 − 0.55 = 0.45。
/// 升 0（x ∈ [−0.5, −1/6]）は全部日照 1、升 1（[−1/6, 1/6]）は [−1/6, −0.1] だけ日照 (1/6 − 0.1)/(1/3) = 0.2、
/// 升 2（[1/6, 1/2]）は [0.45, 0.5] だけ日照 0.05/(1/3) = 0.15。平均は (1 + 0.2 + 0.15)/3 = 0.45 で面全体に一致（保存）。
#[test]
fn an_oblique_shadow_edge_inside_a_cell_gives_the_partial_lit_fraction() {
    let shading = part("A", [0.25, 0.0, TOP + 0.15], [0.5, 1.0, 0.1]);
    let (o, cells) = setup(vec![shading]);
    let d = Vec3::new(1.0, 0.0, 2.0);
    let lit = o.lit_fractions_over(d, &|_| true, &cells);
    let by_i = [1.0, 0.2, 0.15];
    for (k, l) in lit.iter().enumerate() {
        assert!(close(*l, by_i[k / 2]), "cell {k}: {l}");
    }
    assert!(close(o.lit_fraction(d), 0.45));
    assert!(close(lit.iter().sum::<f64>() / 6.0, o.lit_fraction(d)));
    // 影の外の光（真上）では箱の影は x ∈ [0, 0.5] に落ちる: 升 0 は 1、升 1 は 0.5、升 2 は 0
    let lit = o.lit_fractions_over(Vec3::Z, &|_| true, &cells);
    assert!(close(lit[0], 1.0) && close(lit[2], 0.5) && close(lit[4], 0.0));
    // 裏から・平行な光は 0
    assert!(
        o.lit_fractions_over(-Vec3::Z, &|_| true, &cells)
            .iter()
            .all(|l| *l == 0.0)
    );
}

/// 面（1 m × 1 m）と同じ大きさの板が距離 1 m で平行に向き合う。形態係数は文献値（Howell の C-11、等しい正方形で辺 = 距離）0.19982。
/// 升から見る形態係数も同じ `rect_view_factor` で、升の面積で重みづけした和は面全体の値に一致する。求積（面内 10 点の Gauss–Legendre）の誤差は
/// 文献値に対して 5e-4 以内（面全体も升の和も）で、2 つの差も同じ範囲。
/// 向き合う板が面の半分（x ∈ [0, 0.5]）だけにあるときは、升ごとの形態係数が違い（向き合う板の下の升 2 が最大）、面全体の値は升の平均。
#[test]
fn facing_factors_per_cell_sum_to_the_face_value() {
    let above = |x: f64, w: f64| part("B", [x, 0.0, TOP + 1.005], [w, 1.0, 0.01]);
    let (o, cells) = setup(vec![above(0.0, 1.0)]);
    let face = o.facing_fraction_among(&|_| true);
    let sum = cells
        .iter()
        .map(|c| o.facing_fraction_in(&|_| true, Some(c)))
        .sum::<f64>()
        / 6.0;
    assert!(
        (face - 0.19982).abs() < 5e-4 && (sum - 0.19982).abs() < 5e-4,
        "{face} {sum}"
    );
    assert!((face - sum).abs() < 5e-4, "{face} {sum}");
    let (o, cells) = setup(vec![above(0.25, 0.5)]);
    let f: Vec<f64> = cells
        .iter()
        .map(|c| o.facing_fraction_in(&|_| true, Some(c)))
        .collect();
    assert!(f[4] > f[2] && f[2] > f[0], "{f:?}");
    assert!((f[0] - f[1]).abs() < 1e-12 && (f[4] - f[5]).abs() < 1e-12);
    let face = o.facing_fraction_among(&|_| true);
    assert!((f.iter().sum::<f64>() / 6.0 - face).abs() < 5e-4, "{face}");
}
