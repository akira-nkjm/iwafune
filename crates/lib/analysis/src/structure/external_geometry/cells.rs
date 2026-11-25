//! 板の升に分けた面の、升の長方形（面の座標）。**板の面を升の長方形に分ける幾何はここだけ**
//! （升に分けた板の面の日照・覆い・向き合い（`lit`）と、空洞の形態係数（`cavity_views`）が、同じ升の長方形を使う。
//! 格子そのもの（法線の軸・面内の軸・升の寸法と並び）は `plate_grid::PlateGrid`）。
//!
//! 板の面（箱の最も薄い軸に垂直な面）だけが升に分かれる。板の縁の面（法線が面内の軸）は升に分けない（`None`）。
//! 升の並びは `PlateGrid::index`（升の通し番号）の順で、節点 `<部品>[i,j]` の並びと同じ。

use super::super::interference::obb::{BoxFace, OrientedBox};
use crate::plate_grid::PlateGrid;
use sat_astro::math::planar::ConvexPolygon;
use sat_astro::view_factor::Rect;

/// 面の座標（箱の面の中心からの `[u, v]`）での、升 1 つの長方形。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceCell {
    /// 升の中心の面の座標 `[u, v]`
    pub centre: [f64; 2],
    /// `u`・`v` 方向の半幅 [m]
    pub half: [f64; 2],
}

impl FaceCell {
    /// 面の座標での長方形。
    pub fn polygon(&self) -> ConvexPolygon {
        ConvexPolygon::rectangle(
            [self.centre[0] - self.half[0], self.centre[1] - self.half[1]],
            [self.centre[0] + self.half[0], self.centre[1] + self.half[1]],
        )
    }

    /// 面積 [m²]。
    pub fn area(&self) -> f64 {
        4.0 * self.half[0] * self.half[1]
    }

    /// 形態係数の計算（`sat_astro::view_factor`）に渡す長方形（面 `face` の上の升）。
    pub fn rect(&self, face: &BoxFace) -> Rect {
        Rect {
            center: face.center + face.u.scale(self.centre[0]) + face.v.scale(self.centre[1]),
            normal: face.normal,
            u: face.u,
            v: face.v,
            half_u: self.half[0],
            half_v: self.half[1],
        }
    }
}

/// 箱 `shape`（板）の面 `face` を、`cells`（u 方向・v 方向の升の数）の格子の升に分けた長方形（升の通し番号の順）。
/// 面が板の面（法線が箱の最も薄い軸）でなければ、辺の長さが格子に使えなければ `None`。
/// 升の位置は、格子の升の中心（箱の座標系）を機体座標系に写して面の座標に直すので、面の `u`・`v` の向き・反転（空洞の壁）に依らない。
pub fn face_cells(shape: &OrientedBox, face: &BoxFace, cells: [usize; 2]) -> Option<Vec<FaceCell>> {
    let half = shape.half();
    let grid = PlateGrid::new([2.0 * half[0], 2.0 * half[1], 2.0 * half[2]], cells)?;
    if face.axis != grid.normal {
        return None;
    }
    let axes = shape.axes();
    let [du, dv] = grid.cell_size();
    // 面の u 軸が、格子のどちらの面内の軸に沿うか
    let u_along_grid_u = face.u.dot(axes[grid.axes[0]]).abs() > 0.5;
    let half_cell = if u_along_grid_u {
        [0.5 * du, 0.5 * dv]
    } else {
        [0.5 * dv, 0.5 * du]
    };
    Some(
        (0..grid.count())
            .map(|c| {
                let (i, j) = grid.position(c);
                let o = grid.centre_offset(i, j);
                let centre = (0..3).fold(shape.center, |p, k| p + axes[k].scale(o[k]));
                FaceCell {
                    centre: face.local(centre),
                    half: half_cell,
                }
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sat_astro::math::{Quat, Vec3};

    /// 手計算: 0.3 × 0.2 × 0.003 m の板（法線 z、u = x、v = y）を 3 × 2 に分けると升は 0.1 × 0.1 m。
    /// +z の面（u = x、v = y）の升 (0,0) は中心 (−0.1, −0.05)。−z の面は u = y・v = x（`u × v` が外向き）で、中心は (−0.05, −0.1)。
    /// 升の面積の和は面の面積に等しい。縁の面は分けない。
    #[test]
    fn cells_by_hand_on_both_faces() {
        let shape = OrientedBox::from_size(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::IDENTITY,
            Vec3::new(0.3, 0.2, 0.003),
        );
        let faces = shape.faces();
        let top = &faces[5];
        let cells = face_cells(&shape, top, [3, 2]).unwrap();
        assert_eq!(cells.len(), 6);
        assert!(
            (cells[0].centre[0] + 0.1).abs() < 1e-15 && (cells[0].centre[1] + 0.05).abs() < 1e-15
        );
        assert!((cells.iter().map(FaceCell::area).sum::<f64>() - top.area()).abs() < 1e-15);
        let bottom = &faces[4];
        let cells = face_cells(&shape, bottom, [3, 2]).unwrap();
        assert!(
            (cells[0].centre[0] + 0.05).abs() < 1e-15 && (cells[0].centre[1] + 0.1).abs() < 1e-15
        );
        assert!((cells[0].half[0] - 0.05).abs() < 1e-15 && (cells[0].half[1] - 0.05).abs() < 1e-15);
        assert!(face_cells(&shape, &faces[0], [3, 2]).is_none());
    }
}
