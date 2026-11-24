//! 外形の幾何（箱の集まりの投影面積）。箱は向きつきの箱（`OrientedBox`）で、部品の ID を添える。
//! 表面積（面の被覆）は子の `exposed`（`ExternalGeometry::exposure`）、平均の向きの組は子の `directions`。

use super::super::interference::obb::{OrientedBox, PLANE_EPS, plane_basis};
use super::directions::DirectionSet;
use sat_astro::math::Vec3;
use sat_astro::math::planar::union_measure;

/// 面の判定の許容 [m]（0.1 mm）: 内部の部品の包含（頂点が箱の中か）と、面の被覆（面の平面に接しているか）に使う。
/// CAD の寸法の丸めと取付の面の合わせの余裕で、箱の近似の粗さ（mm〜cm）より十分小さい
/// （`structure.interference` の許容差の既定と同じ尺度。意味が違う値なので、別の定数にしている）。
pub const SURFACE_TOLERANCE: f64 = 1e-4;

/// 部品の箱 1 つ。
#[derive(Debug, Clone, PartialEq)]
pub struct PartBox {
    pub id: String,
    pub shape: OrientedBox,
}

/// ある向きに沿って見た投影（輪郭の和集合）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projection {
    /// 投影面積（箱の輪郭の和集合。重なりは 1 度だけ数える）[m²]
    pub area: f64,
    /// 輪郭の和集合の図心。**機体座標系の点**で、機体の原点を通り向きに垂直な平面の上（向きの成分は 0）。
    /// 重心まわりの腕は、重心を同じ平面へ射影した点との差で、向きに垂直な成分だけが効く。箱が無ければ（面積 0）`None`。
    pub centre: Option<Vec3>,
}

/// 箱の集まりの外形。
#[derive(Debug, Clone)]
pub struct ExternalGeometry {
    parts: Vec<PartBox>,
    internal: Vec<Option<usize>>,
    /// 別の箱に（許容なしで）丸ごと含まれる箱。輪郭が含む側の輪郭の中にあるので、投影の和集合から省ける（結果は変わらない）
    hidden: Vec<Option<usize>>,
}

impl ExternalGeometry {
    /// 箱から作る。**内部の部品**（規則の階層 1）も決める: 8 つの頂点がすべて、ほかの 1 つの箱の中
    /// （[`SURFACE_TOLERANCE`] の余裕つき）にある部品は内部。互いに同じ箱（許容の内で相互に収まる）なら、
    /// 並びの先のほう（ID の昇順に並べた入力で、番号が小さいほう）を残して、後のほうを内部とする。
    pub fn new(parts: Vec<PartBox>) -> Self {
        let internal = contained_flags(&parts, SURFACE_TOLERANCE);
        let hidden = contained_flags(&parts, PLANE_EPS);
        Self {
            parts,
            internal,
            hidden,
        }
    }

    /// 全部の箱。
    pub fn parts(&self) -> &[PartBox] {
        &self.parts
    }

    /// 部品 `index` が内部か。
    pub fn is_internal(&self, index: usize) -> bool {
        self.container_of(index).is_some()
    }

    /// 部品 `index` を含む箱の番号（内部でなければ `None`。含む箱が複数あれば番号が最小のもの）。
    pub fn container_of(&self, index: usize) -> Option<usize> {
        self.internal.get(index).copied().flatten()
    }

    /// 内部と判定した部品の ID。
    pub fn internal_ids(&self) -> Vec<&str> {
        (0..self.parts.len())
            .filter(|i| self.is_internal(*i))
            .map(|i| self.parts[i].id.as_str())
            .collect()
    }

    /// 向き `direction` に沿って見た投影（全部の箱の輪郭の和集合。内部の部品は和集合が隠すので、分類しない）。
    /// `direction` は零でも非有限でもよくない（長さは問わず、向きだけ使う）。不正なら `None`。
    pub fn projected(&self, direction: Vec3) -> Option<Projection> {
        let d = direction.normalized().filter(|d| d.x.is_finite())?;
        let silhouettes: Vec<_> = self
            .parts
            .iter()
            .zip(&self.hidden)
            .filter(|(_, hidden)| hidden.is_none())
            .map(|(p, _)| p.shape.silhouette(d))
            .collect();
        let measure = union_measure(&silhouettes);
        let (e1, e2) = plane_basis(d);
        Some(Projection {
            area: measure.area,
            centre: measure.centroid.map(|c| e1.scale(c[0]) + e2.scale(c[1])),
        })
    }

    /// 向きの組の各向きでの投影面積。
    pub fn projected_areas(&self, set: &DirectionSet) -> Vec<DirectionArea> {
        set.nodes()
            .iter()
            .map(|(direction, weight)| DirectionArea {
                direction: *direction,
                weight: *weight,
                area: self.projected(*direction).map_or(0.0, |p| p.area),
            })
            .collect()
    }

    /// 全方向の平均の投影面積 [m²]（[`DirectionSet::standard`] の求積）。箱が無ければ 0。
    pub fn mean_projected_area(&self) -> f64 {
        self.mean_projected_area_with(&DirectionSet::standard())
    }

    /// 向きの組を指定した平均の投影面積 [m²]（細かい組との比較用）。
    pub fn mean_projected_area_with(&self, set: &DirectionSet) -> f64 {
        mean_of(&self.projected_areas(set))
    }
}

/// 別の 1 つの箱に丸ごと含まれる（8 つの頂点が `tolerance` の余裕つきで中にある）箱と、含む箱の番号（最小のもの）。
/// 互いに同じ箱なら、番号の小さいほうを残す（含まれる側にしない）。
fn contained_flags(parts: &[PartBox], tolerance: f64) -> Vec<Option<usize>> {
    let contains = |i: usize, j: usize| parts[i].shape.contains_box(&parts[j].shape, tolerance);
    (0..parts.len())
        .map(|i| (0..parts.len()).find(|&j| j != i && contains(j, i) && (!contains(i, j) || j < i)))
        .collect()
}

/// 向き 1 つの投影面積。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionArea {
    /// 向き（単位ベクトル）
    pub direction: Vec3,
    /// 求積の重み（組の重みの和は 1）
    pub weight: f64,
    /// 投影面積 [m²]
    pub area: f64,
}

/// 向きの組の重みつきの平均面積 [m²]（全球の平均の投影面積の求積）。
pub fn mean_of(areas: &[DirectionArea]) -> f64 {
    areas.iter().map(|a| a.weight * a.area).sum()
}
