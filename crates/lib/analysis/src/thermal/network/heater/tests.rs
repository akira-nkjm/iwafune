//! ヒータの制御則・モードでの有効/無効・検査と、ヒータありの定常解（手計算）のテスト。

use super::super::tests::node;
use super::super::{ThermalConductiveLink, ThermalNetwork, ThermalSurface};
use super::*;
use crate::modes::StateAssignment;
use sat_astro::constants::SIGMA_SB;
use sat_astro::math::Vec3;
use sat_units::typed::Qty;

fn heater(id: &str, node: &str, lo: f64, hi: f64, power: f64) -> ThermalHeater {
    ThermalHeater {
        id: id.to_owned(),
        node: node.to_owned(),
        on_below: Qty::from_raw(lo),
        off_above: Qty::from_raw(hi),
        power: Qty::from_raw(power),
        count: 1,
        redundancy: Redundancy::None,
        states: Vec::new(),
    }
}

/// 面積 `area_m2`・放射率 0.8 で宇宙へ放射する節点。
fn radiating(id: &str, area_m2: f64) -> (super::super::ThermalNode, ThermalSurface) {
    (
        node(id, None),
        ThermalSurface {
            id: format!("s_{id}"),
            node: id.to_owned(),
            area: Qty::from_raw(area_m2),
            normal: Vec3::X,
            count: 1,
            absorptivity: Some(0.5),
            emissivity: Some(0.8),
            coatings: vec![],
        },
    )
}

fn network(parts: &[(&str, f64)], links: &[(&str, &str, f64)]) -> Compiled {
    let (nodes, surfaces): (Vec<_>, Vec<_>) = parts.iter().map(|(id, a)| radiating(id, *a)).unzip();
    Compiled::new(&ThermalNetwork {
        mounts: None,
        nodes,
        conductive: links
            .iter()
            .map(|(a, b, g)| ThermalConductiveLink {
                a: (*a).to_owned(),
                b: (*b).to_owned(),
                conductance: Qty::from_raw(*g),
            })
            .collect(),
        radiative: vec![],
        enclosure: None,
        surfaces,
        shadowing: None,
    })
    .unwrap()
}

/// 放射で宇宙（0 K）へ捨てる電力 ε σ A T⁴ [W]（ε = 0.8）。
fn radiated(area_m2: f64, t: f64) -> f64 {
    0.8 * SIGMA_SB * area_m2 * t.powi(4)
}

fn solve(
    net: &Compiled,
    q: &[f64],
    heaters: &[ThermalHeater],
) -> super::super::heater_steady::HeatedSolution {
    let specs = net.heaters(heaters).unwrap();
    let enabled = vec![true; heaters.len()];
    net.solve_steady_heated(q, 0.0, 290.0, &specs, &enabled)
        .unwrap()
}

/// 制御則: T < on_below でオン、T > off_above でオフ、帯の中は直前の状態（ヒステリシス）。初期は帯の中ならオフ。
#[test]
fn thermostat_law_has_hysteresis() {
    let net = network(&[("n", 0.1)], &[]);
    let h = &net.heaters(&[heater("H", "n", 280.0, 285.0, 5.0)]).unwrap()[0];
    assert!(h.next_state(false, 279.9, 0.0));
    assert!(h.next_state(true, 279.9, 0.0));
    assert!(!h.next_state(true, 285.1, 0.0));
    assert!(!h.next_state(false, 285.1, 0.0));
    // 帯の中は保つ
    assert!(h.next_state(true, 282.0, 0.0));
    assert!(!h.next_state(false, 282.0, 0.0));
    // 初期
    assert!(h.initial_state(270.0));
    assert!(!h.initial_state(282.0));
    assert!(!h.initial_state(300.0));
}

/// 切り替えまでの余裕と、許容つきの次の状態: オンなら `off_above` まで、オフなら `on_below` から。
/// 許容 `tol` 以内か越えていれば切り替える（時間積分が設定温度に届く時刻を求めたとき、その直前の側の値でも切り替えるため）。
#[test]
fn margin_and_next_state_with_tolerance() {
    let net = network(&[("n", 0.1)], &[]);
    let h = &net.heaters(&[heater("H", "n", 280.0, 285.0, 5.0)]).unwrap()[0];
    assert_eq!(h.margin(true, 283.0), 2.0);
    assert_eq!(h.margin(false, 283.0), 3.0);
    // 設定温度の手前 1e-7 K: 許容 1e-5 なら届いたとして切り替え、許容 0 なら切り替えない
    assert!(!h.next_state(true, 285.0 - 1e-7, 1e-5));
    assert!(h.next_state(true, 285.0 - 1e-7, 0.0));
    assert!(h.next_state(false, 280.0 + 1e-7, 1e-5));
    assert!(!h.next_state(false, 280.0 + 1e-7, 0.0));
    // 切り替えた直後は反対側の設定温度までの余裕が帯の幅だけあるので、続けて切り替わらない
    assert_eq!(h.margin(false, 285.0), 5.0);
    assert_eq!(h.margin(true, 280.0), 5.0);
}

/// モードでの有効/無効: 停止状態だけなら無効、`off` に `on` の duty があるか `on` なら有効、割り当てが無い・モードが無ければ有効。
#[test]
fn enabled_by_mode_state() {
    let mode = |state: &str, duty: &[(&str, f64)]| ModeDef {
        id: "M".into(),
        label: "M".into(),
        assignments: vec![StateAssignment {
            part: "H".into(),
            state: state.into(),
            duty: duty.iter().map(|(s, f)| ((*s).into(), *f)).collect(),
            active: None,
            spare_state: None,
        }],
        eclipse: Vec::new(),
    };
    let h = heater("H", "n", 280.0, 285.0, 5.0);
    assert!(!h.enabled_in(Some(&mode("off", &[]))).unwrap());
    assert!(h.enabled_in(Some(&mode("on", &[]))).unwrap());
    assert!(h.enabled_in(Some(&mode("off", &[("on", 0.2)]))).unwrap());
    assert!(!h.enabled_in(Some(&mode("off", &[("on", 0.0)]))).unwrap());
    assert!(h.enabled_in(None).unwrap());
    let other = ModeDef {
        assignments: Vec::new(),
        ..mode("off", &[])
    };
    assert!(h.enabled_in(Some(&other)).unwrap());
}

#[test]
fn invalid_heaters_are_rejected() {
    let net = network(&[("a", 0.1), ("b", 0.1)], &[("a", "b", 1.0)]);
    let ok = |h: ThermalHeater| net.heaters(&[h]).is_ok();
    assert!(ok(heater("H", "a", 280.0, 285.0, 5.0)));
    assert!(!ok(heater("H", "a", 285.0, 280.0, 5.0)));
    assert!(!ok(heater("H", "a", 280.0, 280.0, 5.0)));
    // 帯の幅は下限（0.001 K）以上
    assert!(!ok(heater("H", "a", 280.0, 280.0005, 5.0)));
    assert!(ok(heater("H", "a", 280.0, 280.002, 5.0)));
    assert!(!ok(heater("H", "a", 280.0, 285.0, 0.0)));
    assert!(!ok(heater("H", "missing", 280.0, 285.0, 5.0)));
    assert!(
        net.heaters(&[
            heater("H1", "a", 280.0, 285.0, 5.0),
            heater("H2", "a", 270.0, 275.0, 5.0)
        ])
        .is_err()
    );
}

/// 1 節点（面積 0.1 m²・ε 0.8 で宇宙 0 K へ放射、他の発熱 Q = 3 W）の設定温度 T_set = 280 K。
/// 保つのに要る電力 P = ε σ A (T_set⁴ − 0⁴) − Q ≈ 24.88 W。
/// 容量 30 W なら制御中で P がそれに等しく、節点は 280 K。容量 10 W なら飽和で T = ((10 + 3)/(ε σ A))^¼。
/// Q = 40 W なら何もしなくても (40/(ε σ A))^¼ ≈ 306 K で 280 K より高いのでオフ（P = 0）。
#[test]
fn single_node_required_power_by_hand() {
    use ThermalHeaterState::{Off, Regulating, Saturated};
    let net = network(&[("n", 0.1)], &[]);
    let required = radiated(0.1, 280.0) - 3.0;
    assert!((required - 24.88).abs() < 0.01, "{required}");

    let reg = solve(&net, &[3.0], &[heater("H", "n", 280.0, 285.0, 30.0)]);
    assert_eq!(reg.heaters[0].state, Regulating);
    assert!((reg.heaters[0].power_w - required).abs() < 1e-6);
    assert!((reg.solution.temps[0] - 280.0).abs() < 1e-9);

    let sat = solve(&net, &[3.0], &[heater("H", "n", 280.0, 285.0, 10.0)]);
    assert_eq!(sat.heaters[0].state, Saturated);
    assert_eq!(sat.heaters[0].power_w, 10.0);
    let t_sat = (13.0 / (0.8 * SIGMA_SB * 0.1)).powf(0.25);
    assert!((sat.solution.temps[0] - t_sat).abs() < 1e-6);
    assert!(t_sat < 280.0);

    let off = solve(&net, &[40.0], &[heater("H", "n", 280.0, 285.0, 30.0)]);
    assert_eq!(off.heaters[0].state, Off);
    assert_eq!(off.heaters[0].power_w, 0.0);
    let t_free = (40.0 / (0.8 * SIGMA_SB * 0.1)).powf(0.25);
    assert!((off.solution.temps[0] - t_free).abs() < 1e-6);

    // モードで切っていれば（働かなければ）、ヒータは 0 W で節点は自然な温度
    let specs = net
        .heaters(&[heater("H", "n", 280.0, 285.0, 30.0)])
        .unwrap();
    let disabled = net
        .solve_steady_heated(&[3.0], 0.0, 290.0, &specs, &[false])
        .unwrap();
    assert_eq!(disabled.heaters[0].state, ThermalHeaterState::Disabled);
    let t_cold = (3.0 / (0.8 * SIGMA_SB * 0.1)).powf(0.25);
    assert!((disabled.solution.temps[0] - t_cold).abs() < 1e-6);
}

/// 伝導 G = 0.2 W/K でつないだ 2 節点 A（0.1 m²、Q = 2 W、設定 280 K）と B（0.05 m²、Q = 0.5 W、設定 270 K）。
/// 両方を設定温度に保つ電力: P_A = R_A(280) − Q_A + G (280 − 270)、P_B = R_B(270) − Q_B + G (270 − 280)（R は ε σ A T⁴）。
/// B の容量が P_B ≈ 9.5 W より小さい 5 W なら B は飽和し、T_B は 0.5 + 5 + G (280 − T_B) = R_B(T_B) の解（二分法で求める）。
/// A はその T_B を前提に P_A = R_A(280) − Q_A + G (280 − T_B) を出す（B が冷えるので A の電力が増える）。
#[test]
fn two_coupled_heaters_have_consistent_sets() {
    use ThermalHeaterState::{Regulating, Saturated};
    let net = network(&[("A", 0.1), ("B", 0.05)], &[("A", "B", 0.2)]);
    let q = [2.0, 0.5];
    let (g, ta, tb) = (0.2, 280.0, 270.0);

    let both = solve(
        &net,
        &q,
        &[
            heater("HA", "A", ta, ta + 5.0, 40.0),
            heater("HB", "B", tb, tb + 5.0, 15.0),
        ],
    );
    let p_a = radiated(0.1, ta) - 2.0 + g * (ta - tb);
    let p_b = radiated(0.05, tb) - 0.5 + g * (tb - ta);
    assert!(p_b > 5.0 && p_b < 15.0, "{p_b}");
    assert_eq!(both.heaters[0].state, Regulating);
    assert_eq!(both.heaters[1].state, Regulating);
    assert!((both.heaters[0].power_w - p_a).abs() < 1e-6);
    assert!((both.heaters[1].power_w - p_b).abs() < 1e-6);

    // B の容量が足りない: T_B を二分法で求める
    let balance = |t_b: f64| 0.5 + 5.0 + g * (ta - t_b) - radiated(0.05, t_b);
    let (mut lo, mut hi) = (150.0, tb);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if balance(mid) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let t_b = 0.5 * (lo + hi);
    let sat = solve(
        &net,
        &q,
        &[
            heater("HA", "A", ta, ta + 5.0, 40.0),
            heater("HB", "B", tb, tb + 5.0, 5.0),
        ],
    );
    assert_eq!(sat.heaters[0].state, Regulating);
    assert_eq!(sat.heaters[1].state, Saturated);
    assert!(
        (sat.solution.temps[1] - t_b).abs() < 1e-5,
        "{}",
        sat.solution.temps[1]
    );
    assert!(t_b < tb);
    let p_a_sat = radiated(0.1, ta) - 2.0 + g * (ta - t_b);
    assert!((sat.heaters[0].power_w - p_a_sat).abs() < 1e-5);
    assert!(p_a_sat > p_a);
    assert!((sat.solution.temps[0] - ta).abs() < 1e-9);
}
