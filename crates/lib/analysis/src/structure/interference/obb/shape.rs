//! 向きつきの箱の形（頂点・面・投影の輪郭・断面・包含・外接）。距離と重なり（分離軸の判定）は親の `obb`。
//! 箱を機体座標系の凸多面体として扱う関数はここだけ（外形の面積 `structure.external_geometry` が使う）。
//! 平面の凸多角形は `sat_astro::math::planar`。

use super::OrientedBox;
use sat_astro::math::Vec3;
use sat_astro::math::planar::ConvexPolygon;

/// 平面との交わりを頂点・辺の上と見る距離 [m]。座標（m 単位の 1 前後）の丸め誤差 ~1e-16 より十分大きい。
pub const PLANE_EPS: f64 = 1e-12;

/// 箱の面 1 つ（長方形）。`u × v = normal`（外から見て反時計回り）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxFace {
    /// 面の中心（機体座標系）
    pub center: Vec3,
    /// 外向きの単位法線
    pub normal: Vec3,
    /// 面の内の 2 本の単位ベクトル（`u × v = normal`）
    pub u: Vec3,
    pub v: Vec3,
    /// `u`・`v` 方向の半幅 [m]
    pub half_u: f64,
    pub half_v: f64,
    /// 箱の軸の番号（0・1・2 = x・y・z）と、その軸の正の側の面か
    pub axis: usize,
    pub positive: bool,
}

impl BoxFace {
    /// 面の面積 [m²]。
    pub fn area(&self) -> f64 {
        4.0 * self.half_u * self.half_v
    }

    /// 機体座標系の点の、面の中の座標 `[u, v]`（中心から）。
    pub fn local(&self, p: Vec3) -> [f64; 2] {
        let d = p - self.center;
        [d.dot(self.u), d.dot(self.v)]
    }

    /// 面そのものの長方形（面の座標）。
    pub fn rectangle(&self) -> ConvexPolygon {
        ConvexPolygon::rectangle([-self.half_u, -self.half_v], [self.half_u, self.half_v])
    }
}

/// 向き `direction`（単位ベクトル）に垂直な平面の座標軸 `(e1, e2)`。`(e1, e2, direction)` は右手系で、`direction` から決まる。
/// 向きの符号を替えると `e1` だけが反転する（同じ平面・同じ点を指す）。
pub fn plane_basis(direction: Vec3) -> (Vec3, Vec3) {
    let e1 = direction.any_orthogonal();
    (e1, direction.cross(e1))
}

/// 箱の集まりの、機体座標系の軸に平行な外接の範囲 `(最小, 最大)`。箱が無ければ `None`。
pub fn axis_aligned_extents(boxes: &[OrientedBox]) -> Option<(Vec3, Vec3)> {
    let mut corners = boxes.iter().flat_map(OrientedBox::corners);
    let first = corners.next()?;
    Some(corners.fold((first, first), |(lo, hi), c| {
        (
            Vec3::new(lo.x.min(c.x), lo.y.min(c.y), lo.z.min(c.z)),
            Vec3::new(hi.x.max(c.x), hi.y.max(c.y), hi.z.max(c.z)),
        )
    }))
}

impl OrientedBox {
    /// 8 つの頂点（機体座標系）。番号の 3 ビットが x・y・z の側（1 で正）。
    pub fn corners(&self) -> [Vec3; 8] {
        let (a, h) = (self.axes(), self.half());
        std::array::from_fn(|i| {
            (0..3).fold(self.center, |p, k| {
                let sign = if (i >> k) & 1 == 1 { 1.0 } else { -1.0 };
                p + a[k].scale(sign * h[k])
            })
        })
    }

    /// 点が箱の中（または表面から `tolerance` 以内）にあるか。
    pub fn contains_point(&self, p: Vec3, tolerance: f64) -> bool {
        let d = p - self.center;
        self.axes()
            .iter()
            .zip(self.half())
            .all(|(a, h)| d.dot(*a).abs() <= h + tolerance)
    }

    /// 別の箱が丸ごとこの箱の中にあるか（8 つの頂点がすべて `tolerance` の余裕つきで中。箱は凸なので頂点で足りる）。
    pub fn contains_box(&self, other: &OrientedBox, tolerance: f64) -> bool {
        other
            .corners()
            .iter()
            .all(|c| self.contains_point(*c, tolerance))
    }

    /// 6 つの面（軸 x・y・z の順に、負の側・正の側）。
    pub fn faces(&self) -> [BoxFace; 6] {
        let (a, h) = (self.axes(), self.half());
        std::array::from_fn(|i| {
            let (k, positive) = (i / 2, i % 2 == 1);
            let (next, after) = (a[(k + 1) % 3], a[(k + 2) % 3]);
            let (hn, ha) = (h[(k + 1) % 3], h[(k + 2) % 3]);
            let sign = if positive { 1.0 } else { -1.0 };
            // 負の側は u と v を入れ替えて、`u × v` が外向きの法線になるようにする
            let (u, v, half_u, half_v) = if positive {
                (next, after, hn, ha)
            } else {
                (after, next, ha, hn)
            };
            BoxFace {
                center: self.center + a[k].scale(sign * h[k]),
                normal: a[k].scale(sign),
                u,
                v,
                half_u,
                half_v,
                axis: k,
                positive,
            }
        })
    }

    /// 向き `direction`（単位ベクトル）に沿って見た箱の輪郭（凸多角形）。座標は [`plane_basis`] の `(e1, e2)` での成分。
    /// 8 つの頂点の凸包なので頂点は最大 6 つ。面に垂直なら長方形、辺に沿って真横から見れば線分（面積 0）。
    pub fn silhouette(&self, direction: Vec3) -> ConvexPolygon {
        let (e1, e2) = plane_basis(direction);
        let points: Vec<[f64; 2]> = self
            .corners()
            .iter()
            .map(|c| [c.dot(e1), c.dot(e2)])
            .collect();
        ConvexPolygon::hull(&points)
    }

    /// 平面 `normal · x = offset`（`normal` は単位ベクトル）との交わりの点（機体座標系）。平面が箱から `tolerance` より離れていれば `None`。
    /// 平面が箱を横切るなら断面の頂点、箱の端に `tolerance` 以内で接するだけなら、平面を箱の端まで寄せた接触の頂点
    /// （面に平行なら面の 4 頂点、辺なら 2 頂点、頂点なら 1 点）。凸包を取れば断面・接触面（凸多角形）になる。
    pub fn plane_section(&self, normal: Vec3, offset: f64, tolerance: f64) -> Option<Vec<Vec3>> {
        let (a, h) = (self.axes(), self.half());
        let reach: f64 = a.iter().zip(h).map(|(a, h)| h * normal.dot(*a).abs()).sum();
        let centre = normal.dot(self.center);
        let d = offset - centre;
        if d.abs() > reach + tolerance {
            return None;
        }
        let plane = centre + d.clamp(-reach, reach);
        let corners = self.corners();
        let height: Vec<f64> = corners.iter().map(|c| normal.dot(*c) - plane).collect();
        let mut points: Vec<Vec3> = (0..8)
            .filter(|&i| height[i].abs() <= PLANE_EPS)
            .map(|i| corners[i])
            .collect();
        for i in 0..8 {
            for k in 0..3 {
                let j = i | (1 << k);
                if j == i {
                    continue;
                }
                let (fi, fj) = (height[i], height[j]);
                if fi.abs() > PLANE_EPS && fj.abs() > PLANE_EPS && (fi < 0.0) != (fj < 0.0) {
                    let t = fi / (fi - fj);
                    points.push(corners[i] + (corners[j] - corners[i]).scale(t));
                }
            }
        }
        Some(points)
    }
}

#[cfg(test)]
mod tests;
