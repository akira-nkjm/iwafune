//! 放射線帯の捕捉粒子（AP-8 の陽子・AE-8 の電子）による、アルミの遮蔽の深さごとの全吸収線量率。
//!
//! **方法**: `orbit.trapped_flux` と同じ軌道の標本の位置（L, B/B₀）で、4 つの地図の積分フラックス `> E` を細かいエネルギー格子
//! （10 倍あたり 25 点、陽子 0.1〜400 MeV・電子 0.04〜7 MeV）で引いて時間平均し（台形則）、隣り合う点の差分で微分フラックスにして
//! （`sat_astro::radiation::shielding::differential_from_integral`）、SHIELDOSE-2（Seltzer, NISTIR 5477, 1994）で
//! アルミの遮蔽の深さごとの線量にする。太陽活動の極小（AP8MIN + AE8MIN）と極大（AP8MAX + AE8MAX）の 2 通りで、陽子・電子・電子の制動放射・合計を出す。
//!
//! **既定の取り方（出力の `used` に書く）**: 検出器はシリコン、幾何はアルミの**中実の球の中心**（全方向から入射。SHIELDOSE-2 が球に対して出す値は
//! 片面の板と同じ規格化の「半分の線量」なので、実際の線量はその 2 倍。`sat_astro::radiation::shielding` の説明）、
//! 陽子の核相互作用は見ない（保守側）。厚さは [`DEFAULT_THICKNESSES_MM`]。
//!
//! **制限**: 捕捉粒子だけ（太陽陽子・銀河宇宙線・地球の影の電子の変化は含まない）。静的なモデル。遮蔽はアルミの単純な幾何で、
//! 実機の形・部品の配置・二次粒子の輸送は見ない。データ（地図と SHIELDOSE-2 の表）は配布データで、無ければ入力不足。

use super::trapped_flux::{earth_samples, positions_in};
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, ConditionTopic, ensure_positive,
};
use sat_astro::geomag::{FieldSnapshot, NoL};
use sat_astro::radiation::shielding::{
    Detector, GeometryDoses, ShieldingData, ShieldingModel, aluminium_depth_g_cm2,
};
use sat_astro::radiation::{
    BeltConvention, BeltField, BeltPosition, RadiationTables, SolarCondition, Species, TrappedModel,
};
use sat_astro::timeline::OrbitPoint;
use sat_units::typed::{Length, Qty};
use spectrum::{SpeciesSpectrum, species_spectrum};
use std::collections::HashMap;

mod spectrum;
mod types;

pub use spectrum::{ELECTRON_RANGE_MEV, INTEGRATION_POINTS, POINTS_PER_DECADE, PROTON_RANGE_MEV};
pub use types::{
    DoseGeometry, DoseSettingSource, NuclearAttenuation, TrappedDoseCase, TrappedDoseInput,
    TrappedDoseOutput, TrappedDosePoint, TrappedDoseSpectrumUsed, TrappedDoseUsed,
};

/// 遮蔽の厚さ（等価アルミ）の既定 [mm]。**既定値はここだけ**（出力の `used.thickness_source` に書く）。
pub const DEFAULT_THICKNESSES_MM: [f64; 13] = [
    0.1, 0.2, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 20.0,
];

/// 捕捉粒子による遮蔽の深さごとの線量率。
pub struct TrappedDose;

impl Analysis for TrappedDose {
    const ID: &'static str = "orbit.trapped_dose";
    const TITLE: &'static str = "捕捉粒子の線量と遮蔽の深さ";
    const TITLE_EN: &'static str = "Trapped particle dose versus shielding depth";
    const AREA: AnalysisArea = AnalysisArea::Condition(ConditionTopic::Orbit);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "AP-8（陽子）・AE-8（電子）の軌道平均のスペクトルから、アルミの遮蔽の深さごとの線量率を SHIELDOSE-2 で求める（太陽活動の極小・極大。捕捉粒子だけ）";
    const DESCRIPTION_EN: &'static str = "Dose rate in silicon versus aluminium shielding depth from the orbit-averaged AP-8 and AE-8 spectra with SHIELDOSE-2, for solar minimum and maximum (trapped particles only)";
    type Input = TrappedDoseInput;
    type Output = TrappedDoseOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let shielding = sat_astro::radiation::registry::shielding().ok_or_else(|| {
            AnalysisError::InvalidInput("The SHIELDOSE-2 dose data is not installed".into())
        })?;
        compute(input, &RadiationTables::installed(), shielding)
    }
}

/// 厚さの入力の検査と、SHIELDOSE-2 に渡す深さ [g/cm²]。
fn depths(
    input: &TrappedDoseInput,
) -> Result<(Vec<Qty<Length>>, DoseSettingSource), AnalysisError> {
    let (list, source) = match &input.thicknesses {
        Some(v) => (v.clone(), DoseSettingSource::Given),
        None => (
            DEFAULT_THICKNESSES_MM
                .iter()
                .map(|mm| Qty::from_raw(mm * 1.0e-3))
                .collect(),
            DoseSettingSource::Default,
        ),
    };
    if list.len() < 3 {
        return Err(AnalysisError::InvalidInput(
            "Shielding thicknesses: at least 3 values are needed".into(),
        ));
    }
    for t in &list {
        ensure_positive("shielding thickness", t.value())?;
    }
    if list.windows(2).any(|w| w[1].value() <= w[0].value()) {
        return Err(AnalysisError::InvalidInput(
            "Shielding thicknesses must be strictly increasing".into(),
        ));
    }
    Ok((list, source))
}

/// 引く地図・場・線量の表を引数に取る本体（試験は合成のものを渡す）。
pub fn compute(
    input: &TrappedDoseInput,
    tables: &RadiationTables<'_>,
    shielding: &ShieldingData,
) -> Result<TrappedDoseOutput, AnalysisError> {
    let missing = tables.missing();
    if !missing.is_empty() {
        return Err(AnalysisError::InvalidInput(format!(
            "The AP-8/AE-8 model tables are not installed (missing: {})",
            missing.join(", ")
        )));
    }
    let (thicknesses, thickness_source) = depths(input)?;
    let points = earth_samples(&input.sampling)?;
    let t: Vec<f64> = points.iter().map(|p| p.t_s).collect();
    let positions = belt_positions(tables, &points);
    let geometry = input.geometry.unwrap_or(DoseGeometry::SphereCentre);
    let nuclear = input.nuclear.unwrap_or(NuclearAttenuation::None);
    let model = ShieldingModel::new(shielding, Detector::Silicon, nuclear_option(nuclear))?;
    let depth_g_cm2: Vec<f64> = thicknesses
        .iter()
        .map(|l| aluminium_depth_g_cm2(l.value()))
        .collect();
    let mut cases = Vec::with_capacity(2);
    let mut spectra = Vec::new();
    for solar in [SolarCondition::Minimum, SolarCondition::Maximum] {
        let mut parts: Vec<Vec<f64>> = Vec::new();
        for species in [Species::Proton, Species::Electron] {
            let which = TrappedModel::of(species, solar);
            let (Some(map), Some(pos)) = (tables.map(which), positions.get(&which.field())) else {
                continue;
            };
            let sp = species_spectrum(species, map, pos, &t);
            spectra.push(spectrum_used(species, solar, &sp));
            parts.extend(doses_of(&model, species, &sp, &depth_g_cm2, geometry)?);
        }
        cases.push(case_of(solar, &thicknesses, &parts));
    }
    let samples_without_l = max_samples_without_l(&positions);
    Ok(TrappedDoseOutput {
        cases,
        used: TrappedDoseUsed {
            span: input.sampling.span,
            step: input.sampling.step,
            samples: points.len(),
            samples_without_l,
            geometry,
            geometry_source: source_of(input.geometry.is_some()),
            detector: Detector::Silicon.name().to_owned(),
            nuclear,
            nuclear_source: source_of(input.nuclear.is_some()),
            thickness_source,
            integration_points: INTEGRATION_POINTS,
            spectra,
            scope: SCOPE.to_owned(),
        },
    })
}

/// 磁場ごとの標本の (L, B/B₀)（地図ごとに組にする場で求める。1 つの場につき 1 度だけ）。
fn belt_positions(
    tables: &RadiationTables<'_>,
    points: &[OrbitPoint],
) -> HashMap<BeltField, Vec<Result<BeltPosition, NoL>>> {
    BeltField::ALL
        .into_iter()
        .filter_map(|id| {
            let field = tables.field(id)?;
            let snapshot: FieldSnapshot = field.at(field.epoch());
            Some((
                id,
                positions_in(&snapshot, points, BeltConvention::default()),
            ))
        })
        .collect()
}

/// L が求まらない標本の数（磁場ごとの最大）。
fn max_samples_without_l(positions: &HashMap<BeltField, Vec<Result<BeltPosition, NoL>>>) -> usize {
    positions
        .values()
        .map(|p| p.iter().filter(|x| x.is_err()).count())
        .max()
        .unwrap_or(0)
}

/// 何の線量か（出力の `used.scope`）。
const SCOPE: &str = "Trapped protons (AP-8), trapped electrons (AE-8) and electron bremsstrahlung only, in silicon behind aluminium shielding; no solar energetic protons, no galactic cosmic rays, no secondary particles from the spacecraft structure beyond the SHIELDOSE-2 model";

fn source_of(given: bool) -> DoseSettingSource {
    if given {
        DoseSettingSource::Given
    } else {
        DoseSettingSource::Default
    }
}

fn nuclear_option(n: NuclearAttenuation) -> sat_astro::radiation::shielding::NuclearOption {
    use sat_astro::radiation::shielding::NuclearOption as N;
    match n {
        NuclearAttenuation::None => N::None,
        NuclearAttenuation::Local => N::Local,
        NuclearAttenuation::LocalWithNeutrons => N::LocalWithNeutrons,
    }
}

fn spectrum_used(
    species: Species,
    solar: SolarCondition,
    s: &SpeciesSpectrum,
) -> TrappedDoseSpectrumUsed {
    TrappedDoseSpectrumUsed {
        species,
        solar,
        energy_min_mev: s.energy_range_mev.0,
        energy_max_mev: s.energy_range_mev.1,
        integral_points: s.integral_points,
        differential_points: s.differential_points,
    }
}

/// 選んだ幾何の線量 [Gy/s]。
fn pick(d: &GeometryDoses, geometry: DoseGeometry) -> Vec<f64> {
    match geometry {
        DoseGeometry::SphereCentre => d.sphere_centre(),
        DoseGeometry::SemiInfiniteSlab => d.semi_infinite.clone(),
        DoseGeometry::FiniteSlab => d.finite_slab.clone(),
    }
}

/// 粒子 1 種の、深さごとの線量の並び（陽子は 1 つ、電子は電子と制動放射の 2 つ）。粒子が無ければ 0。
fn doses_of(
    model: &ShieldingModel,
    species: Species,
    sp: &SpeciesSpectrum,
    depths_g_cm2: &[f64],
    geometry: DoseGeometry,
) -> Result<Vec<Vec<f64>>, AnalysisError> {
    let zero = vec![0.0; depths_g_cm2.len()];
    Ok(match (species, &sp.spectrum) {
        (Species::Proton, Some(s)) => {
            vec![pick(&model.proton_dose(s, depths_g_cm2)?, geometry)]
        }
        (Species::Electron, Some(s)) => {
            let d = model.electron_dose(s, depths_g_cm2)?;
            vec![
                pick(&d.electrons, geometry),
                pick(&d.bremsstrahlung, geometry),
            ]
        }
        (Species::Proton, None) => vec![zero],
        (Species::Electron, None) => vec![zero.clone(), zero],
    })
}

/// `parts` は [陽子, 電子, 制動放射]（欠けていれば 0）。
fn case_of(
    solar: SolarCondition,
    thicknesses: &[Qty<Length>],
    parts: &[Vec<f64>],
) -> TrappedDoseCase {
    let at = |k: usize, i: usize| parts.get(k).and_then(|v| v.get(i)).copied().unwrap_or(0.0);
    let table = thicknesses
        .iter()
        .enumerate()
        .map(|(i, &shielding)| {
            let (p, e, b) = (at(0, i), at(1, i), at(2, i));
            TrappedDosePoint {
                shielding,
                protons: Qty::from_raw(p),
                electrons: Qty::from_raw(e),
                bremsstrahlung: Qty::from_raw(b),
                total: Qty::from_raw(p + e + b),
            }
        })
        .collect();
    let (proton, electron) = (
        TrappedModel::of(Species::Proton, solar),
        TrappedModel::of(Species::Electron, solar),
    );
    TrappedDoseCase {
        solar,
        proton_model: proton.id().to_owned(),
        electron_model: electron.id().to_owned(),
        table,
    }
}

#[cfg(test)]
mod tests;
