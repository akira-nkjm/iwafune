//! 熱解析の検証（3）: 日陰のある軌道での定常と非定常の整合、惑星の赤外・アルベドの形態係数。

mod common;

use common::*;
use sat_analysis::spec::Analysis;
use sat_analysis::thermal::steady::ThermalSteadyCase;
use sat_analysis::thermal::transient::ThermalInitial;
use sat_analysis::thermal::{
    ThermalExposure, ThermalMarginPolicy, ThermalSteady, ThermalSteadyInput,
};
use sat_astro::constants::{R_EARTH, SIGMA_SB};
use sat_astro::ephem::CentralBody;
use sat_astro::math::Vec3;
use sat_units::typed::{Fraction, Qty};

/// 1 つの周期解で成り立つ恒等式: 温度が周期的なら 1 周回の ∫C dT = 0 なので、
/// 平均の入熱 = ε σ A (⟨T⁴⟩ − T_s⁴)（放射は T⁴ に線形）。一方、軌道平均の入熱での定常解は
/// T_ss⁴ = T_s⁴ + ⟨Q⟩/(ε σ A)。したがって **熱容量によらず ⟨T⁴⟩^¼ = T_ss**（T_s = 0 では ⟨T⁴⟩ = T_ss⁴）。
/// 帰結: T⁴ の平均が T_ss⁴ に等しいので、T_ss は周期解の最低と最高の間にある。
/// 熱容量が小さい極限（τ ≪ 周期）では T⁴ が入熱に追従するので ⟨T⁴⟩^¼ が T_ss に近く、単純な平均 ⟨T⟩ は Jensen の不等式で T_ss より
/// 小さい。熱容量が大きい極限（τ ≫ 周期）では T がほぼ一定なので ⟨T⟩ も T_ss に近い。
///
/// 検証: 太陽を向く平板（1 面、α 0.92・ε 0.86、0.02 m²）、β 0°（日陰 約 36 分 / 周期 約 96 分）、T_s = 0、
/// 5 周回のうち最後の 1 周回（初期値は軌道平均の定常解。小さい熱容量は 1 周回で周期解に入り、大きい熱容量は初期値がほぼ周期解）。
/// - 両方: T_ss ∈ [min, max]、(⟨T⁴⟩)^¼ = T_ss（許容 0.1 K、観測 0.04 K。入熱が標本の刻み 5 s の段で、定常側の平均は台形則という違いと、
///   初期の過渡の残りによる）。
/// - 小さい熱容量（5 g、τ ≈ 数分）: ⟨T⟩ は T_ss より 3 K 以上低い（Jensen。この差は小さい熱容量で現れる）。
/// - 大きい熱容量（20 kg、τ ≈ 数日）: ⟨T⟩ が T_ss と 0.1 K 以内（観測 0.024 K）、最高と最低の差が 2.5 K 未満（基本波の振幅 ≈ 平板の吸収の基本波 14.7 W / (C ω) ≈ 0.75 K、差は約 2 倍に高調波を加えた 約 1.9 K）。
#[test]
fn steady_matches_the_fourth_power_mean_of_the_periodic_transient() {
    let (alpha, eps, a) = (0.92, 0.86, 0.02);
    for (mass, small) in [(0.005, true), (20.0, false)] {
        let net = || {
            network(
                vec![al_node("n", mass)],
                vec![surface("f", "n", a, Vec3::X, alpha, eps)],
            )
        };
        let tl0 = timeline(1.0, 5.0, 0.0);
        let period = tl0.orbit().period_s();
        let tl = timeline(5.0 * period, 5.0, 0.0);
        let case = ThermalSteadyCase {
            id: "avg".to_owned(),
            label: String::new(),
            environment: env(SOLAR, 0.0),
            heaters: Vec::new(),
            exposure: ThermalExposure::OrbitAverage {
                timeline: tl.clone(),
            },
            loads: vec![],
            mode_loads: None,
        };
        let steady = ThermalSteady::run(&ThermalSteadyInput {
            network: net(),
            cases: vec![case],
            margin: ThermalMarginPolicy::default(),
        })
        .unwrap();
        let t_ss = steady.cases[0].nodes[0].temperature.value();
        let out = transient(
            net(),
            tl,
            env(SOLAR, 0.0),
            vec![],
            ThermalInitial::OrbitAverageSteady,
            5.0,
        );
        let (t, temp) = (&out.series.t, values(&out.series, "n"));
        let from = t.iter().position(|&x| x >= 4.0 * period).unwrap();
        let (tl_t, tl_v) = (&t[from..], &temp[from..]);
        let span = tl_t[tl_t.len() - 1] - tl_t[0];
        let mean = trapezoid(tl_t, tl_v) / span;
        let t4: Vec<f64> = tl_v.iter().map(|x| x.powi(4)).collect();
        let mean4 = (trapezoid(tl_t, &t4) / span).powf(0.25);
        let (lo, hi) = tl_v
            .iter()
            .fold((f64::MAX, f64::MIN), |(l, h), &x| (l.min(x), h.max(x)));
        assert!(lo < t_ss && t_ss < hi, "mass {mass}: {lo} < {t_ss} < {hi}");
        assert!(
            (mean4 - t_ss).abs() < 0.1,
            "mass {mass}: <T^4>^(1/4) {mean4} vs steady {t_ss}"
        );
        if small {
            assert!(
                t_ss - mean > 3.0,
                "mass {mass}: <T> {mean} vs steady {t_ss}"
            );
        } else {
            assert!(
                (mean - t_ss).abs() < 0.1,
                "mass {mass}: <T> {mean} vs steady {t_ss}"
            );
            assert!(hi - lo < 2.5, "mass {mass}: swing {}", hi - lo);
        }
    }
}

/// 天底を向く平板（面は 1 枚、α 0.92・ε 0.86、0.01 m²）が惑星から受ける入熱（高度 500 km）。形態係数 F = (R/(R+h))²（法線が天底）。
/// - 日陰（太陽なし）: 吸収 = ε F q_IR A（q_IR = 237 W/m²）。
/// - 直下点が正午（太陽が天底の反対側から来て、面は太陽に背を向ける）: アルベド α S ρ F A（ρ = 0.3、cos θ = 1）を加える。
/// - 裏向き（法線が天頂、日陰）: F = 0 で吸収 0。
///
/// 定常温度は同じ面から放射するので T⁴ = T_s⁴ + 吸収 / (ε σ A)（T_s = 3 K の既定値）。許容: 吸収は 1e-9 W、温度は 1e-6 K。
#[test]
fn nadir_plate_planet_ir_and_albedo() {
    let (alpha, eps, a, h, q_ir, rho) = (0.92, 0.86, 0.01, 500.0e3, 237.0, 0.3);
    let f = (R_EARTH / (R_EARTH + h)).powi(2);
    let environment = |solar: f64| {
        let mut e = env(solar, 3.0);
        e.albedo = Some(Fraction::new(rho));
        e.planet_ir = Some(Qty::from_raw(q_ir));
        e
    };
    let exposure = |sun: bool| ThermalExposure::Fixed {
        sun_dir_body: sun.then_some(-Vec3::Z),
        nadir_body: Some(Vec3::Z),
        altitude: Qty::from_raw(h),
        central: CentralBody::Earth,
    };
    let plate = |normal: Vec3| {
        network(
            vec![al_node("n", 0.01)],
            vec![surface("f", "n", a, normal, alpha, eps)],
        )
    };
    let cases = [
        (Vec3::Z, false, eps * f * q_ir * a),
        (
            Vec3::Z,
            true,
            eps * f * q_ir * a + alpha * SOLAR * rho * f * a,
        ),
        (-Vec3::Z, false, 0.0),
    ];
    for (normal, sun, expected) in cases {
        let case = steady(plate(normal), environment(SOLAR), exposure(sun), vec![]);
        let node = &case.nodes[0];
        assert!(
            (node.absorbed.value() - expected).abs() < 1e-9,
            "sun {sun}: {} vs {expected}",
            node.absorbed.value()
        );
        if expected > 0.0 {
            let t = (3.0_f64.powi(4) + expected / (eps * SIGMA_SB * a)).powf(0.25);
            assert!(
                (node.temperature.value() - t).abs() < 1e-6,
                "{} vs {t}",
                node.temperature.value()
            );
        }
    }
}
