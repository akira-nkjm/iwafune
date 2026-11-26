//! ページ用に追加した解析出力も手計算できる小さなサンプルで検証する。
use super::*;
use sat_astro::{math::Quat, orbit::Cycle, timeline::TimelineSample};

fn profile(input: &EnergyBalanceInput) -> SolarProfile {
    let mut p = SolarProfile::build(&input.array, &[]);
    p.t = vec![0.0, 10.0, 20.0];
    p.lit = vec![true, true, false];
    p.sunlit = vec![1.0, 1.0, 0.0];
    p.panel_bol = vec![vec![60.0, 60.0, 0.0]];
    p
}

#[test]
fn sunlit_means_by_hand_include_charge_shunt_and_discharge() {
    // 二つの日照サンプルはともに 60 W、負荷 30 W → 余剰 30 W。
    for (start_soc, expected_charge, expected_shunt) in [(1.0, 0.0, 30.0), (0.5, 30.0, 0.0)] {
        let mut i = input(vec![pz_panel()], vec![constant("nominal", 30.0)]);
        i.batteries[0].initial_soc = Some(start_soc);
        let battery = EnergyBalanceBattery::combine(&i.batteries).unwrap().battery;
        let p = profile(&i);
        let profiles = Profiles::new(&i);
        let ctx = Context::new(
            &i,
            &battery,
            &p,
            &profiles,
            Cycle::Periodic { period_s: 100.0 },
        );
        let (case, _) = evaluate_case(&ctx, &i.cases[0]).unwrap();
        assert_eq!(case.sunlit_average_generation.value(), 60.0);
        assert_eq!(case.sunlit_average_discharge.value(), 0.0);
        assert_eq!(case.sunlit_average_charge.value(), expected_charge);
        assert_eq!(case.sunlit_average_shunt.value(), expected_shunt);
    }
    // 発電が負荷に足りない日照サンプル: 20 W − 30 W → バス側放電 10 W。
    let i = input(vec![pz_panel()], vec![constant("nominal", 30.0)]);
    let battery = EnergyBalanceBattery::combine(&i.batteries).unwrap().battery;
    let mut p = profile(&i);
    p.panel_bol[0] = vec![20.0, 20.0, 0.0];
    let profiles = Profiles::new(&i);
    let (case, _) = evaluate_case(
        &Context::new(
            &i,
            &battery,
            &p,
            &profiles,
            Cycle::Periodic { period_s: 100.0 },
        ),
        &i.cases[0],
    )
    .unwrap();
    assert_eq!(case.sunlit_average_discharge.value(), 10.0);
}

#[test]
fn output_limits_mode_loads_and_eclipses_by_hand() {
    let mut i = input(vec![pz_panel()], vec![constant("nominal", 30.0)]);
    i.loads = serde_json::from_value(serde_json::json!({
        "parts":[{"id":"CPU","power":{"by_state":{"on":30,"idle":15}}}],
        "modes":[{"id":"nominal","label":"","assignments":[{"part":"CPU","state":"on"}],"eclipse":[{"part":"CPU","state":"idle"}]}]
    })).unwrap();
    let samples: Vec<_> = [1.0, 0.5, 0.0, 0.0, 1.0]
        .into_iter()
        .enumerate()
        .map(|(index, sunlit)| TimelineSample {
            t_s: index as f64 * 10.0,
            epoch: i.timeline.orbit().epoch().plus_seconds(index as f64 * 10.0),
            r_eci: Vec3::X,
            v_eci: Vec3::Y,
            sun_dir_eci: Vec3::Z,
            sun_distance_m: sat_astro::constants::AU,
            sunlit,
            q_body_to_eci: Quat::IDENTITY,
        })
        .collect();
    // 日照率 0.5 は日照。蝕は t=20,30 s の二サンプル → 区間 20..30 s。
    let p = SolarProfile::build(&i.array, &samples);
    let battery = EnergyBalanceBattery::combine(&i.batteries).unwrap().battery;
    let profiles = Profiles::new(&i);
    let out = Context::new(
        &i,
        &battery,
        &p,
        &profiles,
        Cycle::Periodic { period_s: 100.0 },
    )
    .output(
        vec![],
        None,
        crate::power::mode_load::ModeLoad::run(&i.loads).unwrap(),
    );
    assert_eq!(out.dod_limit.value(), 0.3);
    assert!((out.min_allowed_soc.value() - 0.7).abs() < 1e-12);
    assert_eq!(out.mode_loads.modes[0].total_with_growth.value(), 30.0);
    assert_eq!(
        out.mode_loads.modes[0].total_eclipse_with_growth.value(),
        15.0
    );
    assert_eq!(out.eclipses.len(), 1);
    assert_eq!(out.eclipses[0].start.value(), 20.0);
    assert_eq!(out.eclipses[0].end.value(), 30.0);
    assert_eq!(out.eclipses[0].duration.value(), 10.0);
}

/// 電池の電圧が違うときだけ、不一致の注意が出力の最上位の `checks` に 1 つだけ載る（ケースや並びには重ねない）。
#[test]
fn unequal_batteries_put_one_warning_on_the_output() {
    let mut i = input(Vec::new(), vec![constant("a", 5.0), constant("b", 9.5)]);
    let first = EnergyBalance::run(&i).unwrap();
    assert!(first.checks.is_empty());
    let mut other = i.batteries[0].clone();
    other.voltage = Qty::from_raw(16.0);
    i.batteries.push(other);
    let out = EnergyBalance::run(&i).unwrap();
    assert_eq!(out.cases.len(), 2);
    assert_eq!(
        out.checks
            .iter()
            .filter(|k| k.key == "battery_mismatch")
            .count(),
        1
    );
    assert!(
        out.cases
            .iter()
            .all(|c| c.checks.iter().all(|k| k.key != "battery_mismatch"))
    );
}
