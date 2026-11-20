//! 熱解析の検証（2）: 2 節点の結合（伝導のみ・放射のみ）と、非定常の熱量保存。
//! 記号: G 伝導 [W/K]、E 放射交換面積 [m²]（熱流 σE(T1⁴ − T2⁴)）、C 熱容量、Q 内部発熱、T_s = 0（宇宙の背景温度）。

mod common;

use common::*;
use sat_analysis::thermal::{ThermalConductiveLink, ThermalRadiativeLink};
use sat_astro::constants::SIGMA_SB;
use sat_astro::math::Vec3;
use sat_units::typed::Qty;

fn conduct(a: &str, b: &str, g: f64) -> ThermalConductiveLink {
    ThermalConductiveLink {
        a: a.to_owned(),
        b: b.to_owned(),
        conductance: Qty::from_raw(g),
    }
}

fn radiate(a: &str, b: &str, e: f64) -> ThermalRadiativeLink {
    ThermalRadiativeLink {
        a: a.to_owned(),
        b: b.to_owned(),
        exchange_area: Qty::from_raw(e),
    }
}

/// 熱を発する節点 "hot"（外面なし）と、宇宙へ放射する節点 "sink"（ε 0.8、0.02 m²、日陰）。
fn hot_and_sink(mass_hot: f64, mass_sink: f64) -> sat_analysis::thermal::ThermalNetwork {
    network(
        vec![al_node("hot", mass_hot), al_node("sink", mass_sink)],
        vec![surface("s", "sink", 0.02, -Vec3::X, 0.5, 0.8)],
    )
}

/// 時間の長さ [s]: 熱容量の合計を放射の線形化の伝導 4 ε σ A T³ で割った値の 30 倍（収束の目安。残りは e^-30 ≈ 1e-13 倍）。
fn settle_span(c_total: f64, eps_a: f64, t_k: f64) -> f64 {
    30.0 * c_total / (4.0 * eps_a * SIGMA_SB * t_k.powi(3))
}

/// 伝導のみ（"hot" に Q = 2 W、"sink" が宇宙へ放射、G = 0.5 W/K）:
/// T_sink⁴ = Q / (ε σ A)、T_hot − T_sink = Q / G = 4 K。定常（許容 1e-6 K）と非定常（30 τ 後、許容 1e-8 K。観測 4e-10 K。#297 は 20 τ 後 1e-5 K）。
#[test]
fn conduction_only_steady_and_transient() {
    let (q, g, eps_a) = (2.0, 0.5, 0.8 * 0.02);
    let t_sink = (q / (eps_a * SIGMA_SB)).powf(0.25);
    let mut net = hot_and_sink(0.005, 0.005);
    net.conductive = vec![conduct("hot", "sink", g)];
    let case = steady(
        net.clone(),
        env(SOLAR, 0.0),
        fixed_sun(false),
        vec![load("hot", q)],
    );
    let t = |i: usize| case.nodes[i].temperature.value();
    assert!((t(1) - t_sink).abs() < 1e-6, "{} vs {t_sink}", t(1));
    assert!(
        (t(0) - t(1) - q / g).abs() < 1e-6,
        "{} vs {}",
        t(0) - t(1),
        q / g
    );
    let span = settle_span(2.0 * 0.005 * CP_AL, eps_a, 200.0);
    let out = transient(
        net,
        timeline(span, 10.0, 75.0),
        env(0.0, 0.0),
        constant_phase(vec![load("hot", q)]),
        uniform(200.0),
    );
    let last = |k: &str| *values(&out.series, k).last().unwrap();
    assert!(
        (last("sink") - t_sink).abs() < 1e-8,
        "{} vs {t_sink}",
        last("sink")
    );
    assert!((last("hot") - last("sink") - q / g).abs() < 1e-8);
}

/// 一方を 300 K に固定した伝導（"hot" に Q = 2 W、G = 0.5 W/K）: 定常の T_hot = 300 + Q/G = 304 K（外面なし）。
#[test]
fn conduction_to_a_fixed_temperature_node() {
    let (q, g, t_fix) = (2.0, 0.5, 300.0);
    let mut net = network(
        vec![
            al_node("hot", 0.005),
            node_with("wall", 1.0, CP_AL, Some(t_fix)),
        ],
        vec![],
    );
    net.conductive = vec![conduct("hot", "wall", g)];
    let case = steady(net, env(SOLAR, 0.0), fixed_sun(false), vec![load("hot", q)]);
    assert!((case.nodes[0].temperature.value() - (t_fix + q / g)).abs() < 1e-9);
    assert_eq!(case.nodes[1].temperature.value(), t_fix);
}

/// 孤立した 2 つの質量の緩和（外面なし・入熱なし）: ΔT = T1 − T2 は τ = C1 C2 / (G (C1 + C2)) で e^(−t/τ) に減り、
/// 最終温度は熱容量で重み付けした平均 (C1 T1 + C2 T2)/(C1 + C2)。T1(t) = T_f + C2/(C1+C2)·ΔT(t)、T2(t) = T_f − C1/(C1+C2)·ΔT(t)。
/// C1 = 900 J/K（1 kg）、C2 = 450 J/K（0.5 kg）、G = 0.5 W/K → τ = 600 s。5 τ までを 30 s ごとに比べる。
/// 系は線形で、許容は 1e-8 K（観測 ≲ 1e-13 K。#297 の RK4 は 1e-6 K、観測 ≲ 3e-7 K）。熱容量で重み付けした温度の和の保存は 1e-6 K のまま。
#[test]
fn isolated_masses_relax_to_the_energy_weighted_mean() {
    let (m1, m2, g) = (1.0, 0.5, 0.5);
    let (c1, c2) = (m1 * CP_AL, m2 * CP_AL);
    let tau = c1 * c2 / (g * (c1 + c2));
    assert!((tau - 600.0).abs() < 1e-9);
    let mut net = network(vec![al_node("a", m1), al_node("b", m2)], vec![]);
    net.conductive = vec![conduct("a", "b", g)];
    let init = sat_analysis::thermal::transient::ThermalInitial::PerNode {
        temperatures: [("a".to_owned(), Qty::from_raw(350.0))].into(),
        default: Qty::from_raw(290.0),
    };
    let out = transient(
        net,
        timeline(5.0 * tau, 30.0, 75.0),
        env(0.0, 0.0),
        vec![],
        init,
    );
    let t_final = (c1 * 350.0 + c2 * 290.0) / (c1 + c2);
    let (a, b) = (values(&out.series, "a"), values(&out.series, "b"));
    for (i, t) in out.series.t.iter().enumerate() {
        let d = 60.0 * (-t / tau).exp();
        let (ea, eb) = (t_final + c2 / (c1 + c2) * d, t_final - c1 / (c1 + c2) * d);
        assert!(
            (a[i] - ea).abs() < 1e-8 && (b[i] - eb).abs() < 1e-8,
            "t={t}: {} {} vs {ea} {eb}",
            a[i],
            b[i]
        );
        // 熱容量で重み付けした温度の和は保存される
        assert!((c1 * a[i] + c2 * b[i] - (c1 * 350.0 + c2 * 290.0)).abs() < 1e-6);
    }
}

/// 放射のみ（"hot" に Q = 2 W、交換面積 E = 0.01 m²、"sink" が宇宙へ放射）:
/// Q = ε σ A T_sink⁴（宇宙へ）かつ Q = σ E (T_hot⁴ − T_sink⁴)（節点間）→ T_hot⁴ = T_sink⁴ + Q / (σ E)。
/// 定常（許容 1e-6 K）と非定常（30 τ 後、許容 1e-8 K。観測 4e-10 K。#297 は 20 τ 後 1e-5 K）。
#[test]
fn radiation_only_steady_and_transient() {
    let (q, e, eps_a) = (2.0, 0.01, 0.8 * 0.02);
    let t_sink = (q / (eps_a * SIGMA_SB)).powf(0.25);
    let t_hot = (t_sink.powi(4) + q / (SIGMA_SB * e)).powf(0.25);
    let mut net = hot_and_sink(0.005, 0.005);
    net.radiative = vec![radiate("hot", "sink", e)];
    let case = steady(
        net.clone(),
        env(SOLAR, 0.0),
        fixed_sun(false),
        vec![load("hot", q)],
    );
    assert!((case.nodes[1].temperature.value() - t_sink).abs() < 1e-6);
    assert!(
        (case.nodes[0].temperature.value() - t_hot).abs() < 1e-6,
        "{} vs {t_hot}",
        case.nodes[0].temperature.value()
    );
    let span = settle_span(0.005 * CP_AL, e.min(eps_a), t_hot.min(t_sink));
    let out = transient(
        net,
        timeline(span, 10.0, 75.0),
        env(0.0, 0.0),
        constant_phase(vec![load("hot", q)]),
        uniform(200.0),
    );
    let last = |k: &str| *values(&out.series, k).last().unwrap();
    assert!(
        (last("sink") - t_sink).abs() < 1e-8,
        "{} vs {t_sink}",
        last("sink")
    );
    assert!(
        (last("hot") - t_hot).abs() < 1e-8,
        "{} vs {t_hot}",
        last("hot")
    );
}

/// 放射のみ、一方を 350 K に固定: σ E (T1⁴ − T2⁴) = ε σ A T2⁴ → T2⁴ = E T1⁴ / (E + ε A)。
/// E = 0.01 m²、ε A = 0.016 m²。許容 1e-6 K（Newton 法の収束）。
#[test]
fn radiation_to_a_fixed_temperature_node() {
    let (e, eps_a, t1) = (0.01, 0.8 * 0.02, 350.0_f64);
    let mut net = network(
        vec![
            node_with("wall", 1.0, CP_AL, Some(t1)),
            al_node("sink", 0.005),
        ],
        vec![surface("s", "sink", 0.02, -Vec3::X, 0.5, 0.8)],
    );
    net.radiative = vec![radiate("wall", "sink", e)];
    let case = steady(net, env(SOLAR, 0.0), fixed_sun(false), vec![]);
    let expected = (e * t1.powi(4) / (e + eps_a)).powf(0.25);
    assert!((case.nodes[1].temperature.value() - expected).abs() < 1e-6);
}

/// 熱量保存（非定常）: ∫(Q_in − Q_rad) dt = Σ C (T_end − T_0)。
/// Q_in = 外部入熱（出力の `absorbed`）+ 内部発熱、Q_rad = ε σ A (T⁴ − T_s⁴)（T_s = 0）。公開の出力には放射の熱流が無いので、
/// 温度の系列から同じ式で求める（限界）。入熱は標本の間で連続（右辺の中で時刻ごとに求める）なので、入熱も放射も台形則。
/// 1 辺 0.1 m の立方体（面は 1 つが太陽向き、α 0.92・ε 0.86）に 3 W を加え、200 K から。
/// (a) 日照だけ（β 75°）600 s、標本の刻み 1 s: 入熱も放射も滑らかなのでシンプソン則で積分し、許容は ΔU の 1e-9 倍
///     （積分器の誤差だけ。#297 の RK4 + sample-and-hold では 1e-5、観測 5e-7）。
/// (b) 日陰のある軌道（β 0°）1 周回、標本の刻み 1 s: 日陰の出入り（半影の折れ曲がり）は標本の間で台形則と合わないので、
///     許容は |∫Q_in| の 1e-6 倍（観測 4.8e-7。#297 では刻み 5 s で 1e-5）。残りの差は積分器ではなく、標本から ∫Q_in を作る台形則の誤差。
#[test]
fn transient_conserves_energy() {
    let (alpha, eps, a, q_int) = (0.92, 0.86, 0.1, 3.0);
    let c = 0.05 * CP_AL;
    for (beta, span, step, rel) in [(75.0, 600.0, 1.0, 1e-9), (0.0, 5700.0, 1.0, 1e-6)] {
        let net = network(vec![al_node("n", 0.05)], cube_surfaces("n", a, alpha, eps));
        let out = transient(
            net,
            timeline(span, step, beta),
            env(SOLAR, 0.0),
            constant_phase(vec![load("n", q_int)]),
            uniform(200.0),
        );
        let (t, temp) = (&out.series.t, values(&out.series, "n"));
        let absorbed = values(&out.series, "absorbed");
        let q_total: Vec<f64> = absorbed.iter().map(|q| q + q_int).collect();
        // 日照だけなら入熱も滑らかなのでシンプソン則、日陰の出入りがあれば折れ曲がる（台形則）
        let q_in = if beta > 60.0 {
            simpson(t, &q_total)
        } else {
            trapezoid(t, &q_total)
        };
        let t4: Vec<f64> = temp.iter().map(|x| x.powi(4)).collect();
        let q_rad = eps * SIGMA_SB * 6.0 * a * a * simpson(t, &t4);
        let stored = c * (temp[temp.len() - 1] - temp[0]);
        let err = q_in - q_rad - stored;
        let scale = if beta > 60.0 {
            stored.abs()
        } else {
            q_in.abs()
        };
        assert!(
            err.abs() < rel * scale,
            "beta {beta}: in {q_in} - rad {q_rad} - stored {stored} = {err}"
        );
    }
}
