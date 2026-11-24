//! 熱解析の検証テスト（`thermal_validation_*.rs`）で共有する部品。公開 API だけで組み立てる
//! （`src/` の単体テストの補助は crate 内部のもので、統合テストからは見えない）。
//! 軌道は高度 550 km・傾斜角 97.6° の円軌道。姿勢は太陽指向（+X が太陽）なので、日照中の太陽方向は機体 +X に固定される。
#![allow(dead_code)]

use sat_analysis::modes::Redundancy;
use sat_analysis::spec::{Analysis, Series};
use sat_analysis::thermal::ThermalRanges;
use sat_analysis::thermal::network::{ThermalNode, ThermalNodePart};
use sat_analysis::thermal::steady::{ThermalSteadyCase, ThermalSteadyCaseResult};
use sat_analysis::thermal::transient::{
    ThermalHeatPhase, ThermalInitial, ThermalTransientInput, ThermalTransientOutput,
};
use sat_analysis::thermal::{
    ThermalCoating, ThermalEnvironment, ThermalExposure, ThermalLoad, ThermalMarginPolicy,
    ThermalNetwork, ThermalSteady, ThermalSteadyInput, ThermalSurface, ThermalTransient,
};
use sat_astro::attitude::AttitudeLaw;
use sat_astro::ephem::CentralBody;
use sat_astro::math::Vec3;
use sat_astro::orbit::{OrbitElements, OrbitSpec, Propagator, semi_major_axis_from_altitude};
use sat_astro::time::Epoch;
use sat_astro::timeline::TimelineSpec;
use sat_units::typed::{Fraction, Qty};

/// 太陽光の強さ [W/m²]（検証用の値。入力に明示する）
pub const SOLAR: f64 = 1361.0;
/// アルミ（教科書的な値）の比熱 [J/(kg·K)]
pub const CP_AL: f64 = 900.0;

/// 比熱 `cp` の部品 1 つだけの節点（熱容量 = `mass_kg`·`cp`）。
pub fn node_with(id: &str, mass_kg: f64, cp: f64, fixed: Option<f64>) -> ThermalNode {
    ThermalNode {
        id: id.to_owned(),
        parts: vec![ThermalNodePart {
            id: id.to_owned(),
            mass: Qty::from_raw(mass_kg),
            count: 1,
            specific_heat: Qty::from_raw(cp),
            ranges: ThermalRanges::default(),
            redundancy: Redundancy::None,
            states: Vec::new(),
        }],
        per_part: false,
        fixed_temperature: fixed.map(Qty::from_raw),
        required: Default::default(),
    }
}

/// アルミ（`CP_AL`）の節点。
pub fn al_node(id: &str, mass_kg: f64) -> ThermalNode {
    node_with(id, mass_kg, CP_AL, None)
}

/// 一様な α・ε の外面。
pub fn surface(
    id: &str,
    node: &str,
    area: f64,
    normal: Vec3,
    alpha: f64,
    eps: f64,
) -> ThermalSurface {
    ThermalSurface {
        id: id.to_owned(),
        node: node.to_owned(),
        area: Qty::from_raw(area),
        normal,
        count: 1,
        absorptivity: Some(alpha),
        emissivity: Some(eps),
        coatings: Vec::new(),
    }
}

/// 面積割合 `fraction` と (α, ε) の表面材。
pub fn coating(id: &str, fraction: f64, alpha: f64, eps: f64) -> ThermalCoating {
    ThermalCoating {
        id: id.to_owned(),
        area_fraction: fraction,
        absorptivity: alpha,
        emissivity: eps,
    }
}

/// 立方体（1 辺 `a` [m]）の外面 6 枚。+X の面が太陽を向き、残りの 5 面は太陽に対して cos ≤ 0 で吸収しない。
/// 6 面とも同じ (α, ε)。
pub fn cube_surfaces(node: &str, a: f64, alpha: f64, eps: f64) -> Vec<ThermalSurface> {
    [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z]
        .iter()
        .enumerate()
        .map(|(i, &n)| surface(&format!("f{i}"), node, a * a, n, alpha, eps))
        .collect()
}

/// 節点と外面だけの熱ネットワーク（結合なし）。
pub fn network(nodes: Vec<ThermalNode>, surfaces: Vec<ThermalSurface>) -> ThermalNetwork {
    ThermalNetwork {
        mounts: None,
        nodes,
        conductive: Vec::new(),
        radiative: Vec::new(),
        surfaces,
        enclosure: None,
    }
}

/// 惑星の入熱（アルベド・赤外）を 0 にし、太陽光 `solar` と宇宙の背景温度 `space_k` を明示した熱環境。
pub fn env(solar: f64, space_k: f64) -> ThermalEnvironment {
    ThermalEnvironment {
        solar_flux: Some(Qty::from_raw(solar)),
        albedo: Some(Fraction::new(0.0)),
        planet_ir: Some(Qty::from_raw(0.0)),
        space_temperature: Some(Qty::from_raw(space_k)),
        ..ThermalEnvironment::default()
    }
}

/// 太陽が機体 +X、中心天体は見ない向きの固定の入熱（`sun = false` なら日陰）。
pub fn fixed_sun(sun: bool) -> ThermalExposure {
    ThermalExposure::Fixed {
        sun_dir_body: sun.then_some(Vec3::X),
        nadir_body: None,
        altitude: Qty::from_raw(550.0e3),
        central: CentralBody::Earth,
    }
}

/// 内部発熱 `watts` [W]（すべて熱になる）。
pub fn load(node: &str, watts: f64) -> ThermalLoad {
    ThermalLoad {
        count: 1,
        node: node.to_owned(),
        power: Qty::from_raw(watts),
        rf_output: None,
        heat_fraction: Some(1.0),
    }
}

/// 一定の内部発熱の区間 1 つ（区間が 1 つなら長さは結果に効かない）。
pub fn constant_phase(loads: Vec<ThermalLoad>) -> Vec<ThermalHeatPhase> {
    vec![ThermalHeatPhase {
        label: "const".to_owned(),
        duration: None,
        loads,
        mode_loads: None,
    }]
}

/// 定常解析を 1 ケース走らせる。
pub fn steady(
    net: ThermalNetwork,
    environment: ThermalEnvironment,
    exposure: ThermalExposure,
    loads: Vec<ThermalLoad>,
) -> ThermalSteadyCaseResult {
    let input = ThermalSteadyInput {
        network: net,
        cases: vec![ThermalSteadyCase {
            id: "case".to_owned(),
            label: String::new(),
            environment,
            exposure,
            heaters: Vec::new(),
            loads,
            mode_loads: None,
        }],
        margin: ThermalMarginPolicy::default(),
    };
    ThermalSteady::run(&input).unwrap().cases.remove(0)
}

/// 高度 550 km・傾斜角 97.6° の円軌道で太陽指向（+X が太陽）の時系列。`beta_deg` を与えると β 角をその値にする
/// （75° なら 550 km では日陰が無い）。
pub fn timeline(span_s: f64, step_s: f64, beta_deg: f64) -> TimelineSpec {
    let elements = OrbitElements {
        semi_major_axis: Qty::from_raw(semi_major_axis_from_altitude(CentralBody::Earth, 550.0e3)),
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
    let attitude = AttitudeLaw::SunPointing {
        axis: Vec3::X,
        secondary: Vec3::Z,
    };
    let mut tl = TimelineSpec::new(
        orbit,
        attitude,
        Qty::from_raw(span_s),
        Qty::from_raw(step_s),
    );
    tl.sampling.beta = Some(Qty::from_raw(beta_deg.to_radians()));
    tl
}

/// 非定常解析を走らせる（積分の許容誤差と刻みは解析の既定）。
pub fn transient(
    net: ThermalNetwork,
    tl: TimelineSpec,
    environment: ThermalEnvironment,
    phases: Vec<ThermalHeatPhase>,
    initial: ThermalInitial,
) -> ThermalTransientOutput {
    ThermalTransient::run(&ThermalTransientInput {
        network: net,
        timeline: tl,
        environment,
        phases,
        heaters: Vec::new(),
        initial,
        max_step: None,
        rel_tol: None,
        abs_tol: None,
        margin: ThermalMarginPolicy::default(),
    })
    .unwrap()
}

/// すべての節点を同じ温度から始める。
pub fn uniform(t_k: f64) -> ThermalInitial {
    ThermalInitial::Uniform {
        temperature: Qty::from_raw(t_k),
    }
}

/// 系列の値（key で引く）。
pub fn values<'a>(s: &'a Series, key: &str) -> &'a [f64] {
    &s.channels
        .iter()
        .find(|c| c.key == key)
        .unwrap_or_else(|| panic!("no channel {key}"))
        .values
}

/// 台形則の ∫ v dt（`t` と同じ長さ）。
pub fn trapezoid(t: &[f64], v: &[f64]) -> f64 {
    t.windows(2)
        .zip(v.windows(2))
        .map(|(tw, vw)| 0.5 * (vw[0] + vw[1]) * (tw[1] - tw[0]))
        .sum()
}

/// 等間隔の標本の ∫ v dt（シンプソン則。標本の数が偶数なら最後の 1 区間だけ台形則）。滑らかな量の積分に使う。
pub fn simpson(t: &[f64], v: &[f64]) -> f64 {
    let h = t[1] - t[0];
    let n = v.len() - 1;
    let m = n - n % 2;
    let mut sum = v[0] + v[m];
    for (i, x) in v.iter().enumerate().take(m).skip(1) {
        sum += if i % 2 == 1 { 4.0 * x } else { 2.0 * x };
    }
    let tail = if n > m {
        0.5 * (v[n - 1] + v[n]) * h
    } else {
        0.0
    };
    sum * h / 3.0 + tail
}

/// 放射する面積に対する平衡温度の閉じた形: `T⁴ = T_s⁴ + α S A_abs / (ε σ A_rad)`。
pub fn t_eq(alpha: f64, eps: f64, a_abs: f64, a_rad: f64, space_k: f64) -> f64 {
    (space_k.powi(4) + alpha * SOLAR * a_abs / (eps * sat_astro::constants::SIGMA_SB * a_rad))
        .powf(0.25)
}

/// 許容誤差を指定した非定常解析（`rel_tol`: 相対、`abs_tol_k`: 絶対 [K]）。
pub fn transient_with_tolerance(
    net: ThermalNetwork,
    tl: TimelineSpec,
    environment: ThermalEnvironment,
    initial: ThermalInitial,
    (rel_tol, abs_tol_k): (f64, f64),
) -> ThermalTransientOutput {
    ThermalTransient::run(&ThermalTransientInput {
        network: net,
        timeline: tl,
        environment,
        phases: Vec::new(),
        heaters: Vec::new(),
        initial,
        max_step: None,
        rel_tol: Some(rel_tol),
        abs_tol: Some(Qty::from_raw(abs_tol_k)),
        margin: ThermalMarginPolicy::default(),
    })
    .unwrap()
}
