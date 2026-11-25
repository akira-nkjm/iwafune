//! 外面を添字に直す（`Compiled::new` の一部）: 外面を節点の面（板なら升ごとの面）にし、影・覆いの評価器を作り、
//! 覆われた面積だけ宇宙への放射の係数を減らす。

use super::{Compiled, Surface, ThermalNetwork, ThermalSurface, invalid};
use crate::spec::AnalysisError;
use crate::structure::external_geometry::{Footprint, LitSurfaces};
use sat_astro::constants::SIGMA_SB;
use std::collections::BTreeMap;

impl Compiled {
    /// 外面（と、影・覆いの評価器）を加える。覆われた面積は、アルベド・惑星の赤外の入熱と宇宙への放射をやめる（`Surface::exposed`）。
    pub(super) fn add_surfaces(&mut self, net: &ThermalNetwork) -> Result<(), AnalysisError> {
        // 升に分けた板（部品 → 升の数）。板の面は日照・覆い・向き合いを升の長方形ごとに求める
        let plates: BTreeMap<String, [usize; 2]> = net
            .plates
            .iter()
            .map(|p| (p.part.clone(), p.cells.map(|c| c as usize)))
            .collect();
        self.shading = net.shadowing.as_ref().and_then(|input| {
            LitSurfaces::new_subdivided(
                input,
                Footprint::Credited,
                net.surfaces.iter().map(|s| (s.id.as_str(), s.normal)),
                &plates,
            )
        });
        for (k, s) in net.surfaces.iter().enumerate() {
            for mut surface in self.surface(s, k)? {
                surface.exposed = self.shading.as_ref().map_or(1.0, |l| match surface.cell {
                    Some(c) => l.exposed_cell(k, c),
                    None => l.exposed(k),
                });
                self.space[surface.node] +=
                    surface.eps * SIGMA_SB * surface.area_m2 * surface.exposed;
                self.surfaces.push(surface);
            }
        }
        Ok(())
    }

    /// 外面を節点の面（板なら升ごとに面積の割合で分けた面）にする。
    fn surface(&self, s: &ThermalSurface, input: usize) -> Result<Vec<Surface>, AnalysisError> {
        let (alpha, eps) = s.optics()?;
        match (s.area.value() >= 0.0, s.normal.normalized()) {
            (true, Some(normal)) => {
                let slots = self.slots(&s.node)?;
                // 升の通し番号は、影の評価器が面を分けた升の並びと同じ（`PlateGrid::index`）。数が合うときだけ升ごとに求める
                let per_cell = self
                    .shading
                    .as_ref()
                    .and_then(|l| l.cell_count(input))
                    .is_some_and(|n| n == slots.len() && n > 1);
                Ok(slots
                    .into_iter()
                    .enumerate()
                    .map(|(c, (node, share))| Surface {
                        node,
                        area_m2: s.area.value() * f64::from(s.count) * share,
                        normal,
                        alpha,
                        eps,
                        exposed: 1.0,
                        input,
                        cell: per_cell.then_some(c),
                    })
                    .collect())
            }
            _ => Err(invalid(format!(
                "Surface {}: area must be 0 or more and the normal must be a non-zero vector",
                s.id
            ))),
        }
    }
}
