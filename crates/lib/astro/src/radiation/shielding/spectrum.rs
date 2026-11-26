//! SHIELDOSE-2 のエネルギー側の準備: 対数等間隔のエネルギー格子、入射スペクトルの格子への補間、シンプソン則の積分。
//! （Seltzer, NBS TN 1116, 1980 と NISTIR 5477, 1994 の手順。積分変数は ln E で、被積分関数は E × 微分フラックス。）
//!
//! 入力のスペクトルは**微分の全方向フラックス** [cm⁻² s⁻¹ MeV⁻¹] の表。ln(フラックス) を ln E の自然 3 次スプラインで結び、
//! 表の範囲の外（半格子分より外）は使わない。AP-8・AE-8 は積分フラックスなので、[`differential_from_integral`] で微分にする。

use crate::AstroError;
use crate::math::spline::NaturalSpline;

/// 対数等間隔のエネルギー格子 `E_k = e_min · exp(k · step)`（k = 0..points−1）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyGrid {
    e_min: f64,
    step: f64,
    points: usize,
}

impl EnergyGrid {
    /// 格子を作る。`0 < e_min < e_max`、点は 3 つ以上。
    pub fn new(e_min: f64, e_max: f64, points: usize) -> Result<Self, AstroError> {
        if !(e_min > 0.0 && e_max > e_min && e_max.is_finite()) || points < 3 {
            return Err(AstroError::InvalidInput(
                "Energy grid: need 0 < e_min < e_max and at least 3 points".into(),
            ));
        }
        Ok(Self {
            e_min,
            step: (e_max / e_min).ln() / (points - 1) as f64,
            points,
        })
    }

    /// 点の数。
    pub fn points(&self) -> usize {
        self.points
    }

    /// ln E の刻み。
    pub fn step(&self) -> f64 {
        self.step
    }

    /// 点 `k` の ln E。
    pub fn ln_energy(&self, k: usize) -> f64 {
        self.e_min.ln() + k as f64 * self.step
    }

    /// 点 `k` のエネルギー [MeV]。
    pub fn energy(&self, k: usize) -> f64 {
        self.ln_energy(k).exp()
    }

    /// エネルギーの範囲 `[lo, hi]` に当たる格子点の最初と最後（0 始まり）。最も近い格子点に丸める（SHIELDOSE の `EINDEX`）。
    pub fn index_range(&self, lo: f64, hi: f64) -> (usize, usize) {
        let near = |e: f64| ((e / self.e_min).ln() / self.step + 0.5).trunc();
        let first = near(lo).max(0.0) as usize;
        let last = (near(hi).max(0.0) as usize).min(self.points - 1);
        (first.min(last), last)
    }
}

/// 格子の上のスペクトル。各点の値は `E × 微分フラックス`（ln E で積分する被積分関数）。
#[derive(Debug, Clone, PartialEq)]
pub struct GridSpectrum {
    grid: EnergyGrid,
    first: usize,
    /// `first` から続く各点の `E × 微分フラックス`
    values: Vec<f64>,
}

/// SHIELDOSE-2 の積分（`INTEG`）: 等間隔の点の台形でなくシンプソン則（点が偶数のときは先頭の 6 点まで 3/8 則の重み）。
/// 返すのは和に刻みの 1/3 を掛けたもの。
pub(super) fn simpson(delta: f64, g: &[f64]) -> f64 {
    let n = g.len();
    let sum = |from: usize, to: usize| -> f64 {
        (from..=to).step_by(2).filter_map(|k| g.get(k - 1)).sum()
    };
    let sigma = if n == 0 || n == 1 {
        0.0
    } else if n % 2 == 1 {
        if n == 3 {
            g[0] + 4.0 * g[1] + g[2]
        } else {
            g[0] + 4.0 * sum(2, n - 1) + 2.0 * sum(3, n - 2) + g[n - 1]
        }
    } else if n == 2 {
        1.5 * (g[0] + g[1])
    } else if n == 4 {
        1.125 * (g[0] + 3.0 * g[1] + 3.0 * g[2] + g[3])
    } else {
        let six = g[0] + 3.875 * g[1] + 2.625 * g[2] + 2.625 * g[3] + 3.875 * g[4] + g[5];
        if n == 6 {
            six
        } else {
            six + g[5] + 4.0 * sum(7, n - 1) + 2.0 * sum(8, n - 2) + g[n - 1]
        }
    };
    delta * sigma
}

impl GridSpectrum {
    /// 微分フラックスの表（`energies_mev` が増える順で 3 点以上、`per_mev` が正）から格子の上のスペクトルを作る。
    /// 格子点は表の範囲に最も近い点までで、表の外へはスプラインを延ばさない（半格子分の端だけ延びる）。
    pub fn from_table(
        grid: EnergyGrid,
        energies_mev: &[f64],
        per_mev: &[f64],
    ) -> Result<Self, AstroError> {
        let bad = |m: &str| Err(AstroError::InvalidInput(format!("Spectrum: {m}")));
        if energies_mev.len() < 3 || energies_mev.len() != per_mev.len() {
            return bad("need at least 3 points and equal lengths");
        }
        if energies_mev
            .iter()
            .chain(per_mev)
            .any(|v| !(*v > 0.0 && v.is_finite()))
        {
            return bad("energies and fluxes must be positive and finite");
        }
        let ln_e: Vec<f64> = energies_mev.iter().map(|e| e.ln()).collect();
        let ln_s: Vec<f64> = per_mev.iter().map(|s| s.ln()).collect();
        let Some(spline) = NaturalSpline::new(&ln_e, &ln_s) else {
            return bad("energies must increase");
        };
        let (first, last) = grid.index_range(energies_mev[0], energies_mev[energies_mev.len() - 1]);
        let values = (first..=last)
            .map(|k| grid.energy(k) * spline.eval(grid.ln_energy(k)).exp())
            .collect();
        Ok(Self {
            grid,
            first,
            values,
        })
    }

    /// エネルギーの指数分布 `dN/dE = c/e0 · exp(−E/e0)`（TN 1116 の付録の例のスペクトル。`c` はフルエンス）。
    /// 範囲 `[lo, hi]` の格子点だけに値を持つ。
    pub fn exponential(grid: EnergyGrid, lo: f64, hi: f64, e0: f64, c: f64) -> Self {
        let (first, last) = grid.index_range(lo, hi);
        let values = (first..=last)
            .map(|k| {
                let e = grid.energy(k);
                let arg = e / e0;
                if arg > 85.0 {
                    0.0
                } else {
                    e * c / e0 * (-arg).exp()
                }
            })
            .collect();
        Self {
            grid,
            first,
            values,
        }
    }

    /// 格子。
    pub fn grid(&self) -> &EnergyGrid {
        &self.grid
    }

    /// 最初の格子点の番号。
    pub fn first(&self) -> usize {
        self.first
    }

    /// 各点の `E × 微分フラックス`。
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// 積分フラックス（全エネルギー）[cm⁻² s⁻¹]（SHIELDOSE の `INT SPEC`）。
    pub fn integral_flux(&self) -> f64 {
        simpson(self.grid.step() / 3.0, &self.values)
    }

    /// 平均エネルギー [MeV]（SHIELDOSE の `EAV`）。
    pub fn mean_energy(&self) -> f64 {
        let weighted: Vec<f64> = self
            .values
            .iter()
            .enumerate()
            .map(|(i, v)| v * self.grid.energy(self.first + i))
            .collect();
        simpson(self.grid.step() / 3.0, &weighted) / self.integral_flux()
    }
}

/// 積分フラックス `F(>E)` の表から、微分フラックスの表を作る。
///
/// 隣り合う 2 点の差分 `(F_i − F_{i+1}) / (E_{i+1} − E_i)` をその区間の幾何平均のエネルギーに置く（対数等間隔の格子なら、
/// 算術平均との違いは刻みの 2 乗の 1/8 程度で、無視できる）。端の処理:
///
/// - **低エネルギー端**: 先頭の区間の中点より下の半区間の粒子が落ちるので、ln(微分) を ln E の直線で先頭のエネルギーまで延ばした点を足す。
/// - **高エネルギー端**: `F` が 0 になった所から先は差分が 0 で対数が取れない。**正の差分が続く範囲だけ**を返す
///   （`F` が 0 になる区間は、その区間の平均として `F_i / ΔE` を置く。粒子数はこの区間まで保たれる）。
///
/// 戻り値は `(エネルギー [MeV], 微分フラックス [/MeV])`。正の差分が 2 区間未満なら空。
pub fn differential_from_integral(energies_mev: &[f64], integral: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut es = Vec::new();
    let mut js = Vec::new();
    if energies_mev.len() != integral.len() {
        return (es, js);
    }
    let mut start = 0;
    for (i, (e, f)) in energies_mev.windows(2).zip(integral.windows(2)).enumerate() {
        let j = (f[0] - f[1]) / (e[1] - e[0]);
        if j > 0.0 && j.is_finite() {
            if js.is_empty() {
                start = i;
            }
            es.push((e[0] * e[1]).sqrt());
            js.push(j);
        } else if !js.is_empty() {
            break;
        }
    }
    if es.len() < 2 {
        return (Vec::new(), Vec::new());
    }
    // 先頭の区間より下の半区間: ln j を ln E の直線で先頭のエネルギーまで延ばす。
    let first_e = energies_mev[0];
    let (e0, j0) = (es[0], js[0]);
    if start == 0 && first_e < e0 {
        let slope = (js[1].ln() - j0.ln()) / (es[1].ln() - e0.ln());
        es.insert(0, first_e);
        js.insert(0, (j0.ln() + slope * (first_e.ln() - e0.ln())).exp());
    }
    (es, js)
}
