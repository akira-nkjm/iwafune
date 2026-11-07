//! 熱ネットワークの定常解（Newton 法）。連立一次方程式は `sat_astro::math::solve_dense` で解く。

use super::{Compiled, invalid};
use crate::spec::AnalysisError;
use sat_astro::math::solve_dense;

/// 定常解。
#[derive(Debug, Clone)]
pub(crate) struct SteadySolution {
    pub temps: Vec<f64>,
    pub iterations: u32,
    /// 収束時の熱収支の残差の最大 [W]
    pub residual_w: f64,
}

impl Compiled {
    /// 熱収支のヤコビアン ∂F_i/∂T_j（n×n の行優先）。
    fn jacobian(&self, t: &[f64]) -> Vec<f64> {
        let n = self.len();
        let mut jac = vec![0.0; n * n];
        for i in 0..n {
            jac[i * n + i] -= 4.0 * self.space[i] * t[i].powi(3);
        }
        for &(i, j, g) in &self.cond {
            add_pair(&mut jac, n, (i, j), g, g);
        }
        for &(i, j, sr) in &self.rad {
            let (di, dj) = (4.0 * sr * t[i].powi(3), 4.0 * sr * t[j].powi(3));
            add_pair(&mut jac, n, (i, j), di, dj);
        }
        jac
    }

    /// 定常解を Newton 法で求める（1 回の修正は ±50 K まで）。`guess` は初期値 [K]。
    pub fn solve_steady(
        &self,
        q: &[f64],
        space_k: f64,
        guess: f64,
    ) -> Result<SteadySolution, AnalysisError> {
        let n = self.len();
        let mut t: Vec<f64> = (0..n).map(|i| self.fixed[i].unwrap_or(guess)).collect();
        let scale = q.iter().map(|x| x.abs()).sum::<f64>().max(1.0);
        for iter in 1..=200 {
            let f = self.net_heat(&t, q, space_k);
            let mut jac = self.jacobian(&t);
            let mut rhs: Vec<f64> = f.iter().map(|x| -x).collect();
            for i in 0..n {
                if let Some(tf) = self.fixed[i] {
                    jac[i * n..(i + 1) * n].fill(0.0);
                    jac[i * n + i] = 1.0;
                    rhs[i] = tf - t[i];
                }
            }
            let dt = solve_dense(jac, rhs).ok_or_else(|| {
                invalid(
                    "熱の逃げ道（宇宙への放射面か固定温度の節点）が無い節点があります".to_owned(),
                )
            })?;
            let max_step = dt.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
            let k = if max_step > 50.0 {
                50.0 / max_step
            } else {
                1.0
            };
            for i in 0..n {
                t[i] = (t[i] + k * dt[i]).max(1.0);
            }
            if max_step < 1e-9 {
                let residual = self.residual(&t, q, space_k);
                if residual <= 1e-6 * scale {
                    return Ok(SteadySolution {
                        temps: t,
                        iterations: iter,
                        residual_w: residual,
                    });
                }
            }
        }
        Err(invalid("定常解が収束しません".to_owned()))
    }

    /// 境界節点を除いた熱収支の残差の最大 [W]。
    fn residual(&self, t: &[f64], q: &[f64], space_k: f64) -> f64 {
        self.net_heat(t, q, space_k)
            .iter()
            .enumerate()
            .filter(|(i, _)| self.fixed[*i].is_none())
            .fold(0.0, |m, (_, x)| m.max(x.abs()))
    }
}

/// i–j の対称な結合をヤコビアン（n×n の行優先）に足す（`di` は ∂/∂T_i、`dj` は ∂/∂T_j の係数）。
fn add_pair(jac: &mut [f64], n: usize, (i, j): (usize, usize), di: f64, dj: f64) {
    jac[i * n + i] -= di;
    jac[i * n + j] += dj;
    jac[j * n + j] -= dj;
    jac[j * n + i] += di;
}

#[cfg(test)]
mod tests {
    use super::super::tests::node;
    use super::super::{ThermalConductiveLink, ThermalLoad, ThermalNetwork, ThermalSurface};
    use super::*;
    use sat_astro::constants::SIGMA_SB;
    use sat_astro::math::Vec3;
    use sat_units::typed::Qty;

    fn cond(a: &str, b: &str, g: f64) -> ThermalConductiveLink {
        ThermalConductiveLink {
            a: a.to_owned(),
            b: b.to_owned(),
            conductance: Qty::from_raw(g),
        }
    }

    /// 2 節点の伝導: 固定 300 K の節点に G = 0.5 W/K でつないだ節点に 10 W → 320 K。
    #[test]
    fn two_node_conduction() {
        let net = ThermalNetwork {
            mounts: None,
            nodes: vec![node("sink", Some(300.0)), node("box", None)],
            conductive: vec![cond("sink", "box", 0.5)],
            radiative: vec![],
            surfaces: vec![],
        };
        let c = Compiled::new(&net).unwrap();
        let q = c
            .loads(&[ThermalLoad {
                count: 1,
                node: "box".to_owned(),
                power: Qty::from_raw(10.0),
                heat_fraction: Some(1.0),
            }])
            .unwrap();
        let s = c.solve_steady(&q, 3.0, 290.0).unwrap();
        assert!((s.temps[0] - 300.0).abs() < 1e-9);
        assert!((s.temps[1] - 320.0).abs() < 1e-6, "{}", s.temps[1]);

        // 消費電力 10 W のうち半分が熱になる → 5 W → 310 K。割合が 0〜1 の外ならエラー。
        let half = |f: f64| ThermalLoad {
            count: 1,
            node: "box".to_owned(),
            power: Qty::from_raw(10.0),
            heat_fraction: Some(f),
        };
        let q = c.loads(&[half(0.5)]).unwrap();
        let s = c.solve_steady(&q, 3.0, 290.0).unwrap();
        assert!((s.temps[1] - 310.0).abs() < 1e-6, "{}", s.temps[1]);
        assert!(c.loads(&[half(1.5)]).is_err());
    }

    /// 1 節点の放射平衡: ε σ A T⁴ = Q → T = (Q / (ε σ A))^(1/4)（宇宙 0 K）。
    #[test]
    fn single_node_radiation_balance() {
        let net = ThermalNetwork {
            mounts: None,
            nodes: vec![node("plate", None)],
            conductive: vec![],
            radiative: vec![],
            surfaces: vec![ThermalSurface {
                id: "s".to_owned(),
                node: "plate".to_owned(),
                area: Qty::from_raw(0.1),
                normal: Vec3::X,
                count: 1,
                absorptivity: 0.5,
                emissivity: 0.8,
            }],
        };
        let c = Compiled::new(&net).unwrap();
        let s = c.solve_steady(&[20.0], 0.0, 290.0).unwrap();
        let expected = (20.0 / (0.8 * SIGMA_SB * 0.1)).powf(0.25);
        assert!(
            (s.temps[0] - expected).abs() < 1e-6,
            "{} vs {expected}",
            s.temps[0]
        );
    }

    #[test]
    fn isolated_node_is_rejected() {
        let net = ThermalNetwork {
            mounts: None,
            nodes: vec![node("a", None)],
            conductive: vec![],
            radiative: vec![],
            surfaces: vec![],
        };
        let c = Compiled::new(&net).unwrap();
        assert!(c.solve_steady(&[1.0], 3.0, 290.0).is_err());
    }
}
