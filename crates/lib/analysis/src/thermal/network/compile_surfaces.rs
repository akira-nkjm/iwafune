//! 外面を添字に直す（`Compiled::new` の一部）: 外面を節点の面（板なら升ごとの面）にし、影・覆いの評価器を作り、
//! 覆われた面積だけ宇宙への放射の係数を減らす。

use super::{Compiled, Surface, ThermalNetwork, ThermalSurface, invalid};
use crate::spec::AnalysisError;
use crate::structure::external_geometry::{Footprint, LitSurfaces};
use sat_astro::constants::SIGMA_SB;

impl Compiled {
    /// 外面（と、影・覆いの評価器）を加える。覆われた面積は、アルベド・惑星の赤外の入熱と宇宙への放射をやめる（`Surface::exposed`）。
    pub(super) fn add_surfaces(&mut self, net: &ThermalNetwork) -> Result<(), AnalysisError> {
        self.shading = net.shadowing.as_ref().and_then(|input| {
            LitSurfaces::new(
                input,
                Footprint::Credited,
                net.surfaces.iter().map(|s| (s.id.as_str(), s.normal)),
            )
        });
        for (k, s) in net.surfaces.iter().enumerate() {
            let exposed = self.shading.as_ref().map_or(1.0, |l| l.exposed(k));
            for mut surface in self.surface(s, k)? {
                surface.exposed = exposed;
                self.space[surface.node] += surface.eps * SIGMA_SB * surface.area_m2 * exposed;
                self.surfaces.push(surface);
            }
        }
        Ok(())
    }

    /// 外面を節点の面（板なら升ごとに面積の割合で分けた面）にする。
    fn surface(&self, s: &ThermalSurface, input: usize) -> Result<Vec<Surface>, AnalysisError> {
        let (alpha, eps) = s.optics()?;
        match (s.area.value() >= 0.0, s.normal.normalized()) {
            (true, Some(normal)) => Ok(self
                .slots(&s.node)?
                .into_iter()
                .map(|(node, share)| Surface {
                    node,
                    area_m2: s.area.value() * f64::from(s.count) * share,
                    normal,
                    alpha,
                    eps,
                    exposed: 1.0,
                    input,
                })
                .collect()),
            _ => Err(invalid(format!(
                "Surface {}: area must be 0 or more and the normal must be a non-zero vector",
                s.id
            ))),
        }
    }
}
