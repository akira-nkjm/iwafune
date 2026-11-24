//! 外に出ている表面積（箱の面のうち、ほかの箱に覆われていない部分）。
//!
//! 面 1 つ（長方形）について、ほかの箱を面の平面で切り（`OrientedBox::plane_section`。平面を横切る箱は断面、
//! 面の平面に [`SURFACE_TOLERANCE`] 以内で接する箱は接触面を、面の中の凸多角形にする）、面の長方形で切り取って、
//! 和集合（`sat_astro::math::planar::union_measure`）の面積を面の面積から引く。内部の部品（[`ExternalGeometry::is_internal`]）は
//! 面も数えず、覆う側にもしない（内部の部品が覆う範囲は、それを含む箱がすでに覆っている）。
//!
//! 向きの内訳は、面の外向き法線の **成分が最大の軸**（±X・±Y・±Z）で分ける。軸に平行な箱なら厳密で、回した箱では
//! 法線を最も近い軸に寄せる分け方（面積は 1 つの向きにだけ入り、合計は変わらない）。成分が並ぶ（45° など）法線は、
//! 軸の番号が小さいほう（x、y、z の順）に入れる。

use super::super::interference::obb::BoxFace;
use super::geometry::{ExternalGeometry, SURFACE_TOLERANCE};
use sat_astro::math::planar::{ConvexPolygon, union_measure};

/// 面の残りがこの割合（面の面積に対して）以下なら、覆われきったとみなす（引き算の丸め誤差の除去）。
const COVERED_RELATIVE: f64 = 1e-12;
/// 面の向き（機体座標系の軸）の分け方で、成分が並ぶとみなす差。丸め誤差（~1e-16）より十分大きい。
const AXIS_TIE: f64 = 1e-9;

/// 外に出ている表面積。
#[derive(Debug, Clone, PartialEq)]
pub struct Exposure {
    /// 合計 [m²]
    pub total: f64,
    /// 向き別（添字は `2 × 軸 + (正の側なら 1)`。つまり −X, +X, −Y, +Y, −Z, +Z の順）[m²]
    pub by_direction: [f64; 6],
    /// 内部でない部品ごと（`ExternalGeometry::parts` の添字と面積）。覆われきった部品の 0 も含む
    pub by_part: Vec<(usize, f64)>,
}

impl ExternalGeometry {
    /// 外に出ている表面積（合計・向き別・部品別）。内部の部品は含めない。
    pub fn exposure(&self) -> Exposure {
        let mut out = Exposure {
            total: 0.0,
            by_direction: [0.0; 6],
            by_part: Vec::new(),
        };
        for (i, part) in self.parts().iter().enumerate() {
            if self.is_internal(i) {
                continue;
            }
            let mut part_area = 0.0;
            for face in part.shape.faces() {
                let area = self.exposed_face_area(i, &face);
                part_area += area;
                out.by_direction[direction_index(&face)] += area;
            }
            out.total += part_area;
            out.by_part.push((i, part_area));
        }
        out
    }

    /// 部品 `owner` の面 1 つの、覆われていない面積 [m²]。
    fn exposed_face_area(&self, owner: usize, face: &BoxFace) -> f64 {
        let rectangle = face.rectangle();
        let offset = face.normal.dot(face.center);
        let covers: Vec<ConvexPolygon> = self
            .parts()
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != owner && !self.is_internal(*j))
            .filter_map(|(_, other)| {
                let points = other
                    .shape
                    .plane_section(face.normal, offset, SURFACE_TOLERANCE)?;
                let local: Vec<[f64; 2]> = points.iter().map(|p| face.local(*p)).collect();
                Some(ConvexPolygon::hull(&local).intersection(&rectangle))
            })
            .collect();
        let exposed = face.area() - union_measure(&covers).area;
        // 全部が覆われた面は、引き算の丸め誤差（面積の 1e-16 倍ほど）が残るので、面積の 1e-12 以下は 0 とする
        if exposed <= COVERED_RELATIVE * face.area() {
            0.0
        } else {
            exposed
        }
    }
}

/// 面の法線の向きの添字（成分が最大の軸。並べば軸の番号が小さいほう）。
fn direction_index(face: &BoxFace) -> usize {
    let n = [face.normal.x, face.normal.y, face.normal.z];
    let largest = n.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
    let axis = (0..3)
        .find(|&k| n[k].abs() >= largest - AXIS_TIE)
        .unwrap_or(0);
    2 * axis + usize::from(n[axis] > 0.0)
}
