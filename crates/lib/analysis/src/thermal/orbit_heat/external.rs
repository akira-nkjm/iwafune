//! 外部入熱（節点ごと）の、時系列の周回平均と定常解析の固定の向き。影を数えたときの面の日照割合の報告もここで組む。
//! 1 点の入熱の計算は親の `absorbed`。

use super::{
    Geometry, ThermalEnvironment, ThermalExposure, absorbed, absorbed_with, timeline_geometry,
};
use crate::spec::{
    AnalysisError, Check, SUNLIT_THRESHOLD, Verdict, orbit_mean, orbit_ranges, quantity_text,
};
use crate::structure::external_geometry::{ShadowLoss, ShadowingReport};
use crate::thermal::environment::ResolvedEnvironment;
use crate::thermal::network::Compiled;
use sat_astro::math::Vec3;
use sat_astro::orbit::semi_major_axis_from_altitude;
use sat_astro::timeline::{TimelineSample, TimelineSpec};
use sat_units::typed::{Fraction, Power, Qty};

/// 節点ごとの外部入熱 [W] と、影を数えたときの面の日照割合の報告。
pub(crate) struct ExternalHeat {
    pub q: Vec<f64>,
    pub shadowing: Option<ShadowingReport>,
}

/// 影を落とす部品が外面を持たないときの警告を出す、影で遮られた日射の割合（影を数えた入熱の合計に対する）の下限。**この値はここだけ。**
pub const SHADOW_LOSS_WARNING_FRACTION: f64 = 0.02;

/// 面の日照割合（影）の報告: 日照中の標本の太陽方向について、面ごとの最小と平均、影を落とす部品、
/// 外面を持たない部品が遮って吸収されない日射（影を数えない入熱の合計 − 数えた入熱の合計。`mean` で標本を 1 つの値にする）。
/// 網に部品の箱が無ければ `None`。
pub(crate) fn shadowing_report(
    net: &Compiled,
    env: &ResolvedEnvironment,
    geo: &[Geometry],
    mean: &dyn Fn(&[f64]) -> f64,
) -> Option<ShadowingReport> {
    let shading = net.shading.as_ref()?;
    let sunward: Vec<Vec3> = geo
        .iter()
        .filter(|g| g.sunlit >= SUNLIT_THRESHOLD)
        .map(|g| g.sun_dir)
        .collect();
    let mut report = shading.summary(&sunward);
    let total = |shaded: bool| -> Vec<f64> {
        geo.iter()
            .map(|g| absorbed_with(net, env, g, shaded).iter().sum())
            .collect()
    };
    let (with, without) = (mean(&total(true)), mean(&total(false)));
    let parts: Vec<String> = report
        .casters
        .iter()
        .filter(|id| !shading.has_surface(id))
        .cloned()
        .collect();
    report.lost = Some(ShadowLoss {
        parts,
        power: Qty::from_raw(without - with),
        fraction: Fraction::new(if with > 0.0 {
            (without - with) / with
        } else {
            0.0
        }),
    });
    Some(report)
}

/// 外面を持たない部品が遮って吸収されない日射が、入熱の合計に対して [`SHADOW_LOSS_WARNING_FRACTION`] を超えるときの警告。
pub(crate) fn shadow_loss_check(report: &ShadowingReport, key: &str) -> Option<Check> {
    let lost = report.lost.as_ref()?;
    if lost.parts.is_empty() || lost.fraction.value() <= SHADOW_LOSS_WARNING_FRACTION {
        return None;
    }
    Some(Check::new(
        key,
        "Sunlight intercepted by parts without an outer surface",
        Verdict::Warning,
        format!(
            "Shadows remove {} of direct sunlight ({:.1} % of the absorbed heat; warning above {:.0} %), and the shading parts {} have no outer surface, so that heat is absorbed nowhere. Give them an outer surface (area, normal, absorptivity, emissivity) or lump them into a surface",
            quantity_text::<Power>(lost.power.value()),
            100.0 * lost.fraction.value(),
            100.0 * SHADOW_LOSS_WARNING_FRACTION,
            lost.parts.join(", "),
        ),
    ))
}

/// 時系列の標本（日照中のもの）の太陽方向についての、面の日照割合の報告（影の損失は標本の単純平均）。
pub(crate) fn sample_shadowing(
    net: &Compiled,
    env: &ResolvedEnvironment,
    samples: &[TimelineSample],
) -> Option<ShadowingReport> {
    let geometry: Vec<Geometry> = samples.iter().map(Geometry::from_sample).collect();
    shadowing_report(net, env, &geometry, &|v| {
        if v.is_empty() {
            0.0
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    })
}

/// 時系列の入熱の周回平均 [W]（`crate::spec::orbit_mean`。完全な周回が無ければ全期間の平均）と、影の報告。
pub(crate) fn mean_heat(
    net: &Compiled,
    env: &ThermalEnvironment,
    spec: &TimelineSpec,
) -> Result<ExternalHeat, AnalysisError> {
    let env = env.resolve(spec.orbit().central())?;
    let (t, geo) = timeline_geometry(spec)?;
    let ranges = orbit_ranges(&t, spec.orbit().cycle());
    let per_sample: Vec<Vec<f64>> = geo.iter().map(|g| absorbed(net, &env, g)).collect();
    let q = (0..net.len())
        .map(|i| {
            let v: Vec<f64> = per_sample.iter().map(|q| q[i]).collect();
            orbit_mean(&t, &v, &ranges)
        })
        .collect();
    let mean = |v: &[f64]| orbit_mean(&t, v, &ranges);
    Ok(ExternalHeat {
        q,
        shadowing: shadowing_report(net, &env, &geo, &mean),
    })
}

/// 時系列の入熱の周回平均 [W]（[`mean_heat`] の入熱だけ）。
pub(crate) fn mean_absorbed(
    net: &Compiled,
    env: &ThermalEnvironment,
    spec: &TimelineSpec,
) -> Result<Vec<f64>, AnalysisError> {
    Ok(mean_heat(net, env, spec)?.q)
}

/// 定常解析の入熱 [W] と、影の報告。
pub(crate) fn exposure_heat(
    net: &Compiled,
    env: &ThermalEnvironment,
    exposure: &ThermalExposure,
) -> Result<ExternalHeat, AnalysisError> {
    match exposure {
        ThermalExposure::OrbitAverage { timeline } => mean_heat(net, env, timeline),
        ThermalExposure::Fixed {
            sun_dir_body,
            nadir_body,
            altitude,
            central,
        } => {
            let env = env.resolve(*central)?;
            let flux = match (env.solar_flux_w_m2, central.nominal_solar_flux_w_m2()) {
                (Some(f), _) | (None, Some(f)) => f,
                (None, None) => {
                    return Err(AnalysisError::InvalidInput(format!(
                        "No conventional Sun distance for central body {central:?}. Set solar_flux"
                    )));
                }
            };
            let r = nadir_body
                .and_then(Vec3::normalized)
                .map(|n| -n * semi_major_axis_from_altitude(*central, altitude.value()));
            let g = Geometry {
                sun_dir: sun_dir_body
                    .and_then(Vec3::normalized)
                    .unwrap_or(Vec3::ZERO),
                sunlit: if sun_dir_body.is_some() { 1.0 } else { 0.0 },
                solar_flux_w_m2: flux,
                r_m: r,
            };
            Ok(ExternalHeat {
                q: absorbed(net, &env, &g),
                shadowing: shadowing_report(net, &env, &[g], &|v| {
                    v.first().copied().unwrap_or(0.0)
                }),
            })
        }
    }
}
