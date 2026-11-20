//! 非定常の初期温度の与え方（一様・節点ごと・軌道平均の定常解）。

use super::phases::Phases;
use super::{ThermalInitial, ThermalTransientInput};
use crate::spec::AnalysisError;
use crate::thermal::network::{Compiled, HeaterSpec, INITIAL_GUESS_K};
use crate::thermal::orbit_heat::mean_absorbed;
use sat_astro::timeline::TimelineSpec;

/// 初期温度 [K]（節点の順。固定温度の節点はその温度）。
pub(super) fn initial(
    net: &Compiled,
    input: &ThermalTransientInput,
    timeline: &TimelineSpec,
    phases: &Phases,
    heaters: &[HeaterSpec],
) -> Result<Vec<f64>, AnalysisError> {
    let mut t = match &input.initial {
        ThermalInitial::Uniform { temperature } => vec![temperature.value(); net.len()],
        ThermalInitial::PerNode {
            temperatures,
            default,
        } => {
            let mut t = vec![default.value(); net.len()];
            for (id, v) in temperatures {
                t[net.index(id)?] = v.value();
            }
            t
        }
        ThermalInitial::OrbitAverageSteady => {
            let ext = mean_absorbed(net, &input.environment, timeline)?;
            let q: Vec<f64> = ext.iter().zip(phases.mean()).map(|(a, b)| a + b).collect();
            let space_k = input.environment.space_temperature().value();
            // ヒータは最初の区間のサーモスタットで、下側の設定温度に保つ電力を出した定常解から始める
            let enabled = phases.enabled_at(0.0);
            net.solve_steady_heated(&q, space_k, INITIAL_GUESS_K, heaters, enabled)?
                .solution
                .temps
        }
    };
    if t.iter().any(|x| x.is_nan() || *x <= 0.0) {
        return Err(AnalysisError::InvalidInput(
            "Initial temperature must be positive".to_owned(),
        ));
    }
    for (ti, f) in t.iter_mut().zip(&net.fixed) {
        if let Some(v) = f {
            *ti = *v;
        }
    }
    Ok(t)
}
