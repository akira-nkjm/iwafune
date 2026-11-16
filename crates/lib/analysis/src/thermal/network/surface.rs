//! 宇宙を向く外面と、表面材（面積割合と α/ε）から面の実効の α/ε を求める計算。**表面材の混ぜ方はここだけ。**
//!
//! 1 枚の面は 1 つの節点（等温）に属するので、吸収（α に比例）も放射（ε に比例）も面積に線形で、
//! 表面材 k の面積割合 f_k から α_eff = Σ f_k α_k、ε_eff = Σ f_k ε_k が厳密に成り立つ。
//! 面積割合の合計を 1 とみなす許容差は割合の合計の共通の基準 `modes::DUTY_TOLERANCE`。

use crate::modes::DUTY_TOLERANCE;
use crate::spec::AnalysisError;
use sat_astro::math::Vec3;
use sat_units::typed::{Area, Qty};
use serde::{Deserialize, Serialize};

/// 面の一部を覆う表面材（塗装・テープ・太陽電池セルなど）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalCoating {
    /// 表面材の ID
    pub id: String,
    /// 面の面積のうちこの表面材の割合（0〜1。面の表面材の合計は 1）
    pub area_fraction: f64,
    /// 太陽光の吸収率 α
    pub absorptivity: f64,
    /// 赤外の放射率 ε
    pub emissivity: f64,
}

/// 宇宙を向く外面（入熱を受け、宇宙へ放射する）。1 節点に複数の面を付けてよい。
/// 光学特性は一様な `absorptivity`・`emissivity` か、表面材の並び `coatings` のどちらか一方で与える。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalSurface {
    pub id: String,
    /// 面が属する節点
    pub node: String,
    pub area: Qty<Area>,
    /// 外向きの法線（機体座標系。正規化して使う）
    pub normal: Vec3,
    /// 枚数（実効面積 = 面積 × 枚数。省略で 1、0 なら数えない）
    #[serde(default = "crate::spec::default_count")]
    pub count: u32,
    /// 面全体が一様なときの太陽光の吸収率 α
    #[serde(default)]
    pub absorptivity: Option<f64>,
    /// 面全体が一様なときの赤外の放射率 ε
    #[serde(default)]
    pub emissivity: Option<f64>,
    /// 表面材ごとの面積割合と α/ε（面積割合の合計は 1）
    #[serde(default)]
    pub coatings: Vec<ThermalCoating>,
}

impl ThermalSurface {
    /// 面の実効の (α, ε)。一様な値か、表面材の面積割合の加重和（モジュールの説明）。
    pub fn optics(&self) -> Result<(f64, f64), AnalysisError> {
        let unit = |v: f64| (0.0..=1.0).contains(&v);
        let (alpha, eps) = match (self.absorptivity, self.emissivity, self.coatings.is_empty()) {
            (Some(a), Some(e), true) => (a, e),
            (None, None, false) => {
                let mut sum = (0.0, 0.0, 0.0);
                for c in &self.coatings {
                    if !(unit(c.area_fraction) && unit(c.absorptivity) && unit(c.emissivity)) {
                        return Err(
                            self.invalid("coating area fractions, α and ε must be within 0 to 1")
                        );
                    }
                    sum.0 += c.area_fraction;
                    sum.1 += c.area_fraction * c.absorptivity;
                    sum.2 += c.area_fraction * c.emissivity;
                }
                if (sum.0 - 1.0).abs() > DUTY_TOLERANCE {
                    return Err(self.invalid("coating area fractions must sum to 1"));
                }
                (sum.1, sum.2)
            }
            _ => {
                return Err(self.invalid("give either both α and ε or coatings, not both"));
            }
        };
        if unit(alpha) && unit(eps) {
            Ok((alpha, eps))
        } else {
            Err(self.invalid("α and ε must be within 0 to 1"))
        }
    }

    fn invalid(&self, rule: &str) -> AnalysisError {
        AnalysisError::InvalidInput(format!("Surface {}: {rule}", self.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surface(
        alpha: Option<f64>,
        eps: Option<f64>,
        coatings: Vec<ThermalCoating>,
    ) -> ThermalSurface {
        ThermalSurface {
            id: "px".into(),
            node: "n".into(),
            area: Qty::from_raw(0.1),
            normal: Vec3::X,
            count: 1,
            absorptivity: alpha,
            emissivity: eps,
            coatings,
        }
    }

    fn coat(f: f64, a: f64, e: f64) -> ThermalCoating {
        ThermalCoating {
            id: "c".into(),
            area_fraction: f,
            absorptivity: a,
            emissivity: e,
        }
    }

    /// 手計算: 白色塗装（α 0.2, ε 0.9）70 % とセル（α 0.9, ε 0.85）30 % → α = 0.14 + 0.27 = 0.41、
    /// ε = 0.63 + 0.255 = 0.885。一様な値はそのまま。合計が 1 でない・両方書く・どちらも無いは入力の誤り。
    #[test]
    fn coatings_mix_by_area_fraction() {
        let s = surface(None, None, vec![coat(0.7, 0.2, 0.9), coat(0.3, 0.9, 0.85)]);
        let (a, e) = s.optics().unwrap();
        assert!((a - 0.41).abs() < 1e-12);
        assert!((e - 0.885).abs() < 1e-12);
        assert_eq!(
            surface(Some(0.3), Some(0.8), vec![]).optics().unwrap(),
            (0.3, 0.8)
        );
        assert!(
            surface(None, None, vec![coat(0.6, 0.2, 0.9)])
                .optics()
                .is_err()
        );
        assert!(
            surface(Some(0.3), Some(0.8), vec![coat(1.0, 0.2, 0.9)])
                .optics()
                .is_err()
        );
        assert!(surface(None, None, vec![]).optics().is_err());
        assert!(surface(Some(1.2), Some(0.8), vec![]).optics().is_err());
    }
}
