//! 面の一部（升の長方形 [`FaceCell`]）についての日照割合・覆われた割合・向き合う面への形態係数・環境に開いている割合（`FaceOccluders` の続き）。
//! 面全体の関数（`*_among`）は同じ式を升なし（`None`）で呼ぶ。

use super::dark::clipped;
use super::{FaceCell, FaceOccluders, LIT_SNAP};
use sat_astro::math::Vec3;
use sat_astro::math::planar::{ConvexPolygon, union_measure};
use sat_astro::view_factor::{Quadrature, rect_view_factor};

impl FaceOccluders {
    /// [`FaceOccluders::covered_fraction_among`] を、面の一部（升 `region`。`None` なら面全体）について求める
    /// （覆いの範囲を升の長方形で切り、升の面積で割る）。
    pub fn covered_fraction_in(
        &self,
        keep: &dyn Fn(usize) -> bool,
        region: Option<&FaceCell>,
    ) -> f64 {
        let clip = region.map(FaceCell::polygon);
        let regions: Vec<ConvexPolygon> = self
            .covers
            .iter()
            .filter(|c| keep(c.part))
            .map(|c| clipped(&c.region, clip.as_ref()))
            .collect();
        let covered = union_measure(&regions).area / region.map_or(self.area, FaceCell::area);
        if covered <= LIT_SNAP {
            0.0
        } else if covered >= 1.0 - LIT_SNAP {
            1.0
        } else {
            covered
        }
    }

    /// [`FaceOccluders::facing_fraction_among`] を、面の一部（升 `region`。`None` なら面全体）から見る形態係数で求める
    /// （向き合う箱の選び方は面ごと。升から向き合う面を見る形態係数は面全体と同じ `rect_view_factor`。
    /// 升の面積で重みづけした和は、面全体の値に求積の誤差の範囲で一致する。1 で打ち切るのは升ごと）。
    pub fn facing_fraction_in(
        &self,
        keep: &dyn Fn(usize) -> bool,
        region: Option<&FaceCell>,
    ) -> f64 {
        self.facing
            .iter()
            .filter(|f| keep(f.part))
            .map(|f| match region {
                None => f.factor,
                Some(cell) => {
                    rect_view_factor(&cell.rect(&self.face), &f.target, None, Quadrature::DEFAULT)
                }
            })
            .sum::<f64>()
            .min(1.0)
    }

    /// [`FaceOccluders::exposed_fraction_among`] を、面の一部（升 `region`。`None` なら面全体）について求める。
    pub fn exposed_fraction_in(
        &self,
        keep: &dyn Fn(usize) -> bool,
        region: Option<&FaceCell>,
    ) -> f64 {
        (1.0 - self.covered_fraction_in(keep, region))
            * (1.0 - self.facing_fraction_in(keep, region))
    }

    /// 升（`cells`。面の座標の長方形）ごとの日照割合（升の通し番号の順）。影・接地面・覆いの多角形は 1 度だけ作り、升の長方形で切って、
    /// 升の面積で割る（サンプル点は使わず、面全体と同じ多角形の和集合の面積）。面全体の日照割合は、升の面積で重みづけした和に等しい
    /// （和集合の面積は升の和に分けられる。丸めの誤差の範囲）。
    pub fn lit_fractions_over(
        &self,
        direction: Vec3,
        keep: &dyn Fn(usize) -> bool,
        cells: &[FaceCell],
    ) -> Vec<f64> {
        let Some(shapes) = self.dark_shapes(direction, keep) else {
            return vec![0.0; cells.len()];
        };
        cells
            .iter()
            .map(|c| shapes.lit(Some(c), c.area()))
            .collect()
    }
}
