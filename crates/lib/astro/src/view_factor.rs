//! 長方形どうしの形態係数（Lambert 面、遮蔽なし）。**長方形間の形態係数の式はここだけ。**
//!
//! 式は Howell, A Catalog of Radiation Heat Transfer Configuration Factors の C-11（同じ大きさで真向かいの平行な長方形）と
//! C-14（共通の辺を持つ直交する長方形）。Incropera にも同じ式がある。
//! 任意の位置・向きの長方形どうし（遮蔽つき）は子の `rect`（数値積分。閉じた式との一致は試験で確かめる）。
//! 中心天体に対する平板の形態係数は [`crate::illumination::body_view_factor`]。

use std::f64::consts::PI;

mod rect;
pub use rect::{Quadrature, Rect, point_to_rect, rect_view_factor};

/// 箱の 6 面の並び（外向き法線）: +X, −X, +Y, −Y, +Z, −Z。[`box_faces`] の添字はこの順。
pub const BOX_FACES: [&str; 6] = ["+X", "-X", "+Y", "-Y", "+Z", "-Z"];

/// 同じ大きさ `a × b` で、距離 `c` だけ離れて真向かいに平行な 2 長方形の形態係数（Howell C-11）。寸法は同じ単位。
/// 寸法が正でなければ 0。
pub fn parallel_opposed(a: f64, b: f64, c: f64) -> f64 {
    if !(a > 0.0 && b > 0.0 && c > 0.0) {
        return 0.0;
    }
    let (x, y) = (a / c, b / c);
    let (rx, ry) = ((1.0 + x * x).sqrt(), (1.0 + y * y).sqrt());
    let log = 0.5 * ((1.0 + x * x) * (1.0 + y * y) / (1.0 + x * x + y * y)).ln();
    let terms =
        log + x * ry * (x / ry).atan() + y * rx * (y / rx).atan() - x * x.atan() - y * y.atan();
    2.0 / (PI * x * y) * terms
}

/// 共通の辺（長さ `edge`）を持つ直交する 2 長方形のうち、辺に垂直な幅が `width_from` の長方形から、
/// 幅が `width_to` の長方形を見る形態係数（Howell C-14）。寸法は同じ単位。寸法が正でなければ 0。
pub fn perpendicular_shared_edge(edge: f64, width_from: f64, width_to: f64) -> f64 {
    if !(edge > 0.0 && width_from > 0.0 && width_to > 0.0) {
        return 0.0;
    }
    let (w, h) = (width_from / edge, width_to / edge);
    let (w2, h2) = (w * w, h * h);
    let r = (w2 + h2).sqrt();
    let log = 0.25 * ((1.0 + w2) * (1.0 + h2) / (1.0 + w2 + h2)).ln()
        + 0.25 * w2 * (w2 * (1.0 + w2 + h2) / ((1.0 + w2) * (w2 + h2))).ln()
        + 0.25 * h2 * (h2 * (1.0 + w2 + h2) / ((1.0 + h2) * (w2 + h2))).ln();
    (w * (1.0 / w).atan() + h * (1.0 / h).atan() - r * (1.0 / r).atan() + log) / (PI * w)
}

/// 直方体（X・Y・Z 方向の長さ `size`）の内側の 6 面どうしの形態係数。添字は [`BOX_FACES`] の順で、
/// `f[i][j]` は面 i から面 j を見る係数（自分自身は 0。行の和は 1）。寸法が正でなければ全て 0。
pub fn box_faces(size: [f64; 3]) -> [[f64; 6]; 6] {
    let mut f = [[0.0; 6]; 6];
    if !size.iter().all(|&d| d > 0.0) {
        return f;
    }
    for (i, row) in f.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            let (ai, aj) = (i / 2, j / 2);
            *v = if i == j {
                0.0
            } else if ai == aj {
                // 向かい合う面: 面の寸法は残り 2 軸、距離は法線の軸の長さ
                let [p, q] = other_axes(ai);
                parallel_opposed(size[p], size[q], size[ai])
            } else {
                // 隣り合う面: 共通の辺は残りの軸。i の面の辺に垂直な幅は aj 軸の長さ、j の面は ai 軸の長さ
                let k = 3 - ai - aj;
                perpendicular_shared_edge(size[k], size[aj], size[ai])
            };
        }
    }
    f
}

fn other_axes(axis: usize) -> [usize; 2] {
    match axis {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 立方体の向かい合う面 0.19982、隣り合う面 0.20004（Howell の表・Incropera の図表の値）。
    #[test]
    fn cube_matches_literature() {
        assert!((parallel_opposed(1.0, 1.0, 1.0) - 0.19982).abs() < 5e-6);
        assert!((perpendicular_shared_edge(1.0, 1.0, 1.0) - 0.20004).abs() < 5e-6);
        let f = box_faces([1.0, 1.0, 1.0]);
        assert!((f[0][1] - 0.19982).abs() < 5e-6);
        assert!((f[0][2] - 0.20004).abs() < 5e-6);
    }

    /// 閉じた箱では Σ_j F_ij = 1（辺の長さが違っても）、相反則 A_i F_ij = A_j F_ji。
    #[test]
    fn box_closes_and_is_reciprocal() {
        let size = [0.1, 0.226, 0.366];
        let f = box_faces(size);
        let area = |i: usize| {
            let [p, q] = other_axes(i / 2);
            size[p] * size[q]
        };
        for i in 0..6 {
            assert!((f[i].iter().sum::<f64>() - 1.0).abs() < 1e-9, "row {i}");
            for j in 0..6 {
                assert!(
                    (area(i) * f[i][j] - area(j) * f[j][i]).abs() < 1e-12,
                    "{i} {j}"
                );
            }
        }
    }

    /// 無限に近い平行平板（距離が辺に比べて十分小さい）では F → 1。
    #[test]
    fn flat_plates_see_each_other() {
        assert!((parallel_opposed(10.0, 10.0, 0.01) - 1.0).abs() < 2e-3);
    }

    #[test]
    fn degenerate_sizes_give_zero() {
        assert_eq!(parallel_opposed(0.0, 1.0, 1.0), 0.0);
        assert_eq!(box_faces([1.0, -1.0, 1.0]), [[0.0; 6]; 6]);
    }
}
