//! 平均の投影面積に使う向きの組。**向きの組はここだけ**（高度低下の平均の面積も、報告の最小・最大・平均もこの組を使う）。
//!
//! 凸でない集まりでも、投影面積は `A(d) = A(−d)`（同じ平面への正射影の向きを逆にしただけ）なので、全球の平均は半球の積分で済む:
//! `⟨A⟩ = (1/2π) ∫₀^{2π} ∫₀^{π/2} A(θ, φ) sin θ dθ dφ`（`θ` は +z からの角）。
//! 積の求積則にする: `θ` は区間 `[0, π/2]` のガウス・ルジャンドル則（`sin θ` を重みの一部として掛ける。被積分関数は `θ` について滑らか）、
//! `φ` は 4 つの象限それぞれにガウス・ルジャンドル則（箱の投影面積の折れ目 `|dx|`・`|dy|` は座標面 `φ = 0, π/2, …` の上にあり、象限の境目に合う）。
//! 軸に平行な箱 1 つなら、各象限の中で被積分関数が解析的なので、求積の誤差は点数でほぼ指数的に減る。
//! 折れ目が象限の境目に無い一般の向きの箱の集まりでは、折れ目をまたぐ区間の誤差が残り、点数の逆 2 乗ほどで減る。
//! 既定は 12 × (4 × 8) = 384 向き。箱 1 つの閉じた式 `(xy + yz + zx)/2` との差を試験で測っている:
//! 軸に平行な箱は 1e-15 の程度、任意に回した箱（折れ目が象限の境目に無い）は 40 通りの乱数の最悪で 1e-3 の程度（8 × 24 = 192 向きで 3e-3、12 × 32 で 1e-3）。
//! 費用は箱 41 個（リリースビルド）で 1 向き約 0.09 ms、全体で約 40 ms。

use sat_astro::math::Vec3;
use sat_astro::math::quadrature::gauss_legendre;
use std::f64::consts::FRAC_PI_2;

/// 極角 `θ` の求積点の数。
pub const POLAR_NODES: usize = 12;
/// 方位角 `φ` の 1 象限あたりの求積点の数（全体は 4 倍）。
pub const AZIMUTH_NODES_PER_QUADRANT: usize = 8;

/// 向きと重みの組（重みの和は 1。全球の平均を、各向きの値の重みつき和で出す）。
#[derive(Debug, Clone, PartialEq)]
pub struct DirectionSet {
    nodes: Vec<(Vec3, f64)>,
}

impl DirectionSet {
    /// 半球の積の求積則（極角 `polar` 点 × 方位角 `4 × per_quadrant` 点）。
    pub fn hemisphere(polar: usize, per_quadrant: usize) -> Self {
        let theta = gauss_legendre(polar);
        let phi = gauss_legendre(per_quadrant);
        let mut nodes = Vec::with_capacity(polar * 4 * per_quadrant);
        for (xt, wt) in &theta {
            let angle = FRAC_PI_2 * 0.5 * (1.0 + xt);
            let polar_weight = FRAC_PI_2 * 0.5 * wt * angle.sin();
            for quadrant in 0..4 {
                for (xp, wp) in &phi {
                    let azimuth = FRAC_PI_2 * (f64::from(quadrant) + 0.5 * (1.0 + xp));
                    let azimuth_weight = FRAC_PI_2 * 0.5 * wp;
                    let (sin_t, cos_t) = angle.sin_cos();
                    let (sin_p, cos_p) = azimuth.sin_cos();
                    nodes.push((
                        Vec3::new(sin_t * cos_p, sin_t * sin_p, cos_t),
                        polar_weight * azimuth_weight / (2.0 * std::f64::consts::PI),
                    ));
                }
            }
        }
        Self { nodes }
    }

    /// 既定の向きの組（[`POLAR_NODES`] × 4 [`AZIMUTH_NODES_PER_QUADRANT`] 点）。
    pub fn standard() -> Self {
        Self::hemisphere(POLAR_NODES, AZIMUTH_NODES_PER_QUADRANT)
    }

    /// 向き（単位ベクトル。上半球）と重み。
    pub fn nodes(&self) -> &[(Vec3, f64)] {
        &self.nodes
    }
}
