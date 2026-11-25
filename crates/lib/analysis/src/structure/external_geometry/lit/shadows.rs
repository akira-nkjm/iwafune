//! 向きごとの影の多角形（`FaceOccluders` の続き）。影は向きで変わるが、接地面と覆いは向きによらないので、
//! どの影も面に面積を持たない向きでは日照割合が定数になる（`lit_fraction` はそれを 1 度だけ求める）。

use super::dark::{DarkShapes, FixedDark};
use super::{FaceOccluders, GRAZING_COS};
use sat_astro::math::Vec3;
use sat_astro::math::planar::ConvexPolygon;

/// 向き 1 つの影の多角形。`with_footprint` は面に載る部品（接地面あり）の影と接地面の組、`plain` はそのほかの箱の影。
pub(super) struct Shadows<'a> {
    with_footprint: Vec<(ConvexPolygon, &'a ConvexPolygon)>,
    plain: Vec<ConvexPolygon>,
}

impl FaceOccluders {
    /// 光源の向き `direction` で面が光を受けるときの、影のずれ `(d·u, d·v) / (n·d)`。受けなければ（向きが零か非有限、裏面）`None`。
    pub(super) fn light_shift(&self, direction: Vec3) -> Option<(f64, f64)> {
        let d = direction.normalized().filter(|d| d.x.is_finite())?;
        let cos = self.normal.dot(d);
        if cos <= GRAZING_COS {
            return None;
        }
        Some((d.dot(self.u) / cos, d.dot(self.v) / cos))
    }

    /// `keep(箱の番号)` が真の箱が、ずれ `shift` で作る影。
    pub(super) fn shadows(&self, shift: (f64, f64), keep: &dyn Fn(usize) -> bool) -> Shadows<'_> {
        let mut shadows = Shadows {
            with_footprint: Vec::new(),
            plain: Vec::new(),
        };
        for o in self.occluders.iter().filter(|o| keep(o.part)) {
            let shadow = self.shadow(o, shift);
            match &o.footprint {
                Some(fp) => shadows.with_footprint.push((shadow, fp)),
                None => shadows.plain.push(shadow),
            }
        }
        shadows
    }

    /// 暗い範囲の多角形: 面に載る部品自身の影 M・その接地面 F・そのほかの影と覆いの範囲 B（B は影、覆いの順）。
    /// 暗い範囲は、面に載る部品自身の影から接地面を除いたもの M \ F と B の和集合
    /// （接地面は「載る部品が自分の持ち主を遮らない」ことだけを意味し、ほかの部品の影は接地面の下にも落ちる）。
    pub(super) fn dark_from(
        &self,
        shadows: Shadows<'_>,
        keep: &dyn Fn(usize) -> bool,
    ) -> DarkShapes {
        let Shadows {
            with_footprint,
            plain,
        } = shadows;
        let (m, f): (Vec<_>, Vec<_>) = with_footprint
            .into_iter()
            .map(|(shadow, fp)| (shadow, fp.clone()))
            .unzip();
        let mut b = plain;
        b.extend(
            self.covers
                .iter()
                .filter(|c| keep(c.part))
                .map(|c| c.region.clone()),
        );
        DarkShapes::new(m, f, b)
    }

    /// すべての箱を数えた日照割合（`lit_fraction`）。接地面と覆い（向きによらない）の前処理を使い回す（[`DarkShapes::lit`] と同じ値）。
    pub(super) fn lit_fraction_all(&self, direction: Vec3) -> f64 {
        let Some(shift) = self.light_shift(direction) else {
            return 0.0;
        };
        let keep = |_: usize| true;
        let shadows = self.shadows(shift, &keep);
        let fixed = self.fixed_dark.get_or_init(|| {
            let f: Vec<ConvexPolygon> = self
                .occluders
                .iter()
                .filter_map(|o| o.footprint.clone())
                .collect();
            let b: Vec<ConvexPolygon> = self.covers.iter().map(|c| c.region.clone()).collect();
            FixedDark::new(&f, &b, self.area)
        });
        let Shadows {
            with_footprint,
            plain,
        } = shadows;
        let m: Vec<ConvexPolygon> = with_footprint
            .into_iter()
            .map(|(shadow, _)| shadow)
            .filter(has_area)
            .collect();
        let plain: Vec<ConvexPolygon> = plain.into_iter().filter(has_area).collect();
        if m.is_empty() && plain.is_empty() {
            fixed.unshadowed()
        } else {
            fixed.lit(&m, &plain)
        }
    }
}

/// 面積を持つ多角形か（面積 0 の多角形は和集合の面積 `union_measure` が数えない）。
fn has_area(p: &ConvexPolygon) -> bool {
    p.area().partial_cmp(&0.0) == Some(std::cmp::Ordering::Greater)
}
