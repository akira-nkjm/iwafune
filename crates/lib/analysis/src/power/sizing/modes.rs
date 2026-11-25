//! モードごとの逆算。入射は姿勢則の時系列（`power.orbit_generation`）から取り、電力収支と前提を揃える。
use super::calculate::{ModeLoads, smad_requirements};
use super::{SizingInput, SizingMode, SizingTimeSeries, SizingUsed};
use crate::power::orbit_generation::{OrbitGeneration, OrbitGenerationInput};
use crate::power::solar::{SolarArray, SolarLife};
use crate::spec::{Analysis, AnalysisError};
use sat_astro::attitude::AttitudeLaw;
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
            "EOL generation per unit area is not positive".into(),
        ));
    }
    // 同じ姿勢則のモードは同じ時系列・同じ入力なので、日照中の平均発電は 1 回だけ求める
    let mut averages: Vec<(&AttitudeLaw, f64)> = Vec::new();
    loads
        .iter()
        .map(|l| {
            let law = input
                .mode_laws
                .get(&l.id)
                .unwrap_or(&input.timeline.attitude);
            let average = match averages.iter().find(|(known, _)| *known == law) {
                Some((_, average)) => *average,
                None => {
                    let average = sunlit_average(input, array, law)?;
                    averages.push((law, average));
                    average
                }
            };
            let k = average / reference;
            if k <= 0.0 {
                return Err(AnalysisError::InvalidInput(format!(
                    "The attitude of mode {} generates no power in sunlight",
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
                series_area: None,
                series_capacity: None,
                area_ratio: None,
                capacity_ratio: None,
            })
        })
        .collect()
}

/// 姿勢則 `law` で回した日照中の平均発電（EOL）[W]（モードの姿勢則が無ければ時系列の姿勢則）。
fn sunlit_average(
    input: &SizingInput,
    array: &SolarArray,
    law: &AttitudeLaw,
) -> Result<f64, AnalysisError> {
    let mut timeline = input.timeline.clone();
    timeline.attitude = law.clone();
    let out = OrbitGeneration::run(&OrbitGenerationInput {
        timeline,
        array: array.clone(),
        life: Some(SolarLife::Eol),
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

/// 時系列の逆算の値と、SMAD の値との比をモードごとに入れる。
pub(super) fn attach_series(modes: &mut [SizingMode], ts: &SizingTimeSeries) {
    let ratio = |series: f64, smad: f64| {
        let r = series / smad;
        r.is_finite().then_some(Fraction::new(r))
    };
    for m in modes {
        let Some(t) = ts.targets.iter().find(|t| t.id == m.id) else {
            continue;
        };
        m.series_area = Some(t.required_area);
        m.series_capacity = Some(t.required_capacity);
        m.area_ratio = ratio(t.required_area.value(), m.required_area.value());
        m.capacity_ratio = ratio(t.required_capacity.value(), m.required_capacity.value());
    }
}
