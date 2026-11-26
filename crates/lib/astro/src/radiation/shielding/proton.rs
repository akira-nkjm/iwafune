//! 陽子の線量の核（1 つの入射エネルギーに対する、深さごとの「単位フルエンスあたりの線量」の重み）。
//! SHIELDOSE-2（Seltzer, NISTIR 5477, 1994）の陽子の手順: 深さを CSDA 飛程で割った量 `z/R` の関数として表にした深さ線量を、
//! エネルギーについては表のエネルギー（縮約エネルギー `tp`）の自然スプラインで、`z/R` についてはスプラインで補間する。

use super::data::ShieldingData;
use super::{Detector, NuclearOption};
use crate::AstroError;
use crate::math::spline::NaturalSpline;

/// 核相互作用で中性子へ行ったエネルギーが、遮蔽を通るときに指数で減るときの減衰係数 [cm²/g]（SHIELDOSE-2 の `ENMU`）。
const NEUTRON_ATTENUATION: f64 = 0.03;

/// 陽子の核。検出器と核相互作用の選択ごとに作る。
#[derive(Debug, Clone)]
pub(super) struct ProtonKernel {
    /// ln E → ln R（アルミの CSDA 飛程）
    range: NaturalSpline,
    tp_ln: Vec<f64>,
    /// 深さの格子の点ごとの、ln(深さ線量) の ln E に対するスプライン
    dose: Vec<NaturalSpline>,
    /// 検出器 / アルミの比の同様のスプライン
    ratio: Vec<NaturalSpline>,
    /// 表の両端のエネルギーでの（ln 線量, 比）
    low: Vec<(f64, f64)>,
    high: Vec<(f64, f64)>,
    zrp: Vec<f64>,
    /// 中性子へ行くエネルギーの割合（ln E に対して）。核相互作用ありのときだけ
    neutron: NaturalSpline,
    neutron_start_ln: f64,
    nuclear: NuclearOption,
}

fn table_error(what: &str) -> AstroError {
    AstroError::InvalidInput(format!("SHIELDOSE-2 data: the {what} table is not usable"))
}

impl ProtonKernel {
    pub(super) fn new(
        data: &ShieldingData,
        detector: Detector,
        nuclear: NuclearOption,
    ) -> Result<Self, AstroError> {
        let ln = |v: &[f64]| -> Result<Vec<f64>, AstroError> {
            if v.iter().any(|x| *x <= 0.0) {
                return Err(table_error("proton"));
            }
            Ok(v.iter().map(|x| x.ln()).collect())
        };
        let range = NaturalSpline::new(&ln(data.vector("ep"))?, &ln(data.vector("rp"))?)
            .ok_or_else(|| table_error("proton range"))?;
        let tp_ln = ln(data.vector("tp"))?;
        let tepn_ln = ln(data.vector("tepn"))?;
        let neutron = NaturalSpline::new(&tepn_ln, data.vector("fepn"))
            .ok_or_else(|| table_error("neutron fraction"))?;
        let att = usize::from(nuclear != NuclearOption::None);
        let zrp = data.vector("zrp").to_vec();
        let (mut dose, mut ratio, mut low, mut high) = (vec![], vec![], vec![], vec![]);
        for l in 0..zrp.len() {
            let dose_col: Vec<f64> = (0..tp_ln.len())
                .map(|n| data.row("dalp", &[att, n])[l])
                .collect();
            let ratio_col: Vec<f64> = (0..tp_ln.len())
                .map(|n| data.row("dratp", &[detector.index(), n])[l])
                .collect();
            let dose_ln = ln(&dose_col)?;
            let n = tp_ln.len();
            low.push((dose_ln[0], ratio_col[0]));
            high.push((dose_ln[n - 1], ratio_col[n - 1]));
            dose.push(
                NaturalSpline::new(&tp_ln, &dose_ln).ok_or_else(|| table_error("proton dose"))?,
            );
            ratio.push(
                NaturalSpline::new(&tp_ln, &ratio_col)
                    .ok_or_else(|| table_error("proton ratio"))?,
            );
        }
        Ok(Self {
            range,
            tp_ln,
            dose,
            ratio,
            low,
            high,
            zrp,
            neutron,
            neutron_start_ln: tepn_ln[0],
            nuclear,
        })
    }

    /// 入射エネルギー `E = exp(ln_e)` [MeV] の陽子の、各深さ（`depths` [g/cm²]）での重み
    /// `E · D(z/R) / R`（単位フルエンスあたりの線量の MeV·cm²/g 相当。`D` は表の深さ線量 × 検出器の比）。
    /// 飛程 `R` より深ければ 0。核相互作用の選択 3 では中性子のエネルギーの指数減衰の項を足す。
    pub(super) fn weights(&self, ln_e: f64, depths: &[f64], detector: Detector) -> Vec<f64> {
        let e = ln_e.exp();
        let r = self.range.eval(ln_e).exp();
        let n = self.tp_ln.len();
        let din: Vec<f64> = (0..self.zrp.len())
            .map(|l| {
                let (ans, ratio) = if ln_e >= self.tp_ln[n - 1] {
                    (self.high[l].0, self.high[l].1)
                } else if ln_e <= self.tp_ln[0] {
                    (self.low[l].0, self.low[l].1)
                } else {
                    let ratio = if detector == Detector::Aluminum {
                        1.0
                    } else {
                        self.ratio[l].eval(ln_e)
                    };
                    (self.dose[l].eval(ln_e), ratio)
                };
                ans + ratio.ln()
            })
            .collect();
        // 中性子へ行くエネルギー（選択 2 以上）。表の最小エネルギー以下では 0。
        let neutron_energy = if self.nuclear != NuclearOption::None && ln_e > self.neutron_start_ln
        {
            e * self.neutron.eval(ln_e)
        } else {
            0.0
        };
        let Some(curve) = NaturalSpline::new(&self.zrp, &din) else {
            return vec![0.0; depths.len()];
        };
        let top = self.zrp[self.zrp.len() - 1];
        depths
            .iter()
            .map(|&z| {
                let zr = z / r;
                let mut g = if zr < top {
                    e * curve.eval(zr).exp() / r
                } else {
                    0.0
                };
                if self.nuclear == NuclearOption::LocalWithNeutrons && ln_e > self.neutron_start_ln
                {
                    g += neutron_energy * NEUTRON_ATTENUATION * (-NEUTRON_ATTENUATION * z).exp();
                }
                g
            })
            .collect()
    }
}
