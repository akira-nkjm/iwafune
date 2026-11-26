//! 電子と制動放射の線量の核（1 つの入射エネルギーに対する、深さごとの重み）。
//! SHIELDOSE-2（Seltzer, NISTIR 5477, 1994）の電子の手順: ETRAN の計算に基づく深さ線量を、**縮約した深さ**（電子は外挿飛程で割った `z/R_s`、
//! 制動放射は電子の飛程で割った `ln(z/R)`）の関数として表にしたものを、エネルギーについては ln T の**直線**（第 2 版の方法）で、
//! 深さについてはスプライン（電子）か直線（制動放射）で補間する。検出器 / アルミの比は、半無限の媒質（`m = 0`）と有限の板（`m = 1`）の 2 組。

use super::Detector;
use super::data::ShieldingData;
use crate::AstroError;
use crate::math::spline::{ExtendedLinear, NaturalSpline};

fn table_error(what: &str) -> AstroError {
    AstroError::InvalidInput(format!("SHIELDOSE-2 data: the {what} table is not usable"))
}

/// 表の ln T に対する直線の補間。両端の外は端の値（SHIELDOSE の端の扱い）。値は端でそのまま、内側で `floor` 以上に切る。
#[derive(Debug, Clone)]
struct EnergyLine {
    line: ExtendedLinear,
    low: f64,
    high: f64,
}

impl EnergyLine {
    fn new(te_ln: &[f64], values: &[f64]) -> Result<Self, AstroError> {
        let line = ExtendedLinear::new(te_ln, values).ok_or_else(|| table_error("electron"))?;
        Ok(Self {
            line,
            low: values[0],
            high: values[values.len() - 1],
        })
    }

    /// 端の外・端では端の値、内側では直線。`clamp_zero` なら内側の値が負なら 0。
    fn at(&self, ln_t: f64, te_ln: &[f64], clamp_zero: bool) -> f64 {
        let n = te_ln.len();
        if ln_t >= te_ln[n - 1] {
            self.high
        } else if ln_t <= te_ln[0] {
            self.low
        } else {
            let v = self.line.eval(ln_t);
            if clamp_zero { v.max(0.0) } else { v }
        }
    }
}

/// 電子と制動放射の核。
#[derive(Debug, Clone)]
pub(super) struct ElectronKernel {
    te_ln: Vec<f64>,
    range: NaturalSpline,
    yield_: NaturalSpline,
    absorb: NaturalSpline,
    scale: NaturalSpline,
    bs: Vec<f64>,
    zre: Vec<f64>,
    zs_ln: Vec<f64>,
    zb_ln: Vec<f64>,
    dale: Vec<EnergyLine>,
    dalb: Vec<EnergyLine>,
    /// `[m][l]`
    drate: [Vec<EnergyLine>; 2],
    dratb: [Vec<EnergyLine>; 2],
}

/// 電子と制動放射の核の値。`[m][深さ]`（`m` = 0 は半無限の媒質、1 は有限の板）。
#[derive(Debug, Clone)]
pub(super) struct ElectronWeights {
    pub(super) electron: [Vec<f64>; 2],
    pub(super) brems: [Vec<f64>; 2],
}

/// 深さ（g/cm²）と、その自然対数。
pub(super) struct Depths<'a> {
    pub(super) z: &'a [f64],
    pub(super) ln_z: &'a [f64],
}

impl ElectronKernel {
    pub(super) fn new(data: &ShieldingData, detector: Detector) -> Result<Self, AstroError> {
        let ln = |name: &str| -> Result<Vec<f64>, AstroError> {
            let v = data.vector(name);
            if v.iter().any(|x| *x <= 0.0) {
                return Err(table_error(name));
            }
            Ok(v.iter().map(|x| x.ln()).collect())
        };
        let (ee, te_ln) = (ln("ee")?, ln("te")?);
        let spline = |x: &[f64], y: Vec<f64>| {
            NaturalSpline::new(x, &y).ok_or_else(|| table_error("electron"))
        };
        let columns =
            |name: &str, len: usize, take_ln: bool| -> Result<Vec<EnergyLine>, AstroError> {
                (0..len)
                    .map(|l| {
                        let col: Vec<f64> =
                            (0..te_ln.len()).map(|n| data.row(name, &[n])[l]).collect();
                        let col = if take_ln {
                            col.iter().map(|v| v.ln()).collect()
                        } else {
                            col
                        };
                        EnergyLine::new(&te_ln, &col)
                    })
                    .collect()
            };
        let ratio_columns =
            |name: &str, m: usize, len: usize| -> Result<Vec<EnergyLine>, AstroError> {
                (0..len)
                    .map(|l| {
                        let col: Vec<f64> = (0..te_ln.len())
                            .map(|n| data.row(name, &[detector.index(), m, n])[l])
                            .collect();
                        EnergyLine::new(&te_ln, &col)
                    })
                    .collect()
            };
        Ok(Self {
            range: spline(&ee, ln("re")?)?,
            yield_: spline(&ee, ln("ye")?)?,
            absorb: spline(&te_ln, ln("ar")?)?,
            scale: spline(&te_ln, ln("rs")?)?,
            bs: data.vector("bs").to_vec(),
            zre: data.vector("zre").to_vec(),
            zs_ln: ln("zs")?,
            zb_ln: ln("zb")?,
            dale: columns("dale", 34, true)?,
            dalb: columns("dalb", 37, true)?,
            drate: [
                ratio_columns("drate", 0, 51)?,
                ratio_columns("drate", 1, 51)?,
            ],
            dratb: [
                ratio_columns("dratb", 0, 47)?,
                ratio_columns("dratb", 1, 47)?,
            ],
            te_ln,
        })
    }

    /// 入射エネルギー `T = exp(ln_t)` の電子の、各深さでの電子線量の重みと制動放射線量の重み。
    pub(super) fn weights(
        &self,
        ln_t: f64,
        depths: &Depths<'_>,
        detector: Detector,
    ) -> Result<ElectronWeights, AstroError> {
        let at = |cols: &[EnergyLine]| -> Vec<f64> {
            cols.iter()
                .map(|c| c.at(ln_t, &self.te_ln, false))
                .collect()
        };
        let dale =
            NaturalSpline::new(&self.bs, &at(&self.dale)).ok_or_else(|| table_error("electron"))?;
        let dalb = ExtendedLinear::new(&self.zs_ln, &at(&self.dalb))
            .ok_or_else(|| table_error("bremsstrahlung"))?;
        let mut out = ElectronWeights {
            electron: [Vec::new(), Vec::new()],
            brems: [Vec::new(), Vec::new()],
        };
        for m in 0..2 {
            // 半無限の媒質のアルミの比は 1（板の表を使わない）
            let identity = detector == Detector::Aluminum && m == 0;
            let ratio = |cols: &[EnergyLine]| -> Vec<f64> {
                cols.iter()
                    .map(|c| {
                        if identity {
                            1.0
                        } else {
                            c.at(ln_t, &self.te_ln, true)
                        }
                    })
                    .collect()
            };
            out.electron[m] = self.electron_weights(ln_t, depths, &dale, &ratio(&self.drate[m]))?;
            out.brems[m] = self.brems_weights(ln_t, depths, &dalb, &ratio(&self.dratb[m]))?;
        }
        Ok(out)
    }

    /// 電子線量の重み: 縮約した深さ `z/R_s` の曲線 `curve`（ln 線量）と、検出器の比 `ratio`（深さ `z/R` の関数）。
    fn electron_weights(
        &self,
        ln_t: f64,
        depths: &Depths<'_>,
        curve: &NaturalSpline,
        ratio: &[f64],
    ) -> Result<Vec<f64>, AstroError> {
        let t = ln_t.exp();
        let r_in = self.range.eval(ln_t).exp();
        let r_s = r_in * self.scale.eval(ln_t).exp();
        let ares = self.absorb.eval(ln_t).exp();
        let ratio_curve =
            NaturalSpline::new(&self.zre, ratio).ok_or_else(|| table_error("electron ratio"))?;
        let (bs_top, zre_top) = (self.bs[self.bs.len() - 1], self.zre[self.zre.len() - 1]);
        Ok(depths
            .z
            .iter()
            .map(|&z| {
                let zs = z / r_s;
                if zs >= bs_top {
                    return 0.0;
                }
                let g = t * curve.eval(zs).exp() * ares / r_s;
                let zr = z / r_in;
                g * if zr < zre_top {
                    ratio_curve.eval(zr).max(0.0)
                } else {
                    ratio[ratio.len() - 1]
                }
            })
            .collect())
    }

    /// 制動放射線量の重み: `ln(z/R)` の直線の曲線 `curve`（ln 線量）と、検出器の比 `ratio`（`ln z` の関数）。
    fn brems_weights(
        &self,
        ln_t: f64,
        depths: &Depths<'_>,
        curve: &ExtendedLinear,
        ratio: &[f64],
    ) -> Result<Vec<f64>, AstroError> {
        let t = ln_t.exp();
        let r_in = self.range.eval(ln_t).exp();
        let yld = self.yield_.eval(ln_t).exp();
        let ratio_curve = ExtendedLinear::new(&self.zb_ln, ratio)
            .ok_or_else(|| table_error("bremsstrahlung ratio"))?;
        let zb_top = self.zb_ln[self.zb_ln.len() - 1];
        Ok(depths
            .z
            .iter()
            .zip(depths.ln_z)
            .map(|(&z, &lz)| {
                let g = t * curve.eval((z / r_in).ln()).exp() * yld / r_in;
                g * if lz < zb_top {
                    ratio_curve.eval(lz).max(0.0)
                } else {
                    ratio[ratio.len() - 1]
                }
            })
            .collect())
    }
}
