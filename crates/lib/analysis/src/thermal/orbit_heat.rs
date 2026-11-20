//! 外面への入熱（太陽・中心天体のアルベドと赤外放射）。`thermal.steady` と `thermal.transient` で共有する。
//!
//! 入射の幾何は `sat_astro::illumination`（太陽の cos 則と日照率、中心天体の形態係数、太陽光の強さ）を使い、
//! 軌道上の向きは `sat_astro::timeline` のサンプルから取る。ここでは吸収率・放射率を掛けて節点に配るだけ。
//! アルベドは「形態係数 × max(0, 衛星の位置と太陽方向の cos)」で近似する（概念設計の精度）。天体の赤外放射は
//! 同じ cos（直下点の太陽天頂角）で天体ごとのモデル（`thermal::planet`）から求める。環境の値は `thermal::environment`。

use crate::spec::{AnalysisError, orbit_mean, orbit_ranges};
use crate::thermal::environment::ResolvedEnvironment;
pub use crate::thermal::environment::{
    DEFAULT_SPACE_TEMPERATURE_K, ThermalEnvironment, ThermalEnvironmentUsed,
};
use crate::thermal::network::{Compiled, Surface};
use sat_astro::ephem::CentralBody;
use sat_astro::illumination::{body_view_factor, solar_flux_w_m2, solar_incidence};
use sat_astro::math::Vec3;
use sat_astro::orbit::semi_major_axis_from_altitude;
use sat_astro::timeline::{TimelineSample, TimelineSpec, build};
use sat_units::typed::{Length, Qty};
use serde::{Deserialize, Serialize};

/// 定常解析の入熱の与え方。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ThermalExposure {
    /// 時系列の入熱の平均（完全な周回があればその範囲で平均する。1 周回以上の span で軌道平均）
    /// β 角を指定するときは `timeline` の `beta`（`sat_astro::timeline` が RAAN を置き換える）。
    OrbitAverage { timeline: TimelineSpec },
    /// 向きを固定する（最悪の太陽方向など）。`sun_dir_body` が無ければ日陰、`nadir_body` が無ければ中心天体を見ない
    Fixed {
        #[serde(default)]
        sun_dir_body: Option<Vec3>,
        #[serde(default)]
        nadir_body: Option<Vec3>,
        /// 中心天体の形態係数とアルベドに使う高度
        altitude: Qty<Length>,
        /// 中心天体（既定は地球）。太陽光の強さを書かなければ天体の太陽距離の慣例値（1 AU）から求める
        #[serde(default)]
        central: CentralBody,
    },
}

impl ThermalExposure {
    /// 入熱の中心天体。
    pub fn central(&self) -> CentralBody {
        match self {
            Self::OrbitAverage { timeline } => timeline.orbit().central,
            Self::Fixed { central, .. } => *central,
        }
    }
}

/// ある時刻の入熱の幾何（すべて機体座標系）。
#[derive(Debug, Clone, Copy)]
pub(crate) struct Geometry {
    /// 太陽方向（単位ベクトル）
    pub sun_dir: Vec3,
    /// 日照率（1 = 全日照）
    pub sunlit: f64,
    /// 太陽光の強さ [W/m²]（環境の既定値に使う）
    pub solar_flux_w_m2: f64,
    /// 中心天体 → 衛星の位置 [m]。`None` なら中心天体を見ない
    pub r_m: Option<Vec3>,
}

impl Geometry {
    /// 時系列の 1 点から作る。
    pub fn from_sample(s: &TimelineSample) -> Self {
        Self {
            sun_dir: s.sun_dir_body(),
            sunlit: s.sunlit,
            solar_flux_w_m2: solar_flux_w_m2(s.sun_distance_m),
            r_m: Some(s.to_body(s.r_eci)),
        }
    }
}

/// 外面 1 枚の吸収熱 [W]。
fn surface_absorbed(s: &Surface, env: &ResolvedEnvironment, g: &Geometry) -> f64 {
    let flux = env.solar_flux_w_m2.unwrap_or(g.solar_flux_w_m2);
    let solar = s.alpha * flux * s.area_m2 * solar_incidence(s.normal, g.sun_dir, g.sunlit);
    let Some(r) = g.r_m else {
        return solar;
    };
    let f = body_view_factor(s.normal, r, env.body_radius_m);
    let cos_sun = r.normalized().map_or(0.0, |u| u.dot(g.sun_dir).max(0.0));
    let albedo = s.alpha * flux * env.albedo * f * cos_sun * s.area_m2;
    let ir = s.eps * env.ir.emitted_w_m2(env.albedo, flux, cos_sun) * f * s.area_m2;
    solar + albedo + ir
}

/// 節点ごとの外部入熱 [W]。
pub(crate) fn absorbed(net: &Compiled, env: &ResolvedEnvironment, g: &Geometry) -> Vec<f64> {
    let mut q = vec![0.0; net.len()];
    for s in &net.surfaces {
        q[s.node] += surface_absorbed(s, env, g);
    }
    q
}

/// 熱解析が扱える軌道か（中心天体が太陽だとアルベドも天体の赤外も無いので扱わない）。
pub(crate) fn require_planetary(spec: &TimelineSpec) -> Result<(), AnalysisError> {
    if spec.orbit().central == CentralBody::Sun {
        return Err(AnalysisError::InvalidInput(
            "Thermal analysis with the Sun as the central body is not supported (it has no albedo or planet IR form)"
                .to_owned(),
        ));
    }
    Ok(())
}

/// 時系列の各点の幾何。
pub(crate) fn timeline_geometry(
    spec: &TimelineSpec,
) -> Result<(Vec<f64>, Vec<Geometry>), AnalysisError> {
    require_planetary(spec)?;
    let samples = build(spec)?;
    if samples.is_empty() {
        return Err(AnalysisError::InvalidInput(
            "The time series is empty".to_owned(),
        ));
    }
    let t = samples.iter().map(|s| s.t_s).collect();
    Ok((t, samples.iter().map(Geometry::from_sample).collect()))
}

/// 時系列の入熱の周回平均 [W]（`crate::spec::orbit_mean`。完全な周回が無ければ全期間の平均）。
pub(crate) fn mean_absorbed(
    net: &Compiled,
    env: &ThermalEnvironment,
    spec: &TimelineSpec,
) -> Result<Vec<f64>, AnalysisError> {
    let env = env.resolve(spec.orbit().central)?;
    let (t, geo) = timeline_geometry(spec)?;
    let ranges = orbit_ranges(&t, spec.orbit().period_s());
    let per_sample: Vec<Vec<f64>> = geo.iter().map(|g| absorbed(net, &env, g)).collect();
    Ok((0..net.len())
        .map(|i| {
            let v: Vec<f64> = per_sample.iter().map(|q| q[i]).collect();
            orbit_mean(&t, &v, &ranges)
        })
        .collect())
}

/// 定常解析の入熱 [W]。
pub(crate) fn exposure_absorbed(
    net: &Compiled,
    env: &ThermalEnvironment,
    exposure: &ThermalExposure,
) -> Result<Vec<f64>, AnalysisError> {
    match exposure {
        ThermalExposure::OrbitAverage { timeline } => mean_absorbed(net, env, timeline),
        ThermalExposure::Fixed {
            sun_dir_body,
            nadir_body,
            altitude,
            central,
        } => {
            let env = env.resolve(*central)?;
            let flux = match (env.solar_flux_w_m2, central.nominal_sun_distance_m()) {
                (Some(f), _) => f,
                (None, Some(d)) => solar_flux_w_m2(d),
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
            Ok(absorbed(net, &env, &g))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thermal::network::{ThermalNetwork, ThermalSurface, tests::node};
    use crate::thermal::preset::ThermalPreset;
    use sat_astro::constants::{EARTH_IR, R_EARTH, R_MOON};

    fn plate(normal: Vec3) -> Compiled {
        Compiled::new(&ThermalNetwork {
            mounts: None,
            nodes: vec![node("p", None)],
            conductive: vec![],
            radiative: vec![],
            enclosure: None,
            surfaces: vec![ThermalSurface {
                id: "s".to_owned(),
                node: "p".to_owned(),
                area: Qty::from_raw(0.01),
                normal,
                count: 1,
                absorptivity: Some(0.9),
                emissivity: Some(0.8),
                coatings: vec![],
            }],
        })
        .unwrap()
    }

    fn fixed(
        sun: Option<Vec3>,
        nadir: Option<Vec3>,
        h: f64,
        central: CentralBody,
    ) -> ThermalExposure {
        ThermalExposure::Fixed {
            sun_dir_body: sun,
            nadir_body: nadir,
            altitude: Qty::from_raw(h),
            central,
        }
    }

    /// 太陽に正対した 0.01 m² の面（α = 0.9、1361 W/m²）は 12.25 W を吸収する。地球は見ない。
    #[test]
    fn sun_facing_plate_without_earth() {
        let net = plate(Vec3::X);
        let exposure = fixed(Some(Vec3::X), None, 500.0e3, CentralBody::Earth);
        let q = exposure_absorbed(&net, &ThermalEnvironment::default(), &exposure).unwrap();
        assert!((q[0] - 0.9 * 1361.0 * 0.01).abs() < 1e-9);
    }

    /// 高温のプリセットの太陽光 1414 W/m²: 太陽に正対した面（α 0.9、0.01 m²）は 0.9·1414·0.01 W を吸収する。
    #[test]
    fn preset_solar_flux_is_used() {
        let env = ThermalEnvironment {
            preset: Some(ThermalPreset::Hot),
            ..ThermalEnvironment::default()
        };
        let exposure = fixed(Some(Vec3::X), None, 500.0e3, CentralBody::Earth);
        let q = exposure_absorbed(&plate(Vec3::X), &env, &exposure).unwrap();
        assert!((q[0] - 0.9 * 1414.0 * 0.01).abs() < 1e-9);
    }

    /// 天底を向いた面は日陰でも地球赤外を受ける: ε·IR·A·sin²λ（sin λ = R/(R+h)）。
    #[test]
    fn nadir_plate_in_eclipse_gets_earth_ir() {
        let net = plate(Vec3::Z);
        let h = 500.0e3;
        let exposure = fixed(None, Some(Vec3::Z), h, CentralBody::Earth);
        let q = exposure_absorbed(&net, &ThermalEnvironment::default(), &exposure).unwrap();
        let f = (R_EARTH / (R_EARTH + h)).powi(2);
        assert!((q[0] - 0.8 * EARTH_IR * 0.01 * f).abs() < 1e-9, "{}", q[0]);
    }

    /// 月の高度 100 km で天底を向いた面（α 0.9、ε 0.8、0.01 m²）の手計算。F = (1737.4/1837.4)² ≈ 0.894。
    /// - 高温（直下点が正午、1414 W/m²）: アルベド 0.9·1414·0.12·F·A ≈ 1.365 W、赤外 0.8·(1 − 0.12)·1414·F·A ≈ 8.90 W、
    ///   合計 ≈ 10.27 W（直下点 ≈ 390 K の放射）。
    /// - 低温（夜側）: 赤外 0.8·(0.95 σ 100⁴)·F·A ≈ 0.0385 W だけ。
    #[test]
    fn lunar_nadir_plate_hot_and_cold() {
        let net = plate(Vec3::Z);
        let h = 100.0e3;
        let f = (R_MOON / (R_MOON + h)).powi(2);
        let hot = ThermalEnvironment {
            preset: Some(ThermalPreset::Hot),
            ..ThermalEnvironment::default()
        };
        let noon = fixed(Some(-Vec3::Z), Some(Vec3::Z), h, CentralBody::Moon);
        let q = exposure_absorbed(&net, &hot, &noon).unwrap()[0];
        let expected = 0.9 * 1414.0 * 0.12 * f * 0.01 + 0.8 * 0.88 * 1414.0 * f * 0.01;
        assert!(
            (q - expected).abs() < 1e-9 && (q - 10.27).abs() < 0.01,
            "{q}"
        );
        let night = fixed(None, Some(Vec3::Z), h, CentralBody::Moon);
        let cold = ThermalEnvironment {
            preset: Some(ThermalPreset::Cold),
            ..ThermalEnvironment::default()
        };
        let q = exposure_absorbed(&net, &cold, &night).unwrap()[0];
        assert!((q - 0.0385).abs() < 1e-3, "{q}");
        // 火星はアルベドと赤外を書かないと止まる
        let mars = fixed(None, Some(Vec3::Z), h, CentralBody::Mars);
        assert!(exposure_absorbed(&net, &ThermalEnvironment::default(), &mars).is_err());
    }

    /// `timeline` の `beta` を与えると元期の β 角がその値になり（RAAN の置き換えは `sat_astro::timeline`）、
    /// 慣性固定の立方体は β が大きい（日陰が短い）ほど平均入熱が多い。
    #[test]
    fn timeline_beta_is_honoured() {
        use super::fixtures::{cube, leo_timeline};
        let mut tl = leo_timeline(1.0, 30.0);
        tl.sampling.span = Qty::from_raw(tl.orbit().period_s());
        let net = Compiled::new(&cube(1.0)).unwrap();
        let env = ThermalEnvironment::default();
        let mean = |beta_deg: f64| {
            let mut spec = tl.clone();
            spec.sampling.beta = Some(Qty::from_raw(beta_deg.to_radians()));
            let orbit = spec.resolved_orbit().unwrap();
            let b = sat_astro::orbit::beta_angle_rad(&orbit, orbit.epoch).unwrap();
            assert!(
                (b.to_degrees() - beta_deg).abs() < 0.5,
                "{} vs {beta_deg}",
                b.to_degrees()
            );
            mean_absorbed(&net, &env, &spec).unwrap()[0]
        };
        assert!(mean(75.0) > mean(0.0));
    }
}

/// 熱のテストで共有する軌道と機体。
#[cfg(test)]
pub(crate) mod fixtures {
    use crate::thermal::network::{ThermalNetwork, ThermalSurface, tests::node};
    use sat_astro::attitude::AttitudeLaw;
    use sat_astro::math::{Quat, Vec3};
    use sat_astro::orbit::{OrbitElements, OrbitSpec, Propagator};
    use sat_astro::time::Epoch;
    use sat_astro::timeline::TimelineSpec;
    use sat_units::typed::Qty;

    /// 高度 550 km・傾斜角 97.6° の円軌道（日陰あり）で、慣性固定の姿勢。
    pub(crate) fn leo_timeline(span: f64, step: f64) -> TimelineSpec {
        let elements = OrbitElements {
            semi_major_axis: Qty::from_raw(sat_astro::orbit::semi_major_axis_from_altitude(
                sat_astro::ephem::CentralBody::Earth,
                550.0e3,
            )),
            eccentricity: 0.0,
            inclination: Qty::from_raw(97.6_f64.to_radians()),
            raan: Qty::from_raw(0.0),
            arg_perigee: Qty::from_raw(0.0),
            mean_anomaly: Qty::from_raw(0.0),
        };
        let orbit = OrbitSpec::new(
            Epoch::parse("2027-04-01T00:00:00Z").unwrap(),
            elements,
            Propagator::Kepler,
        );
        let attitude = AttitudeLaw::Inertial {
            q_body_to_eci: Quat::IDENTITY,
        };
        TimelineSpec::new(orbit, attitude, Qty::from_raw(span), Qty::from_raw(step))
    }

    /// 1 節点の立方体（6 面とも 0.01 m²、α = 0.6、ε = 0.8）。質量 `mass_kg`、比熱 900 J/(kg·K)。
    pub(crate) fn cube(mass_kg: f64) -> ThermalNetwork {
        let mut n = node("cube", None);
        n.parts[0].mass = Qty::from_raw(mass_kg);
        let faces = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z];
        ThermalNetwork {
            mounts: None,
            nodes: vec![n],
            conductive: vec![],
            radiative: vec![],
            enclosure: None,
            surfaces: faces
                .iter()
                .enumerate()
                .map(|(i, &normal)| ThermalSurface {
                    id: format!("f{i}"),
                    node: "cube".to_owned(),
                    area: Qty::from_raw(0.01),
                    normal,
                    count: 1,
                    absorptivity: Some(0.6),
                    emissivity: Some(0.8),
                    coatings: vec![],
                })
                .collect(),
        }
    }
}
