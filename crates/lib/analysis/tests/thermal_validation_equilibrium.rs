//! 熱解析の検証（1）: 等温の 1 節点の物体（太陽光だけ・惑星なし）の平衡温度・放射冷却・1 次遅れ・表面材。
//! 期待値はすべて閉じた式。定常（`thermal.steady`）と非定常（`thermal.transient`）の両方で確かめる。
//! 式の記号: α 吸収率、ε 放射率、S 太陽光 [W/m²]、A_abs 太陽を向く吸収面積、A_rad 放射面積、C 熱容量 [J/K]、T_s 宇宙の背景温度。
//! 外面モデルの前提: 1 枚の外面は自分の面積で α を使って吸収し、同じ面積で ε を使って放射する（裏面は別の外面として足す）。

mod common;

use common::*;
use sat_analysis::thermal::ThermalNetwork;
use sat_astro::constants::SIGMA_SB;
use sat_astro::math::Vec3;

/// 熱容量 [J/K]（アルミ 5 g。時定数を数分にして、テストを短くする）。
const MASS_KG: f64 = 0.005;
const C: f64 = MASS_KG * CP_AL;

/// 線形化した時定数 τ = C / (4 ε σ A_rad T³)。
fn tau(eps: f64, a_rad: f64, t_k: f64) -> f64 {
    C / (4.0 * eps * SIGMA_SB * a_rad * t_k.powi(3))
}

/// 定常解が `expected` に一致し、非定常（200 K から、一定の入熱）が 30 τ 後に `expected` へ収束することを確かめる。
/// 30 τ 後の残りは e^-30 ≈ 1e-13 倍なので、収束の誤差は積分の誤差だけ。
fn assert_steady_and_transient(
    net: impl Fn() -> ThermalNetwork,
    space_k: f64,
    expected: f64,
    a_rad: f64,
    eps: f64,
) {
    let case = steady(net(), env(SOLAR, space_k), fixed_sun(true), vec![]);
    let t_steady = case.nodes[0].temperature.value();
    assert!(
        (t_steady - expected).abs() < 1e-6,
        "steady {t_steady} vs {expected}"
    );
    let span = 30.0 * tau(eps, a_rad, expected).max(tau(eps, a_rad, 200.0));
    let out = transient(
        net(),
        timeline(span, 10.0, 75.0),
        env(SOLAR, space_k),
        vec![],
        uniform(200.0),
    );
    let last = *values(&out.series, "n").last().unwrap();
    assert!(
        (last - expected).abs() < 1e-9,
        "transient {last} vs {expected} (span {span} s)"
    );
}

/// 平板（1 面だけが太陽を向き、他方は断熱）: T_eq⁴ = T_s⁴ + α S A / (ε σ A) = T_s⁴ + α S / (ε σ)。
/// 黒色塗装 α 0.92・ε 0.86 の 0.01 m²。T_s = 0（理想）と既定の 3 K の両方。
/// 許容: 定常は Newton 法の収束（1e-6 K）、非定常は適応刻み（既定の許容）で 1e-9 K（観測 ≲ 1e-13 K。#297 の RK4 は 1e-6 K）。
#[test]
fn flat_plate_one_face_equilibrium() {
    let (alpha, eps, a) = (0.92, 0.86, 0.01);
    for space_k in [0.0, 3.0] {
        let expected = t_eq(alpha, eps, a, a, space_k);
        let net = || {
            network(
                vec![al_node("n", MASS_KG)],
                vec![surface("front", "n", a, Vec3::X, alpha, eps)],
            )
        };
        assert_steady_and_transient(net, space_k, expected, a, eps);
    }
}

/// 平板の両面が放射する（表裏とも ε、裏は太陽に当たらない）: T_eq⁴ = α S A / (ε σ 2A)、断熱の面より √√2 倍だけ低い。
#[test]
fn flat_plate_both_faces_radiate() {
    let (alpha, eps, a) = (0.92, 0.86, 0.01);
    let expected = t_eq(alpha, eps, a, 2.0 * a, 0.0);
    let one_face = t_eq(alpha, eps, a, a, 0.0);
    assert!((one_face / expected - 2.0_f64.powf(0.25)).abs() < 1e-12);
    let net = || {
        network(
            vec![al_node("n", MASS_KG)],
            vec![
                surface("front", "n", a, Vec3::X, alpha, eps),
                surface("back", "n", a, -Vec3::X, alpha, eps),
            ],
        )
    };
    assert_steady_and_transient(net, 0.0, expected, 2.0 * a, eps);
}

/// 立方体（1 辺 0.1 m、1 面だけが太陽を向く）: A_abs = a²、A_rad = 6a² → T_eq⁴ = α S / (6 ε σ)。
/// 他の 5 面は太陽に対して cos ≤ 0（側面は 0 で吸収しない）。
#[test]
fn cube_one_face_to_the_sun() {
    let (alpha, eps, a) = (0.92, 0.86, 0.1);
    let expected = t_eq(alpha, eps, a * a, 6.0 * a * a, 0.0);
    let net = || {
        network(
            vec![al_node("n", MASS_KG)],
            cube_surfaces("n", a, alpha, eps),
        )
    };
    assert_steady_and_transient(net, 0.0, expected, 6.0 * a * a, eps);
}

/// 宇宙での放射冷却（入熱なし、T_s = 0）: C dT/dt = −ε σ A T⁴ → 1/T³ = 1/T0³ + 3 ε σ A t / C。
/// T0 = 400 K から 1200 s まで、10 s ごとのすべてのサンプルを比べる。許容 1e-8 K（観測 2e-10 K。#297 の RK4 は 1e-6 K、観測 4e-9 K）。
#[test]
fn radiative_cooling_in_deep_space() {
    let (eps, a, t0) = (0.86, 0.012, 400.0);
    let net = network(
        vec![al_node("n", MASS_KG)],
        vec![surface("f", "n", a, Vec3::X, 0.2, eps)],
    );
    let out = transient(
        net,
        timeline(1200.0, 10.0, 75.0),
        env(0.0, 0.0),
        vec![],
        uniform(t0),
    );
    let v = values(&out.series, "n");
    for (t, temp) in out.series.t.iter().zip(v) {
        let expected = (t0.powi(-3) + 3.0 * eps * SIGMA_SB * a * t / C).powf(-1.0 / 3.0);
        assert!(
            (temp - expected).abs() < 1e-8,
            "t={t}: {temp} vs {expected}"
        );
    }
}

/// 平衡の近くの 1 次応答: 線形化した τ = C / (4 ε σ A T_eq³)。平衡から ±1 K ずらして τ 後の偏差は e^-1 K。
/// 非線形性の補正は相対で約 3 δ/T ≈ 1 %（δ = 1 K, T ≈ 290 K）×偏差 0.37 K ≈ 0.004 K なので、許容 0.0015 K（観測 0.0009 K。#297 は 0.006 K）。
/// 符号を反転した 2 つの平均は 2 次の項が打ち消すので、その平均は 2e-5 K で一致する（観測 6e-6 K。#297 は 0.001 K）。
/// 非線形性を含めた厳密解との比較は `thermal_validation_integration.rs` の `nonlinear_radiative_relaxation_matches_the_exact_solution`（1e-8 K）。
#[test]
fn small_perturbation_decays_with_linear_time_constant() {
    let (alpha, eps, a) = (0.92, 0.86, 0.01);
    let t_star = t_eq(alpha, eps, a, a, 0.0);
    let tau_s = tau(eps, a, t_star);
    let run = |delta: f64| {
        let net = network(
            vec![al_node("n", MASS_KG)],
            vec![surface("f", "n", a, Vec3::X, alpha, eps)],
        );
        let out = transient(
            net,
            timeline(2.0 * tau_s, 1.0, 75.0),
            env(SOLAR, 0.0),
            vec![],
            uniform(t_star + delta),
        );
        let at_tau = out
            .series
            .t
            .iter()
            .position(|&t| t >= tau_s)
            .expect("series covers one time constant");
        // サンプルは 1 s ごと。τ ちょうどの偏差は、隣のサンプルから線形補間で求める
        let (t_a, t_b) = (out.series.t[at_tau - 1], out.series.t[at_tau]);
        let v = values(&out.series, "n");
        let w = (tau_s - t_a) / (t_b - t_a);
        (1.0 - w) * v[at_tau - 1] + w * v[at_tau] - t_star
    };
    let (up, down) = (run(1.0), run(-1.0));
    let expected = (-1.0_f64).exp();
    assert!((up - expected).abs() < 0.0015, "+1 K: {up} vs {expected}");
    assert!(
        (down + expected).abs() < 0.0015,
        "-1 K: {down} vs {}",
        -expected
    );
    assert!(((up - down) / 2.0 - expected).abs() < 2e-5);
}

/// 表面の性質: 同じ物体（1 面が太陽を向く立方体）を 白色塗装（α 0.2, ε 0.85）・黒色塗装（α 0.92, ε 0.86）・
/// MLI 相当（実効放射率 0.02、α 0.3）で包む。T_eq 比 = ((α1/ε1)/(α2/ε2))^¼。
/// MLI のモデル化: 外面 1 枚に「実効 ε 0.02・α 0.3」を一様な値として与える。実際のブランケットは外層が内部と
/// 熱的にほぼ切り離されて浮くので、ここでの値は実効放射率の考え方を借りた仮の値であり、比の検証にだけ使う。
/// 定常と非定常（各 30 τ）で比を比べる。許容は定常 1e-8、非定常 1e-11（比。観測 7e-15。#297 は非定常も 1e-8）。
#[test]
fn surface_finish_ratios() {
    let a = 0.1;
    let finishes = [
        ("white", 0.2, 0.85),
        ("black", 0.92, 0.86),
        ("mli_like", 0.3, 0.02),
    ];
    let temps: Vec<(f64, f64, f64)> = finishes
        .iter()
        .map(|&(_, alpha, eps)| {
            let net = || {
                network(
                    vec![al_node("n", MASS_KG)],
                    cube_surfaces("n", a, alpha, eps),
                )
            };
            let ts = steady(net(), env(SOLAR, 0.0), fixed_sun(true), vec![]).nodes[0]
                .temperature
                .value();
            let eq = t_eq(alpha, eps, a * a, 6.0 * a * a, 0.0);
            let span = 30.0 * tau(eps, 6.0 * a * a, eq).max(tau(eps, 6.0 * a * a, 200.0));
            let out = transient(
                net(),
                timeline(span, 10.0, 75.0),
                env(SOLAR, 0.0),
                vec![],
                uniform(200.0),
            );
            (ts, *values(&out.series, "n").last().unwrap(), eq)
        })
        .collect();
    for i in 0..finishes.len() {
        for j in 0..finishes.len() {
            let (ai, ei) = (finishes[i].1, finishes[i].2);
            let (aj, ej) = (finishes[j].1, finishes[j].2);
            let want = ((ai / ei) / (aj / ej)).powf(0.25);
            let steady_ratio = temps[i].0 / temps[j].0;
            let trans_ratio = temps[i].1 / temps[j].1;
            assert!(
                (steady_ratio - want).abs() < 1e-8,
                "{} / {}: steady {steady_ratio} vs {want}",
                finishes[i].0,
                finishes[j].0
            );
            assert!(
                (trans_ratio - want).abs() < 1e-11,
                "{} / {}: transient {trans_ratio} vs {want}",
                finishes[i].0,
                finishes[j].0
            );
        }
        assert!((temps[i].0 - temps[i].2).abs() < 1e-6);
    }
}

/// 混ぜた表面（白 70 % + 黒 30 %）は面積加重の α_eff = 0.7·0.2 + 0.3·0.92 = 0.416、ε_eff = 0.7·0.85 + 0.3·0.86 = 0.853 の
/// 一様な面と同じ温度になる（吸収も放射も面積に線形）。定常・非定常とも。
#[test]
fn mixed_surface_equals_area_weighted_properties() {
    let (a, a_eff, e_eff) = (0.1, 0.7 * 0.2 + 0.3 * 0.92, 0.7 * 0.85 + 0.3 * 0.86);
    let mixed = || {
        let mut s = cube_surfaces("n", a, 0.5, 0.5);
        s[0].absorptivity = None;
        s[0].emissivity = None;
        s[0].coatings = vec![
            coating("white", 0.7, 0.2, 0.85),
            coating("black", 0.3, 0.92, 0.86),
        ];
        // 他の 5 面は一様な ε_eff（吸収しないので α は効かない）
        for f in &mut s[1..] {
            f.absorptivity = Some(a_eff);
            f.emissivity = Some(e_eff);
        }
        network(vec![al_node("n", MASS_KG)], s)
    };
    let expected = t_eq(a_eff, e_eff, a * a, 6.0 * a * a, 0.0);
    assert_steady_and_transient(mixed, 0.0, expected, 6.0 * a * a, e_eff);
}

/// 高温まで上がる物体でも非定常が発散しないこと（#297 で見つかった不具合の再発防止）。固定刻みの RK4 で刻みを初期温度だけで決めていたとき、
/// 平衡温度が 400 K を大きく超える物体を粗い時刻刻みで走らせると、RK4 が不安定になって温度が −∞ に発散した
/// （α 0.3・ε 0.02 の平板、平衡 ≈ 775 K、サンプルの刻み 1000 s、`max_step` なし）。今は適応刻みで、平衡との差は 1e-8 K（#297 は 1 K）。
#[test]
fn transient_stays_stable_when_equilibrium_is_hot() {
    let (alpha, eps, a) = (0.3, 0.02, 0.01);
    let expected = t_eq(alpha, eps, a, a, 0.0);
    let net = network(
        vec![al_node("n", MASS_KG)],
        vec![surface("f", "n", a, Vec3::X, alpha, eps)],
    );
    let tl = timeline(60_000.0, 1000.0, 75.0);
    let out = transient(net, tl, env(SOLAR, 0.0), vec![], uniform(200.0));
    let last = *values(&out.series, "n").last().unwrap();
    assert!((last - expected).abs() < 1e-8, "{last} vs {expected}");
}
