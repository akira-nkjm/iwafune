//! SHIELDOSE-2 の線量の組み立て: 格子の上のスペクトルと核の積を ln E で積分して深さごとの線量にし、球の中心の線量を板の曲線から求める。
//! （Seltzer, NBS TN 1116, 1980 と NISTIR 5477, 1994 の手順。）
//!
//! # 線量の規格化と 3 つの幾何
//!
//! 入力は全方向（4π）の微分フラックス `φ`。**板**（半無限の媒質・有限の板）は片面だけに等方に入射する場合で、面を横切る粒子の数は `φ/4`、
//! 斜めに入る粒子の経路が長いぶんの平均 `⟨1/cosθ⟩ = 2` を含めて、深さ 0 の極限の線量は `φ/2 × 阻止能`（片面の半空間の寄与）になる。
//! SHIELDOSE-2 の単位定数の `1/4` はこの `φ/4` で、深さ線量の表が `2` を含む。
//! **球の中心**は全方向から入るので、同じ極限で `φ × 阻止能`。SHIELDOSE-2 が出す球の値は「板の曲線から `D(1 − dlnD/dlnz)` で求めた」もので、
//! 見出しが `1/2 DOSE AT CENTER OF ALUMINUM SPHERES` となっている通り、**球の中心の実際の線量の半分**（板と同じ片面の規格化）になる。
//! このモジュールの [`GeometryDoses::sphere_half`] は SHIELDOSE-2 が出す値そのまま、実際の線量は [`GeometryDoses::sphere_centre`]（2 倍）。
//! 板の 2 つは `semi_infinite` と `finite_slab`（有限の板の透過面）で、陽子では同じ値（直進近似）。

use super::electron::{Depths, ElectronKernel};
use super::proton::ProtonKernel;
use super::spectrum::{GridSpectrum, simpson};
use super::{Detector, NuclearOption, ShieldingData};
use crate::AstroError;
use crate::math::spline::NaturalSpline;

/// 線量の単位定数 [Gy g/MeV]（SHIELDOSE-2 の `RADCON` = 1.6021892e-8 rad g/MeV の Gy 版）。
pub const GY_PER_MEV_PER_G: f64 = 1.6021892e-10;

/// SHIELDOSE-2 が扱う遮蔽の最小の深さ [g/cm²]（それより薄い入力はこの値にする）。
pub const MIN_DEPTH_G_CM2: f64 = 1.0e-6;

/// 遮蔽材（アルミ）の密度 [g/cm³]（SHIELDOSE-2 が厚さと深さの換算に使う値）。
pub const AL_DENSITY_G_CM3: f64 = 2.70;

/// アルミの厚さ [m] を面密度 [g/cm²] にする。
pub fn aluminium_depth_g_cm2(thickness_m: f64) -> f64 {
    /// 1 m = 100 cm（厚さ [m] を密度 [g/cm³] に合わせる）。
    const CM_PER_M: f64 = 100.0;
    thickness_m * CM_PER_M * AL_DENSITY_G_CM3
}

/// 3 つの幾何の線量 [Gy/s]（フラックスが cm⁻² s⁻¹ MeV⁻¹ のとき。深さごと）。
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryDoses {
    /// 半無限のアルミの媒質の中（片面の規格化）
    pub semi_infinite: Vec<f64>,
    /// 有限のアルミの板の透過面（片面の規格化）
    pub finite_slab: Vec<f64>,
    /// SHIELDOSE-2 が出す、アルミの球の中心の「半分の線量」（片面と同じ規格化。[モジュールの説明](self)）
    pub sphere_half: Vec<f64>,
}

impl GeometryDoses {
    /// 球の中心の実際の線量（`sphere_half` の 2 倍。全方向から入射する）。
    pub fn sphere_centre(&self) -> Vec<f64> {
        self.sphere_half.iter().map(|d| 2.0 * d).collect()
    }
}

/// 電子と制動放射の線量。
#[derive(Debug, Clone, PartialEq)]
pub struct ElectronDoses {
    pub electrons: GeometryDoses,
    pub bremsstrahlung: GeometryDoses,
}

/// 検出器と核相互作用の選択ごとにそろえた線量の計算。
#[derive(Debug, Clone)]
pub struct ShieldingModel {
    proton: ProtonKernel,
    electron: ElectronKernel,
    detector: Detector,
}

/// 深さ（g/cm²。厚い順でなく増える順）を検査し、最小の深さで切って、`ln z` とともに返す。
fn prepare_depths(depths: &[f64]) -> Result<(Vec<f64>, Vec<f64>), AstroError> {
    if depths.is_empty() || depths.iter().any(|d| !d.is_finite()) {
        return Err(AstroError::InvalidInput(
            "Depths must be finite and not empty".into(),
        ));
    }
    let z: Vec<f64> = depths.iter().map(|d| d.max(MIN_DEPTH_G_CM2)).collect();
    if z.windows(2).any(|w| w[1] <= w[0]) {
        return Err(AstroError::InvalidInput(
            "Depths must be strictly increasing".into(),
        ));
    }
    let ln_z = z.iter().map(|d| d.ln()).collect();
    Ok((z, ln_z))
}

/// 板の曲線から球の中心の「半分の線量」（`SPHERE`）: 直進近似で `D_sphere = D (1 − dlnD/dlnz)`。
/// 先頭から線量が正の深さ（3 点以上）までで求め、線量が 0 になった深さから先は 0。
/// 正の深さが 3 点未満なら補間できないので全部 0。
pub(super) fn sphere_from_slab(ln_z: &[f64], dose: &[f64]) -> Vec<f64> {
    let positive = dose.iter().take_while(|d| **d > 0.0).count();
    let mut out = vec![0.0; dose.len()];
    if positive < 3 {
        return out;
    }
    let ln_d: Vec<f64> = dose[..positive].iter().map(|d| d.ln()).collect();
    let Some(spline) = NaturalSpline::new(&ln_z[..positive], &ln_d) else {
        return out;
    };
    for i in 0..positive {
        out[i] = dose[i] * (1.0 - spline.derivative(ln_z[i]));
    }
    out
}

fn geometry(semi: Vec<f64>, finite: Vec<f64>, ln_z: &[f64]) -> GeometryDoses {
    let sphere_half = sphere_from_slab(ln_z, &semi);
    GeometryDoses {
        semi_infinite: semi,
        finite_slab: finite,
        sphere_half,
    }
}

/// 点ごとの重み `weights[k][i]` とスペクトルの積を ln E で積分して、深さ `i` ごとの線量 [Gy/s] にする。
fn integrate(spectrum: &GridSpectrum, weights: &[Vec<f64>], depth_count: usize) -> Vec<f64> {
    let delta = spectrum.grid().step() / 3.0;
    (0..depth_count)
        .map(|i| {
            let g: Vec<f64> = spectrum
                .values()
                .iter()
                .zip(weights)
                .map(|(s, w)| s * w[i])
                .collect();
            GY_PER_MEV_PER_G / 4.0 * simpson(delta, &g)
        })
        .collect()
}

impl ShieldingModel {
    /// データベースから、検出器と陽子の核相互作用の選択に合わせて作る。
    pub fn new(
        data: &ShieldingData,
        detector: Detector,
        nuclear: NuclearOption,
    ) -> Result<Self, AstroError> {
        Ok(Self {
            proton: ProtonKernel::new(data, detector, nuclear)?,
            electron: ElectronKernel::new(data, detector)?,
            detector,
        })
    }

    /// 陽子のスペクトルの、深さ（g/cm²。増える順）ごとの線量 [Gy/s]。陽子では 2 つの板は同じ。
    pub fn proton_dose(
        &self,
        spectrum: &GridSpectrum,
        depths_g_cm2: &[f64],
    ) -> Result<GeometryDoses, AstroError> {
        let (z, ln_z) = prepare_depths(depths_g_cm2)?;
        let weights: Vec<Vec<f64>> = (0..spectrum.values().len())
            .map(|k| {
                let ln_e = spectrum.grid().ln_energy(spectrum.first() + k);
                self.proton.weights(ln_e, &z, self.detector)
            })
            .collect();
        let slab = integrate(spectrum, &weights, z.len());
        Ok(geometry(slab.clone(), slab, &ln_z))
    }

    /// 電子のスペクトルの、深さ（g/cm²。増える順）ごとの電子と制動放射の線量 [Gy/s]。
    pub fn electron_dose(
        &self,
        spectrum: &GridSpectrum,
        depths_g_cm2: &[f64],
    ) -> Result<ElectronDoses, AstroError> {
        let (z, ln_z) = prepare_depths(depths_g_cm2)?;
        let depths = Depths { z: &z, ln_z: &ln_z };
        let mut nodes = Vec::with_capacity(spectrum.values().len());
        for k in 0..spectrum.values().len() {
            let ln_t = spectrum.grid().ln_energy(spectrum.first() + k);
            nodes.push(self.electron.weights(ln_t, &depths, self.detector)?);
        }
        let pick = |m: usize, brems: bool| -> Vec<Vec<f64>> {
            nodes
                .iter()
                .map(|n| {
                    if brems {
                        n.brems[m].clone()
                    } else {
                        n.electron[m].clone()
                    }
                })
                .collect()
        };
        let one = |brems: bool| {
            let semi = integrate(spectrum, &pick(0, brems), z.len());
            let finite = integrate(spectrum, &pick(1, brems), z.len());
            geometry(semi, finite, &ln_z)
        };
        Ok(ElectronDoses {
            electrons: one(false),
            bremsstrahlung: one(true),
        })
    }
}
