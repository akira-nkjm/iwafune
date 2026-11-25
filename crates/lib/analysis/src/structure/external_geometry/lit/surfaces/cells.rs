//! 升に分けた板の面の、升ごとの日照割合と環境に開いている割合（`LitSurfaces` の続き）。升の長方形は `external_geometry::cells`、計算は `FaceOccluders`。

use super::super::super::cells::{FaceCell, face_cells};
use super::{FaceOccluders, LitSurfaces};
use crate::structure::external_geometry::{ExternalGeometry, direction_key};
use sat_astro::math::Vec3;
use std::collections::BTreeMap;
use std::rc::Rc;

/// 面ごとの升（`plates` の部品の板の面だけ。升に分けない面は `None`）。
pub(super) fn split_cells(
    geometry: &ExternalGeometry,
    ids: &[String],
    faces: &[Option<FaceOccluders>],
    plates: &BTreeMap<String, [usize; 2]>,
) -> Vec<Option<Vec<FaceCell>>> {
    ids.iter()
        .zip(faces)
        .map(|(id, face)| {
            let split = plates.get(id)?;
            let face = face.as_ref()?;
            let owner = geometry.parts().iter().find(|p| p.id == *id)?;
            face_cells(&owner.shape, face.face(), *split)
        })
        .collect()
}

/// 面ごと・升ごとの、環境に開いている割合（`keep` が真の箱だけを数える。升に分けない面は空）。
pub(super) fn exposed_per_cell(
    cells: &[Option<Vec<FaceCell>>],
    faces: &[Option<FaceOccluders>],
    keep: &dyn Fn(usize) -> bool,
) -> Vec<Vec<f64>> {
    cells
        .iter()
        .zip(faces)
        .map(|(cells, face)| match (cells, face) {
            (Some(cells), Some(face)) => cells
                .iter()
                .map(|c| face.exposed_fraction_in(keep, Some(c)))
                .collect(),
            _ => Vec::new(),
        })
        .collect()
}

impl LitSurfaces {
    /// 面 `i` を分けた升の数（升に分けない面は `None`）。升の並びは `PlateGrid::index`（節点 `<部品>[i,j]` と同じ）。
    pub fn cell_count(&self, i: usize) -> Option<usize> {
        self.cells.get(i)?.as_ref().map(Vec::len)
    }

    /// 面 `i` の升 `c` が環境に開いている割合（面全体の [`LitSurfaces::exposed`] を升の長方形に当てたもの）。升に分けない面は面全体の値。
    pub fn exposed_cell(&self, i: usize, c: usize) -> f64 {
        self.exposed_cells
            .get(i)
            .and_then(|e| e.get(c))
            .copied()
            .unwrap_or_else(|| self.exposed(i))
    }

    /// 面 `i` の升 `c` が環境に開いている割合を、熱の外面を持つ部品の覆いと向き合いだけで数えたもの。
    pub fn exposed_surfaced_only_cell(&self, i: usize, c: usize) -> f64 {
        self.exposed_surfaced_cells
            .get(i)
            .and_then(|e| e.get(c))
            .copied()
            .unwrap_or_else(|| self.exposed_surfaced_only(i))
    }

    /// 面 `i` の升 `c` の、光源の向き `direction` での日照割合。升に分けない面は面全体の [`LitSurfaces::lit`]。
    /// 向きごとに全部の升をまとめて求めて使い回す。
    pub fn lit_cell(&self, i: usize, c: usize, direction: Vec3) -> f64 {
        self.lit_cell_of(i, c, direction, false)
            .unwrap_or_else(|| self.lit(i, direction))
    }

    /// [`LitSurfaces::lit_cell`] を、熱の外面を持つ部品の影と覆いだけで数えたもの（[`LitSurfaces::lit_surfaced_only`] の升版）。
    pub fn lit_surfaced_only_cell(&self, i: usize, c: usize, direction: Vec3) -> f64 {
        self.lit_cell_of(i, c, direction, true)
            .unwrap_or_else(|| self.lit_surfaced_only(i, direction))
    }

    /// 面 `i` の升 `c` の日照割合（升に分けない面・升の番号が範囲外は `None`）。`surfaced_only` は外面を持つ部品だけを数える。
    fn lit_cell_of(&self, i: usize, c: usize, direction: Vec3, surfaced_only: bool) -> Option<f64> {
        let cells = self.cells.get(i)?.as_ref()?;
        let face = self.faces.get(i)?.as_ref()?;
        let Some(d) = direction.normalized().filter(|d| d.x.is_finite()) else {
            return (c < cells.len()).then_some(0.0);
        };
        let key = (i, direction_key(d));
        if let Some((li, lk, ls, v)) = self.last_cells.borrow().as_ref()
            && (*li, *lk, *ls) == (key.0, key.1, surfaced_only)
        {
            return v.get(c).copied();
        }
        let cache = if surfaced_only {
            &self.cache_cells_surfaced
        } else {
            &self.cache_cells
        };
        let hit = cache.borrow().get(&key).cloned();
        let lit = match hit {
            Some(v) => v,
            None => {
                let v = Rc::new(if surfaced_only {
                    face.lit_fractions_over(
                        d,
                        &|j| self.surfaced.get(j).copied().unwrap_or(false),
                        cells,
                    )
                } else {
                    face.lit_fractions_over(d, &|_| true, cells)
                });
                cache.borrow_mut().insert(key, Rc::clone(&v));
                v
            }
        };
        let value = lit.get(c).copied();
        *self.last_cells.borrow_mut() = Some((key.0, key.1, surfaced_only, lit));
        value
    }
}
