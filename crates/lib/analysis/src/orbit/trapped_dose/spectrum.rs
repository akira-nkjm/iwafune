//! `orbit.trapped_dose` の入口の準備: 軌道平均の積分スペクトルを細かいエネルギー格子で引き、微分して SHIELDOSE-2 に渡すスペクトルにする。

use sat_astro::geomag::NoL;
use sat_astro::radiation::shielding::{EnergyGrid, GridSpectrum, differential_from_integral};
use sat_astro::radiation::{BeltPosition, Species, TrappedMap};
use sat_astro::timeline::time_mean;

/// 陽子の積分フラックスを引くエネルギーの範囲 [MeV]（AP-8 の範囲）。
pub const PROTON_RANGE_MEV: (f64, f64) = (0.1, 400.0);
/// 電子の範囲 [MeV]（AE-8 の範囲）。
pub const ELECTRON_RANGE_MEV: (f64, f64) = (0.04, 7.0);
/// 積分フラックスを引くエネルギーの密度（10 倍あたりの点の数。隣り合う点のエネルギー比は約 1.10）。
pub const POINTS_PER_DECADE: f64 = 25.0;
/// SHIELDOSE-2 の積分に使う ln E の格子点の数（SHIELDOSE-2 の最大 1001）。
pub const INTEGRATION_POINTS: usize = 1001;

/// 粒子 1 種の、線量の計算へ渡すスペクトルとその作り方。
pub(super) struct SpeciesSpectrum {
    pub(super) energy_range_mev: (f64, f64),
    /// 積分フラックスを引いたエネルギーの数
    pub(super) integral_points: usize,
    /// 微分フラックスの点の数（粒子が無ければ 0）
    pub(super) differential_points: usize,
    /// 粒子が無い（正の微分が 2 点未満）ときは `None`
    pub(super) spectrum: Option<GridSpectrum>,
}

/// 粒子の範囲を 10 倍あたり [`POINTS_PER_DECADE`] 点で割った対数等間隔のエネルギー。
pub(super) fn integral_energies(range: (f64, f64)) -> Vec<f64> {
    let decades = (range.1 / range.0).ln() / std::f64::consts::LN_10;
    let n = (decades * POINTS_PER_DECADE).ceil() as usize + 1;
    let step = (range.1 / range.0).ln() / (n - 1) as f64;
    (0..n).map(|k| range.0 * (k as f64 * step).exp()).collect()
}

/// 粒子の範囲 [MeV]。
pub(super) fn range_of(species: Species) -> (f64, f64) {
    match species {
        Species::Proton => PROTON_RANGE_MEV,
        Species::Electron => ELECTRON_RANGE_MEV,
    }
}

/// 1 つのマップの軌道平均の積分フラックス `F(>E)` [cm⁻² s⁻¹]（`energies` ごと。L が求まらないサンプルは 0）。
pub(super) fn mean_integral_flux(
    map: &TrappedMap,
    positions: &[Result<BeltPosition, NoL>],
    t: &[f64],
    energies: &[f64],
) -> Vec<f64> {
    energies
        .iter()
        .map(|&e| {
            let flux: Vec<f64> = positions
                .iter()
                .map(|p| {
                    p.as_ref()
                        .map_or(0.0, |p| map.integral_flux(p.l, p.b_over_b0, e))
                })
                .collect();
            let mean = time_mean(t, &flux);
            if mean.is_finite() { mean } else { 0.0 }
        })
        .collect()
}

/// 軌道平均の積分スペクトルから、線量の計算へ渡すスペクトルを作る（微分のとり方は `differential_from_integral`）。
pub(super) fn species_spectrum(
    species: Species,
    map: &TrappedMap,
    positions: &[Result<BeltPosition, NoL>],
    t: &[f64],
) -> SpeciesSpectrum {
    let range = range_of(species);
    let energies = integral_energies(range);
    let integral = mean_integral_flux(map, positions, t, &energies);
    let (es, js) = differential_from_integral(&energies, &integral);
    let spectrum = match (es.first(), es.last()) {
        (Some(&lo), Some(&hi)) if es.len() >= 3 => EnergyGrid::new(lo, hi, INTEGRATION_POINTS)
            .ok()
            .and_then(|grid| GridSpectrum::from_table(grid, &es, &js).ok()),
        _ => None,
    };
    SpeciesSpectrum {
        energy_range_mev: range,
        integral_points: energies.len(),
        differential_points: if spectrum.is_some() { es.len() } else { 0 },
        spectrum,
    }
}
