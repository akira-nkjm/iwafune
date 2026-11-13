//! モードごとの逆算。入射は姿勢則の時系列（`power.orbit_generation`）から取り、電力収支と前提を揃える。
use super::calculate::{ModeLoads, smad_requirements};
use super::{SizingInput, SizingMode, SizingUsed};
use crate::power::orbit_generation::{OrbitGeneration, OrbitGenerationInput};
use crate::power::solar::{SolarArray, SolarLife};
use crate::spec::{Analysis, AnalysisError};
use sat_units::typed::{Fraction, Qty};

/// 対象モードごとに入射係数・必要発電・面積・容量を求める。
pub(super) fn evaluate(
    input: &SizingInput,
    array: &SolarArray,
    used: &SizingUsed,
    loads: &[ModeLoads],
    td: f64,
    te: f64,
) -> Result<Vec<SizingMode>, AnalysisError> {
    let area: f64 = array.panels.iter().map(|p| p.area.value()).sum();
    let reference = array
        .normal_incidence_power(array.flux(None), SolarLife::Eol)
        .value();
    let normal_density = reference / area * used.life_factor * used.inherent_degradation;
    if !(normal_density.is_finite() && normal_density > 0.0) {
        return Err(AnalysisError::InvalidInput(
            "EOL の単位面積あたり発電が正ではありません".into(),
        ));
    }
    loads
        .iter()
        .map(|l| {
            let k = sunlit_average(input, array, &l.id)? / reference;
            if k <= 0.0 {
                return Err(AnalysisError::InvalidInput(format!(
                    "モード {} の姿勢では日照中の発電がありません",
                    l.id
                )));
            }
            let (generation, area, capacity) =
                smad_requirements(l.sunlit, l.eclipse, td, te, used, normal_density * k)?;
            Ok(SizingMode {
                id: l.id.clone(),
                sunlit_load: Qty::from_raw(l.sunlit),
                eclipse_load: Qty::from_raw(l.eclipse),
                incidence_factor: Fraction::new(k),
                required_generation: Qty::from_raw(generation),
                required_area: Qty::from_raw(area),
                required_capacity: Qty::from_raw(capacity),
            })
        })
        .collect()
}

/// モード `mode` の姿勢則で回した日照中の平均発電（EOL）[W]。
fn sunlit_average(
    input: &SizingInput,
    array: &SolarArray,
    mode: &str,
) -> Result<f64, AnalysisError> {
    let mut timeline = input.timeline.clone();
    if let Some(law) = input.mode_laws.get(mode) {
        timeline.attitude = law.clone();
    }
    let out = OrbitGeneration::run(&OrbitGenerationInput {
        timeline,
        array: array.clone(),
        life: SolarLife::Eol,
    })?;
    Ok(out.sunlit_average.value())
}

/// `key` が最大のモード（同じ値なら先のもの）。`modes` は空でない。
pub(super) fn governing(modes: &[SizingMode], key: impl Fn(&SizingMode) -> f64) -> &SizingMode {
    modes.iter().skip(1).fold(
        &modes[0],
        |best, m| if key(m) > key(best) { m } else { best },
    )
}
