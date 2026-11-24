//! 熱解析の検証（4）: 時間積分そのもの（適応刻み・日陰の境・ヒータの切り替えの時刻・標本の刻みへの依存・剛い系）。
//! 非定常は `sat_astro::ode::Dop853`（適応刻み）で、外部入熱は標本の間も連続に求め、日陰の境・ヒータの切り替えで積分を分ける。

mod common;

use common::*;
use sat_analysis::modes::Redundancy;
use sat_analysis::spec::Analysis;
use sat_analysis::thermal::network::ThermalHeater;
use sat_analysis::thermal::transient::ThermalInitial;
use sat_analysis::thermal::{
    ThermalConductiveLink, ThermalMarginPolicy, ThermalNetwork, ThermalTransient,
    ThermalTransientInput,
};
use sat_astro::constants::{R_EARTH, SIGMA_SB};
use sat_astro::ephem::{Body, CentralBody};
use sat_astro::math::{Vec3, bisect};
use sat_astro::shadow::ShadowModel;
use sat_astro::timeline::TimelineSpec;
use sat_units::typed::Qty;

/// 日陰のある軌道（β 0°）で、小さな板（α 0.92・ε 0.86、0.01 m²）が太陽を向く系。
fn plate_in_orbit(step_s: f64, span_s: f64, mass_kg: f64) -> (ThermalNetwork, TimelineSpec) {
    let net = network(
        vec![al_node("n", mass_kg)],
        vec![surface("front", "n", 0.01, Vec3::X, 0.92, 0.86)],
    );
    (net, timeline(span_s, step_s, 0.0))
}

fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).fold(0.0, |m, (x, y)| m.max((x - y).abs()))
}

/// 2 節点（0.5 kg と 5 g、熱容量が 100 倍違う）を許容誤差の組 `ladder` で走らせ、許容を半分にしたときの変化・
/// 厳しい許容（1e-14）の結果との差・刻みの数を調べる。`slack` は許容の大きさに対する許す倍率。
fn convergence(shadow: ShadowModel, slack: f64) -> (Vec<f64>, Vec<u32>) {
    let build = || {
        let mut net = network(
            vec![al_node("big", 0.5), al_node("small", 0.005)],
            vec![
                surface("a", "big", 0.01, Vec3::X, 0.92, 0.86),
                surface("b", "small", 0.001, Vec3::X, 0.3, 0.8),
            ],
        );
        net.conductive = vec![ThermalConductiveLink {
            a: "big".to_owned(),
            b: "small".to_owned(),
            conductance: Qty::from_raw(0.05),
        }];
        net
    };
    let period = timeline(1.0, 30.0, 0.0).orbit().cycle().length_s();
    let run = |rel: f64, abs: f64| {
        let mut tl = timeline(2.0 * period, 30.0, 0.0);
        tl.shadow = shadow;
        let out =
            transient_with_tolerance(build(), tl, env(SOLAR, 0.0), uniform(290.0), (rel, abs));
        let all: Vec<f64> = out
            .nodes
            .iter()
            .flat_map(|n| values(&out.series, &n.node).to_vec())
            .collect();
        (all, out.integration)
    };
    let (reference, _) = run(1e-14, 1e-12);
    let ladder = [
        (1e-6, 1e-4),
        (1e-7, 1e-5),
        (1e-8, 1e-6),
        (1e-9, 1e-7),
        (1e-10, 1e-8),
    ];
    let (mut errors, mut steps) = (Vec::new(), Vec::new());
    for (rel, abs) in ladder {
        let scale = abs + rel * 350.0;
        let (coarse, cost) = run(rel, abs);
        let (fine, _) = run(rel / 2.0, abs / 2.0);
        let halved = max_abs_diff(&coarse, &fine);
        assert!(
            halved < slack * scale,
            "{shadow:?} tol {rel}: halving changed {halved} K (scale {scale})"
        );
        let error = max_abs_diff(&coarse, &reference);
        assert!(
            error < 10.0 * slack * scale,
            "{shadow:?} tol {rel}: error {error} K (scale {scale})"
        );
        errors.push(error);
        steps.push(cost.accepted_steps);
    }
    (errors, steps)
}

/// 収束（円柱影: 日照が跳ぶ境は区間の端なので、区間の中は滑らか）: 許容誤差を半分にしたときの結果の変化は、許容の大きさ
/// （`abs + rel·T`）より小さく、全体の誤差は日陰の出入り（4 回）で積もっても許容の 10 倍の中に入る。
/// 区間（最長 30 s）が短いので、どの許容でも実際の誤差は許容よりずっと小さい（観測 6e-11 K。許容 1e-6 K 以上の組で）。
/// 許容を厳しくしても刻みは減らない（費用の報告が使える）。
#[test]
fn halving_the_tolerance_changes_the_result_by_less_than_the_tolerance() {
    let (errors, steps) = convergence(ShadowModel::Cylindrical, 1.0);
    assert!(errors.iter().all(|e| *e < 1e-9), "{errors:?}");
    assert!(steps.windows(2).all(|w| w[1] >= w[0]), "{steps:?}");
}

/// 円錐影（既定）は半影の入口と出口で日照率が x^{3/2} 型に立ち上がる（折れ曲がりの度合いが強い）ので、
/// 区間の端でそこを分けても、刻みの誤差推定が許容の数倍まで見逃す。許容の 3 倍の中に入ることを確かめる。
#[test]
fn conical_shadow_converges_within_a_few_tolerances() {
    let (errors, _) = convergence(ShadowModel::Conical, 3.0);
    assert!(errors[4] < errors[0] / 10.0, "{errors:?}");
}

/// 円柱影の食の出入りの時刻（幾何の定義どおり: 位置の太陽方向に垂直な成分 = 地球半径）。
fn cylinder_edge(tl: &TimelineSpec, t0: f64, t1: f64) -> f64 {
    let orbit = tl.resolved_orbit().unwrap();
    let perp = |t: f64| {
        let r = orbit.state_at(t).unwrap().r_eci;
        let s = CentralBody::Earth
            .relative_position_m(Body::Sun, orbit.epoch().plus_seconds(t))
            .normalized()
            .unwrap();
        (r - s * r.dot(s)).norm() - R_EARTH
    };
    bisect(perp, t0, t1, 1e-9, 200).expect("the interval brackets an edge")
}

/// 日陰の境: 太陽だけで温まる板（ε ≈ 0 なので放射せず、温度の傾き = 入熱 / 熱容量 が日照中は一定、日陰では 0）。
/// 食に入る標本の間隔の温度変化 ΔT = (Q/C)·(t_entry − t_k) から、温度が上がるのをやめた時刻を逆算し、幾何から求めた
/// 円柱影への入りの時刻と比べる（日陰を出る時刻も同様）。標本の刻みは 60 s だが、時刻は 2e-4 s（観測 4e-5 s。残りは ε = 1e-6 の放射）で一致する
/// （sample-and-hold なら、境は標本の時刻に量子化されて最大 60 s ずれる）。
#[test]
fn temperature_stops_rising_at_the_analytic_eclipse_entry_and_exit() {
    let (alpha, a, mass) = (0.9, 0.01, 0.5);
    let (step, c) = (60.0, mass * CP_AL);
    let net = network(
        vec![al_node("n", mass)],
        vec![surface("front", "n", a, Vec3::X, alpha, 1e-6)],
    );
    let mut tl = timeline(6900.0, step, 0.0);
    tl.shadow = ShadowModel::Cylindrical;
    let out = transient(net, tl.clone(), env(SOLAR, 0.0), vec![], uniform(290.0));
    let (t, v) = (&out.series.t, values(&out.series, "n"));
    let rate = alpha * SOLAR * a / c;
    let full = rate * step;
    // 食に入る標本の間隔: 温度変化が満額より小さくなった最初と、そのあと再び動き始めた最初
    let entry_k = (0..v.len() - 1)
        .find(|&k| v[k + 1] - v[k] < 0.999 * full)
        .expect("the orbit has an eclipse");
    let exit_k = (entry_k + 1..v.len() - 1)
        .find(|&k| v[k + 1] - v[k] > 1e-6)
        .expect("the eclipse ends");
    let entry = t[entry_k] + (v[entry_k + 1] - v[entry_k]) / rate;
    let exit = t[exit_k + 1] - (v[exit_k + 1] - v[exit_k]) / rate;
    let entry_exact = cylinder_edge(&tl, t[entry_k], t[entry_k + 1]);
    let exit_exact = cylinder_edge(&tl, t[exit_k], t[exit_k + 1]);
    assert!(
        (entry - entry_exact).abs() < 2e-4 && (exit - exit_exact).abs() < 2e-4,
        "entry {entry} vs {entry_exact}, exit {exit} vs {exit_exact}"
    );
    // 食の間は温度が動かない（入熱 0、放射は ε = 1e-6 で無視できる）
    assert!((v[exit_k] - v[entry_k + 1]).abs() < 1e-4);
}

/// 標本の刻みへの依存（利用者に見える効果）: 同じ系を標本の刻み 10 s と 120 s で走らせて、共通の時刻（120 s ごと）の温度は
/// 3e-5 K 以内で同じ（観測 8e-6 K。残りは半影の入口と出口の積分誤差）。日陰の出入り・入射角の変化は標本の間も連続に扱うので、刻みは出力の間隔でしかない
/// （sample-and-hold なら、120 s の刻みは入熱の段が最大 120 s 遅れて、板は数 K ずれる）。
#[test]
fn results_do_not_depend_on_the_sample_step() {
    let span = 8600.0;
    let run = |step: f64| {
        let (net, tl) = plate_in_orbit(step, span, 0.5);
        let out = transient(net, tl, env(SOLAR, 0.0), vec![], uniform(290.0));
        (out.series.t.clone(), values(&out.series, "n").to_vec())
    };
    let ((t_fine, v_fine), (t_coarse, v_coarse)) = (run(10.0), run(120.0));
    let mut compared = 0;
    let mut worst = 0.0_f64;
    for (tc, vc) in t_coarse.iter().zip(&v_coarse) {
        if let Some(i) = t_fine.iter().position(|t| (t - tc).abs() < 1e-9) {
            worst = worst.max((v_fine[i] - vc).abs());
            compared += 1;
        }
    }
    assert!(compared > 60, "{compared}");
    assert!(worst < 3e-5, "{worst}");
    // 板の温度は日陰の出入りで十分に動いている（検定が自明でないこと）
    let (lo, hi) = v_fine
        .iter()
        .fold((f64::MAX, f64::MIN), |(l, h), x| (l.min(*x), h.max(*x)));
    assert!(hi - lo > 30.0, "{lo} {hi}");
}

/// 非線形の放射緩和の厳密解: C dT/dt = ε σ A (T_e⁴ − T⁴) の解は、F(x) = atan x + ½ ln|(1 + x)/(1 − x)|（x = T/T_e）として
/// F(x(t)) = F(x(0)) + 2 T_e³ ε σ A t / C。平衡より下（100 K）と上（700 K）の両方から、各標本で F の残差を温度に直して比べる。
/// 許容 1e-8 K（#297 の線形化した 1 次遅れでは、非線形性のために 0.006 K までしか確かめられなかった）。
#[test]
fn nonlinear_radiative_relaxation_matches_the_exact_solution() {
    let (alpha, eps, a, mass) = (0.92, 0.86, 0.01, 0.005);
    let c = mass * CP_AL;
    let t_e = t_eq(alpha, eps, a, a, 0.0);
    let f = |x: f64| x.atan() + 0.5 * ((1.0 + x) / (1.0 - x)).abs().ln();
    for t0 in [100.0, 700.0] {
        let net = network(
            vec![al_node("n", mass)],
            vec![surface("front", "n", a, Vec3::X, alpha, eps)],
        );
        let out = transient(
            net,
            timeline(1500.0, 5.0, 75.0),
            env(SOLAR, 0.0),
            vec![],
            uniform(t0),
        );
        let mut worst = 0.0_f64;
        for (t, temp) in out.series.t.iter().zip(values(&out.series, "n")) {
            let x = temp / t_e;
            let residual = f(x) - f(t0 / t_e) - 2.0 * t_e.powi(3) * eps * SIGMA_SB * a * t / c;
            // dT/dF = T_e (1 − x⁴) / 2
            worst = worst.max((residual * t_e * (1.0 - x.powi(4)) / 2.0).abs());
        }
        assert!(worst < 1e-8, "T0 {t0}: {worst}");
    }
}

fn heater(on_below: f64, off_above: f64, power: f64) -> ThermalHeater {
    ThermalHeater {
        id: "H".to_owned(),
        node: "n".to_owned(),
        on_below: Qty::from_raw(on_below),
        off_above: Qty::from_raw(off_above),
        power: Qty::from_raw(power),
        count: 1,
        redundancy: Redundancy::None,
        states: Vec::new(),
    }
}

/// ヒータの切り替えは標本の刻みに量子化されない: 日陰のある軌道の板に、帯 [270, 285] K・5 W のヒータをつけて、
/// 標本の刻み 10 s と 120 s で走らせる。標本の温度はどちらも帯の下限を割らず（許容 1e-5 K）、
/// 切り替えの回数・デューティ・共通の時刻の温度（1e-4 K。観測 4e-6 K）が一致する。
#[test]
fn heater_switching_does_not_depend_on_the_sample_step() {
    let run = |step: f64| {
        let (net, tl) = plate_in_orbit(step, 11_000.0, 0.05);
        let out = ThermalTransient::run(&ThermalTransientInput {
            network: net,
            timeline: tl,
            environment: env(SOLAR, 0.0),
            phases: Vec::new(),
            heaters: vec![heater(270.0, 285.0, 5.0)],
            initial: ThermalInitial::Uniform {
                temperature: Qty::from_raw(290.0),
            },
            max_step: None,
            rel_tol: None,
            abs_tol: None,
            margin: ThermalMarginPolicy::default(),
        })
        .unwrap();
        let (t, v) = (out.series.t.clone(), values(&out.series, "n").to_vec());
        (t, v, out.heaters[0].clone(), out.integration)
    };
    let ((t_fine, v_fine, h_fine, _), (t_coarse, v_coarse, h_coarse, cost)) =
        (run(10.0), run(120.0));
    assert!(h_fine.switchings >= 4, "{}", h_fine.switchings);
    assert_eq!(h_fine.switchings, h_coarse.switchings);
    assert!((h_fine.duty_fraction.value() - h_coarse.duty_fraction.value()).abs() < 1e-8);
    assert!((h_fine.max_on_duration.value() - h_coarse.max_on_duration.value()).abs() < 1e-3);
    assert_eq!(cost.events, h_coarse.switchings);
    let mut worst = 0.0_f64;
    for (tc, vc) in t_coarse.iter().zip(&v_coarse) {
        if let Some(i) = t_fine.iter().position(|t| (t - tc).abs() < 1e-9) {
            worst = worst.max((v_fine[i] - vc).abs());
        }
    }
    assert!(worst < 1e-4, "{worst}");
    // 日陰で冷えても帯の下限（270 K）を割らない（5 W は 270 K の板の放射 2.6 W より大きい）。切り替えが標本の刻みに量子化されると、
    // 刻み 120 s の間の温度変化の分だけ割る
    for v in [&v_fine, &v_coarse] {
        let lo = v.iter().fold(f64::MAX, |l, x| l.min(*x));
        assert!(lo > 270.0 - 1e-5 && lo < 285.0, "{lo}");
    }
}

/// 剛い系（熱容量が 10⁴ 違う節点が強く結合した 12 節点の鎖）を `span_s` 走らせる: 熱容量 0.09 J/K の節点と 900 J/K の節点を交互に
/// G = 10 W/K でつなぐ。陰的な積分器は使わないので、刻みは最も速い節点の時定数（0.09/20 ≈ 4.5 ms）で決まる。
fn stiff_chain(
    span_s: f64,
) -> (
    sat_analysis::thermal::transient::ThermalTransientOutput,
    f64,
) {
    let n = 12;
    let nodes: Vec<_> = (0..n)
        .map(|i| al_node(&format!("n{i}"), if i % 2 == 0 { 1e-4 } else { 1.0 }))
        .collect();
    let surfaces = (0..n)
        .map(|i| surface(&format!("s{i}"), &format!("n{i}"), 0.005, Vec3::X, 0.9, 0.8))
        .collect();
    let mut net = network(nodes, surfaces);
    net.conductive = (0..n - 1)
        .map(|i| ThermalConductiveLink {
            a: format!("n{i}"),
            b: format!("n{}", i + 1),
            conductance: Qty::from_raw(10.0),
        })
        .collect();
    let started = std::time::Instant::now();
    let out = transient(
        net,
        timeline(span_s, 30.0, 0.0),
        env(SOLAR, 3.0),
        vec![],
        uniform(290.0),
    );
    let elapsed = started.elapsed().as_secs_f64();
    let cost = &out.integration;
    println!(
        "stiff chain, {span_s} s: {} accepted, {} rejected steps, {} rhs evaluations, {elapsed:.2} s",
        cost.accepted_steps, cost.rejected_steps, cost.rhs_evaluations
    );
    // 結合が強いので、隣り合う節点の温度差は小さい（熱の流れが抵抗で決まる）
    let last = |i: usize| *values(&out.series, &format!("n{i}")).last().unwrap();
    assert!((0..n - 1).all(|i| (last(i) - last(i + 1)).abs() < 5.0));
    (out, elapsed)
}

/// 剛い系が解ける（0.1 周回。刻みが自動で小さくなり、費用は出力の `integration` に出る）。
#[test]
fn stiff_chain_with_capacity_ratio_1e4_is_solved() {
    let (out, _) = stiff_chain(600.0);
    assert!(
        out.integration.accepted_steps > 10_000,
        "the fast node limits the step"
    );
}

/// 剛い系の 5 周回の費用の記録（`cargo test -- --ignored --nocapture`。デバッグビルドで約 1 分: 刻みは約 100 万）。
#[test]
#[ignore = "slow: records the cost of a stiff 5-orbit run"]
fn stiff_chain_five_orbits_cost() {
    let period = timeline(1.0, 30.0, 0.0).orbit().cycle().length_s();
    stiff_chain(5.0 * period);
}
