use super::*;
use crate::view_factor::{parallel_opposed, perpendicular_shared_edge};

/// 中心・法線・半幅から長方形を作る（`u × v = normal`。`u` は `any_orthogonal` が選ぶ向きで、
/// 辺の向きが要る配置は `rect_along`）。
fn rect(center: [f64; 3], normal: Vec3, half_u: f64, half_v: f64) -> Rect {
    let n = normal.normalized().unwrap();
    rect_along(center, n, n.any_orthogonal(), half_u, half_v)
}

/// `u` の向きを指定して長方形を作る（`u` は法線に直交する単位ベクトル）。
fn rect_along(center: [f64; 3], n: Vec3, u: Vec3, half_u: f64, half_v: f64) -> Rect {
    Rect {
        center: Vec3::from_array(center),
        normal: n,
        u,
        v: n.cross(u),
        half_u,
        half_v,
    }
}

fn clear(a: &Rect, b: &Rect) -> f64 {
    rect_view_factor(a, b, None, Quadrature::DEFAULT)
}

/// 向き合う 2 枚の正方形（辺 1、距離 1）: 閉じた式 Howell C-11（`parallel_opposed`）に一致する。
#[test]
fn parallel_unit_squares_match_the_closed_form() {
    let a = rect([0.0, 0.0, 0.0], Vec3::Z, 0.5, 0.5);
    let b = rect([0.0, 0.0, 1.0], -Vec3::Z, 0.5, 0.5);
    let exact = parallel_opposed(1.0, 1.0, 1.0);
    assert!((clear(&a, &b) - exact).abs() < 1e-12, "{}", clear(&a, &b));
}

/// 隙間が面の寸法よりずっと小さい（90 mm × 96 mm の面が 2.5 mm 離れる。基板が積み重なる配置）でも閉じた式に一致する。
#[test]
fn nearly_touching_plates_match_the_closed_form() {
    let a = rect([0.0, 0.0, 0.0], Vec3::Z, 0.045, 0.048);
    let b = rect([0.0, 0.0, 0.0025], -Vec3::Z, 0.045, 0.048);
    let exact = parallel_opposed(0.09, 0.096, 0.0025);
    // 隙間 / 寸法 = 1/36 で相対 7e-4（折れ目が辺の近くの幅 ~ 隙間に集まるため、遠い配置より誤差が大きい）
    assert!(
        (clear(&a, &b) / exact - 1.0).abs() < 1e-3,
        "{} {exact}",
        clear(&a, &b)
    );
}

/// 共通の辺を持つ直交する 2 枚の正方形（辺 1）: Howell C-14（`perpendicular_shared_edge`）。0.2000…
#[test]
fn perpendicular_unit_squares_match_the_closed_form() {
    let a = rect([0.5, 0.5, 0.0], Vec3::Z, 0.5, 0.5);
    let b = rect([0.0, 0.5, 0.5], Vec3::X, 0.5, 0.5);
    let exact = perpendicular_shared_edge(1.0, 1.0, 1.0);
    assert!((clear(&a, &b) - exact).abs() < 1e-10, "{}", clear(&a, &b));
}

/// 大きさの違う直交する長方形（共通の辺 2、A の幅 3、B の幅 1.5。誤差は 1e-5 ほど）。両方向とも閉じた式に一致し、相反則 `A_A F_AB = A_B F_BA` も成り立つ。
#[test]
fn perpendicular_rectangles_of_different_size_match_and_are_reciprocal() {
    let a = rect_along([1.5, 1.0, 0.0], Vec3::Z, Vec3::X, 1.5, 1.0);
    let b = rect_along([0.0, 1.0, 0.75], Vec3::X, Vec3::Y, 1.0, 0.75);
    let (fab, fba) = (clear(&a, &b), clear(&b, &a));
    assert!((fab - perpendicular_shared_edge(2.0, 3.0, 1.5)).abs() < 1e-4);
    assert!((fba - perpendicular_shared_edge(2.0, 1.5, 3.0)).abs() < 1e-4);
    assert!((a.area() * fab - b.area() * fba).abs() < 1e-4 * a.area());
}

/// 閉じた式のない配置（大きさも位置もずれた平行な 2 枚）は、定義の 4 重積分（ガウス・ルジャンドル 14⁴ 点）と比べる。相反則も成り立つ。
#[test]
fn offset_parallel_rectangles_match_the_definition() {
    let a = rect_along([0.0, 0.0, 0.0], Vec3::Z, Vec3::X, 0.5, 0.3);
    let b = rect_along([0.4, -0.2, 0.8], -Vec3::Z, Vec3::X, 0.35, 0.6);
    let reference = brute_force(&a, &b, 14);
    let (fab, fba) = (clear(&a, &b), clear(&b, &a));
    assert!((fab - reference).abs() < 1e-5, "{fab} {reference}");
    assert!((a.area() * fab - b.area() * fba).abs() < 1e-5 * a.area());
}

/// 定義どおりの 4 重積分（各面 `n × n` 点）。核 `cosθ_A cosθ_B / (π r²)`。
fn brute_force(a: &Rect, b: &Rect, n: usize) -> f64 {
    let nodes = gauss_legendre(n);
    let mut sum = 0.0;
    for &(s1, w1) in &nodes {
        for &(t1, w2) in &nodes {
            let p = a.point(s1, t1);
            for &(s2, w3) in &nodes {
                for &(t2, w4) in &nodes {
                    let d = b.point(s2, t2) - p;
                    let r2 = d.norm_squared();
                    let (ca, cb) = (a.normal.dot(d), -b.normal.dot(d));
                    if ca > 0.0 && cb > 0.0 {
                        sum += w1 * w2 * w3 * w4 * ca * cb / (PI * r2 * r2);
                    }
                }
            }
        }
    }
    // 2 つの面の面積要素 (A_A/4)(A_B/4) を掛けて A_A で割る
    sum * b.area() / 16.0
}

/// 箱の内側の 6 面（法線は内向き）。`size` は X・Y・Z の辺。
fn inner_faces(size: [f64; 3]) -> Vec<Rect> {
    let mut faces = Vec::new();
    for axis in 0..3 {
        for sign in [-1.0, 1.0] {
            let mut centre = [0.0; 3];
            centre[axis] = sign * size[axis] / 2.0;
            let mut inward = [0.0; 3];
            inward[axis] = -sign;
            let (p, q) = match axis {
                0 => (1, 2),
                1 => (0, 2),
                _ => (0, 1),
            };
            let mut along = [0.0; 3];
            along[p] = 1.0;
            faces.push(rect_along(
                centre,
                Vec3::from_array(inward),
                Vec3::from_array(along),
                size[p] / 2.0,
                size[q] / 2.0,
            ));
        }
    }
    faces
}

/// 閉じた箱の内側の 6 面: 各面から見た形態係数の和は 1（遮蔽なしの厳密な値と求積の誤差の範囲で）。
/// 立方体と細長い箱で、行の和が 1 から外れる量を確かめる。
#[test]
fn inner_faces_of_a_closed_box_sum_to_one() {
    for size in [[1.0, 1.0, 1.0], [0.1, 0.226, 0.366]] {
        let faces = inner_faces(size);
        for (i, a) in faces.iter().enumerate() {
            let sum: f64 = faces
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, b)| clear(a, b))
                .sum();
            assert!((sum - 1.0).abs() < 1e-10, "{size:?} face {i}: {sum}");
        }
    }
}

/// 立方体の箱の内側の向かい合う面・隣り合う面は、閉じた式（`box_faces`）と同じ。
#[test]
fn box_inner_faces_agree_with_box_faces() {
    let size = [0.1, 0.226, 0.366];
    let faces = inner_faces(size);
    let closed = crate::view_factor::box_faces(size);
    for i in 0..6 {
        for j in 0..6 {
            if i != j {
                assert!(
                    (clear(&faces[i], &faces[j]) - closed[i][j]).abs() < 1e-4,
                    "{i} {j}"
                );
            }
        }
    }
}

/// 2 面の間を全部ふさぐ遮蔽物（どの線分も遮る）なら 0。何も遮らない可視判定なら遮蔽なしと同じ値。
#[test]
fn full_blocker_gives_zero_and_no_blocker_gives_the_clear_value() {
    let a = rect([0.0, 0.0, 0.0], Vec3::Z, 0.5, 0.5);
    let b = rect([0.0, 0.0, 1.0], -Vec3::Z, 0.5, 0.5);
    let blocked: &dyn Fn(Vec3, Vec3) -> bool = &|_, _| false;
    let open: &dyn Fn(Vec3, Vec3) -> bool = &|_, _| true;
    assert_eq!(
        rect_view_factor(&a, &b, Some(blocked), Quadrature::DEFAULT),
        0.0
    );
    let f = rect_view_factor(&a, &b, Some(open), Quadrature::DEFAULT);
    assert!((f - clear(&a, &b)).abs() < 1e-12);
}

/// 中間の高さの平面の半分（x > 0）が線分を遮る。線分が平面と交わる点は両端の中点なので、`x_p + x_q > 0` の線分が遮られる。
/// 核は `(x_p, x_q) → (−x_p, −x_q)` で対称なので、遮られる核の積分は全体のちょうど半分 ⇒ `F = F₀ / 2`（手計算）。
#[test]
fn half_blocking_plate_halves_the_view_factor() {
    let a = rect([0.0, 0.0, 0.0], Vec3::Z, 0.5, 0.5);
    let b = rect([0.0, 0.0, 1.0], -Vec3::Z, 0.5, 0.5);
    // 面の u 方向を機体の x 軸にそろえる（`any_orthogonal` の向きに依らない）
    let along = a.u;
    let visible: &dyn Fn(Vec3, Vec3) -> bool = &|p, q| (p + q).dot(along) <= 0.0;
    let half = rect_view_factor(&a, &b, Some(visible), Quadrature::DEFAULT);
    let expected = 0.5 * parallel_opposed(1.0, 1.0, 1.0);
    // 対称な節点と重みで標本化するので、`p` と `−p` の見える割合の和がちょうど 1 になり、境目の標本化の誤差が打ち消し合う
    assert!((half / expected - 1.0).abs() < 1e-9, "{half} {expected}");
}

/// 対称でない遮蔽（x の中点が 0.2 を超える線分を遮る）は、定義の 4 重積分（ガウス・ルジャンドル 24⁴ 点。段差のある被積分関数なので
/// それ自体に 1 % ほどの誤差がある）に数 % 以内で一致する。遮蔽ありの値の精度は「数 %」（見える割合の標本化）。
#[test]
fn off_centre_blocker_matches_the_definition_within_a_few_percent() {
    let a = rect([0.0, 0.0, 0.0], Vec3::Z, 0.5, 0.5);
    let b = rect([0.0, 0.0, 1.0], -Vec3::Z, 0.5, 0.5);
    let along = a.u;
    let visible = |p: Vec3, q: Vec3| (p + q).dot(along) * 0.5 <= 0.2;
    let nodes = gauss_legendre(24);
    let mut sum = 0.0;
    for &(s1, w1) in &nodes {
        for &(t1, w2) in &nodes {
            let p = a.point(s1, t1);
            for &(s2, w3) in &nodes {
                for &(t2, w4) in &nodes {
                    let q = b.point(s2, t2);
                    let d = q - p;
                    let r2 = d.norm_squared();
                    let (ca, cb) = (a.normal.dot(d), -b.normal.dot(d));
                    if ca > 0.0 && cb > 0.0 && visible(p, q) {
                        sum += w1 * w2 * w3 * w4 * ca * cb / (PI * r2 * r2);
                    }
                }
            }
        }
    }
    let reference = sum * b.area() / 16.0;
    let f = rect_view_factor(&a, &b, Some(&visible), Quadrature::DEFAULT);
    assert!((f / reference - 1.0).abs() < 0.03, "{f} {reference}");
}

/// 向き合っていない面（背中合わせ・同じ向き）は 0。
#[test]
fn faces_that_do_not_face_each_other_give_zero() {
    let a = rect([0.0, 0.0, 0.0], Vec3::Z, 0.5, 0.5);
    let back = rect([0.0, 0.0, 1.0], Vec3::Z, 0.5, 0.5);
    assert_eq!(clear(&a, &back), 0.0);
    let behind = rect([0.0, 0.0, -1.0], -Vec3::Z, 0.5, 0.5);
    assert_eq!(clear(&a, &behind), 0.0);
}
