//! 板の升への配り方の重み: 相手の箱が各升に載る割合（取付の伝導を配る）。箱と升の接触面積、箱どうしが接していなければ
//! 相手の箱を板の面に射影した形と升の重なりの面積。

use super::Group;
use crate::structure::interference::obb::OrientedBox;
use crate::structure::interference::{DEFAULT_TOLERANCE, is_usable};
use sat_astro::math::Vec3;
use sat_astro::math::planar::ConvexPolygon;

/// 最大の接触面積に対するこの割合以下の升は、箱の辺をかするだけ（丸めの誤差の面積）として接触に数えない。
const EDGE_CONTACT_FRACTION: f64 = 1e-9;

impl Group {
    /// 相手の節点 `other` の箱が各升に載る割合。まず箱と升の接触面積（`OrientedBox::contact_area` を升の箱に当てる）、
    /// 箱どうしが許容差の外で接していなければ、相手の箱を板の面に射影した形と升の重なりの面積。どちらも決められなければ `None`。
    pub(super) fn located(&self, other: &str) -> Option<Vec<f64>> {
        let frame = &self.plate.frame;
        if !is_usable(frame) {
            return None;
        }
        let others: Vec<OrientedBox> = self
            .plate
            .attachments
            .iter()
            .filter(|a| a.node == other && is_usable(&a.footprint))
            .map(|a| {
                let f = &a.footprint;
                OrientedBox::from_size(f.position, f.orientation, f.size)
            })
            .collect();
        let touching = self.cell_areas(|cell, _| {
            let tol = DEFAULT_TOLERANCE.value();
            others
                .iter()
                .filter_map(|b| cell.contact_area(b, tol))
                .sum()
        });
        normalised(touching).or_else(|| {
            normalised(
                self.cell_areas(|_, rect| {
                    others.iter().map(|b| self.overlap_in_plane(b, &rect)).sum()
                }),
            )
        })
    }

    /// 升ごとの量（升の箱と、板の座標系の面内の長方形 [u 下限, v 下限, u 上限, v 上限] から求める）。
    fn cell_areas(&self, f: impl Fn(&OrientedBox, [f64; 4]) -> f64) -> Vec<f64> {
        let frame = &self.plate.frame;
        let size = self.grid.cell_box_size();
        let size = Vec3::new(size[0], size[1], size[2]);
        let [u, v] = self.grid.axes;
        let [du, dv] = self.grid.cell_size();
        (0..self.grid.count())
            .map(|c| {
                let (i, j) = self.grid.position(c);
                let o = self.grid.centre_offset(i, j);
                let centre = frame.position + frame.orientation.rotate(Vec3::new(o[0], o[1], o[2]));
                let cell = OrientedBox::from_size(centre, frame.orientation, size);
                f(
                    &cell,
                    [
                        o[u] - 0.5 * du,
                        o[v] - 0.5 * dv,
                        o[u] + 0.5 * du,
                        o[v] + 0.5 * dv,
                    ],
                )
            })
            .collect()
    }

    /// 箱 `b` を板の面（板の座標系の u, v）に射影した凸多角形と、長方形 `rect` の共通部分の面積。
    fn overlap_in_plane(&self, b: &OrientedBox, rect: &[f64; 4]) -> f64 {
        let frame = &self.plate.frame;
        let [u, v] = self.grid.axes;
        let points: Vec<[f64; 2]> = b
            .corners()
            .iter()
            .map(|c| {
                let p = frame.orientation.rotate_inverse(*c - frame.position);
                let p = [p.x, p.y, p.z];
                [p[u], p[v]]
            })
            .collect();
        ConvexPolygon::hull(&points)
            .intersection(&ConvexPolygon::rectangle(
                [rect[0], rect[1]],
                [rect[2], rect[3]],
            ))
            .area()
    }
}

/// 升ごとの面積を、辺や角でかするだけの升（丸めで出る極小の面積）を 0 にして割合にする。合計が 0 なら `None`。
fn normalised(areas: Vec<f64>) -> Option<Vec<f64>> {
    let largest = areas.iter().copied().fold(0.0, f64::max);
    let kept: Vec<f64> = areas
        .into_iter()
        .map(|a| {
            if a > EDGE_CONTACT_FRACTION * largest {
                a
            } else {
                0.0
            }
        })
        .collect();
    let total: f64 = kept.iter().sum();
    (total > 0.0).then(|| kept.iter().map(|a| a / total).collect())
}
