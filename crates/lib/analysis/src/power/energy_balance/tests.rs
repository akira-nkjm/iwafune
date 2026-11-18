use super::case::max_eclipse_drop;
use super::*;
use crate::modes::{ModeDef, StateAssignment};
use crate::power::mode_load::ModeLoadPart;
use crate::power::solar::SolarPanel;
use crate::power::testkit::{leo_timeline, sun_pointing_z};
use crate::spec::Verdict;
use sat_astro::attitude::AttitudeLaw;
use sat_astro::math::Vec3;
use std::collections::BTreeMap;

#[test]
fn eclipse_drop_by_hand() {
    let lit = [true, true, false, false, true, false, false];
    let soc = [1.0, 1.0, 0.9, 0.8, 0.85, 0.82, 0.7];
    // 1 回目: 1.0 → 0.8（0.2）、2 回目: 0.85 → 0.7（0.15）
    assert!((max_eclipse_drop(&lit, &soc) - 0.2).abs() < 1e-12);
}

#[test]
fn battery_validation() {
    let mut b = EnergyBalanceBattery {
        capacity: Qty::from_raw(36_000.0),
        voltage: Qty::from_raw(8.0),
        charge_efficiency: Some(0.9),
        discharge_efficiency: Some(0.95),
        initial_soc: Some(1.0),
        dod_limit: 0.3,
        max_charge_power: None,
    };
    assert!(EnergyBalanceBattery::combine(&[b.clone()]).is_ok());
    b.discharge_efficiency = Some(0.0);
    assert!(EnergyBalanceBattery::combine(&[b.clone()]).is_err());
}

fn battery_100wh() -> EnergyBalanceBattery {
    EnergyBalanceBattery {
        capacity: Qty::from_raw(360_000.0),
        voltage: Qty::from_raw(8.0),
        charge_efficiency: Some(0.9),
        discharge_efficiency: Some(0.95),
        initial_soc: Some(1.0),
        dod_limit: 0.3,
        max_charge_power: None,
    }
}

fn input(panels: Vec<SolarPanel>, cases: Vec<EnergyBalanceCase>) -> EnergyBalanceInput {
    EnergyBalanceInput {
        timeline: leo_timeline(sun_pointing_z(), 2.0, 10.0),
        array: SolarArray {
            panels,
            solar_flux: Some(Qty::from_raw(1361.0)),
            conversion_efficiency: None,
        },
        life: Some(SolarLife::Bol),
        batteries: vec![battery_100wh()],
        loads: ModeLoadInput::default(),
        load_growth: Some(LoadGrowth::Included),
        cases,
        sequence: Vec::new(),
        sequence_timing: Default::default(),
        base_mode: None,
        mode_attitudes: BTreeMap::new(),
    }
}

fn constant(id: &str, watts: f64) -> EnergyBalanceCase {
    EnergyBalanceCase {
        id: id.to_owned(),
        label: String::new(),
        load: EnergyBalanceLoad::Constant {
            power: Qty::from_raw(watts),
        },
        attitude: None,
        max_duration: None,
        timing: Default::default(),
    }
}

fn pz_panel() -> SolarPanel {
    SolarPanel {
        id: "PZ".to_owned(),
        area: Qty::from_raw(1.0),
        cell_efficiency: 0.3,
        packing_factor: Some(1.0),
        eol_factor: Some(0.8),
        normal: Vec3::Z,
    }
}

#[test]
fn discharge_only_by_hand() {
    // 発電なし、9.5 W を 2 周 → 電池からは 10 W。SOC = 1 − 10·span / 360 000 J
    let out = EnergyBalance::run(&input(Vec::new(), vec![constant("idle", 9.5)])).unwrap();
    let c = &out.cases[0];
    let span = *c.series.t.last().unwrap();
    let expect = 1.0 - 10.0 * span / 360_000.0;
    assert!(
        (c.final_soc.value() - expect).abs() < 1e-9,
        "{} vs {expect}",
        c.final_soc.value()
    );
    assert!((c.max_dod.value() - (1.0 - expect)).abs() < 1e-9);
    // 1 周の余裕は −10 W × 周期
    let margin = -10.0 * out.period.value();
    assert!(out.complete_orbits >= 2);
    assert!((c.energy_margin_per_orbit.unwrap().value() - margin).abs() < 1e-6 * margin.abs());
    assert_eq!(c.energy_margin_over_span, None);
    let verdict = |k: &str| c.checks.iter().find(|x| x.key == k).unwrap().verdict;
    assert_eq!(verdict("dod"), Verdict::Violation);
    assert_eq!(verdict("energy_balance"), Verdict::Violation);
    assert!((c.peak_discharge_current.value() - 9.5 / 8.0).abs() < 1e-12);
}

#[test]
fn half_orbit_reports_span_balance_without_judging_per_orbit_margin() {
    for watts in [0.0, 9.5] {
        let mut i = input(Vec::new(), vec![constant("idle", watts)]);
        i.timeline = leo_timeline(sun_pointing_z(), 0.5, 10.0);
        let out = EnergyBalance::run(&i).unwrap();
        let c = &out.cases[0];
        assert_eq!(out.complete_orbits, 0);
        assert_eq!(out.span_fallback, Some(SpanFallback::OrbitMeanOverSpan));
        assert!(c.orbits.is_empty());
        assert_eq!(c.energy_margin_per_orbit, None);
        // 発電なし: 9.5 W / 放電効率 0.95 = 10 W。周期へ換算せず実際の期間を掛ける。
        let span = c.series.t.last().unwrap() - c.series.t[0];
        let expected = 0.0 - watts / 0.95 * span;
        assert!((c.energy_margin_over_span.unwrap().value() - expected).abs() < 1e-6);
        let check = c.checks.iter().find(|c| c.key == "energy_balance").unwrap();
        assert_eq!(check.verdict, Verdict::Warning);
        assert_eq!(
            check.message,
            format!(
                "The analysis span is shorter than one orbit, so the per-orbit margin cannot be judged (balance over the span is {}, including battery efficiency)",
                crate::spec::quantity_text::<Energy>(expected)
            )
        );
    }
}

#[test]
fn sun_pointing_eclipse_dod_by_hand() {
    // 日陰中は 20 W / 0.95 を電池から引く → 1 回の日陰の DoD ≈ 21.05 W × 日陰の長さ / 360 000 J
    let out =
        EnergyBalance::run(&input(vec![pz_panel()], vec![constant("nominal", 20.0)])).unwrap();
    let c = &out.cases[0];
    let eclipse_s = (1.0 - out.sunlit_fraction.value()) * out.period.value();
    let expect = 20.0 / 0.95 * eclipse_s / 360_000.0;
    assert!(
        (c.max_eclipse_dod.value() - expect).abs() < 0.02 * expect,
        "{} vs {expect}",
        c.max_eclipse_dod.value()
    );
    assert!(c.energy_margin_per_orbit.unwrap().value() > 0.0);
    assert_eq!(c.energy_margin_over_span, None);
    assert!(
        c.checks.iter().all(|x| x.verdict == Verdict::Ok),
        "{:?}",
        c.checks
    );
    assert!(c.peak_charge_power.value() > 380.0);
    assert!(c.series.channels.iter().any(|ch| ch.key == "soc"));
}

#[test]
fn sunlit_and_eclipse_modes() {
    // 日照中は MIS（OBC 2 W + CAM 8 W）、日陰中は SAFE（OBC 2 W、CAM なし→未割り当ての警告）
    let part = |id: &str, w: f64| ModeLoadPart {
        id: id.to_owned(),
        label: String::new(),
        group: String::new(),
        count: 1,
        power: crate::modes::StateValues {
            default: None,
            by_state: [("on".to_owned(), Qty::from_raw(w))].into(),
        },
        growth_margin: Some(0.0),
        redundancy: crate::modes::Redundancy::None,
        states: Vec::new(),
    };
    let on = |p: &str| StateAssignment {
        part: p.to_owned(),
        state: "on".to_owned(),
        duty: Default::default(),
        active: None,
        spare_state: None,
    };
    let mode = |id: &str, a: Vec<StateAssignment>| ModeDef {
        id: id.to_owned(),
        label: String::new(),
        assignments: a,
        eclipse: Vec::new(),
    };
    let mut i = input(
        vec![pz_panel()],
        vec![EnergyBalanceCase {
            id: "mission".to_owned(),
            label: String::new(),
            load: EnergyBalanceLoad::SunlitEclipse {
                sunlit_mode: "MIS".to_owned(),
                eclipse_mode: "SAFE".to_owned(),
            },
            attitude: None,
            max_duration: None,
            timing: Default::default(),
        }],
    );
    i.loads = ModeLoadInput {
        parts: vec![part("OBC", 2.0), part("CAM", 8.0)],
        modes: vec![
            mode("MIS", vec![on("OBC"), on("CAM")]),
            mode("SAFE", vec![on("OBC")]),
        ],
        default_growth_margin: Some(0.0),
    };
    let c = &EnergyBalance::run(&i).unwrap().cases[0];
    assert!((c.load_sunlit.value() - 10.0).abs() < 1e-12);
    assert!((c.load_eclipse.value() - 2.0).abs() < 1e-12);
    assert!(c.checks.iter().any(|x| x.key == "unassigned"));
    // 未知のモードはエラー
    i.cases[0].load = EnergyBalanceLoad::Mode {
        mode: "NONE".to_owned(),
    };
    assert!(EnergyBalance::run(&i).is_err());
}

/// +X 面 1 枚（デモの SP_PX 相当: 0.0827 m²・29.5 %・実装率 0.73）。1361 W/m² で 24.24 W。
fn px_panel() -> SolarPanel {
    SolarPanel {
        id: "SP_PX".to_owned(),
        area: Qty::from_raw(0.0827),
        cell_efficiency: 0.295,
        packing_factor: Some(0.73),
        eol_factor: Some(0.88),
        normal: Vec3::X,
    }
}

/// ケースの時系列の `key` の値。
fn channel<'a>(c: &'a EnergyBalanceCaseResult, key: &str) -> &'a [f64] {
    &c.series
        .channels
        .iter()
        .find(|ch| ch.key == key)
        .unwrap()
        .values
}

/// 日照中（日照率 1）の標本で、発電が `full` [W] に等しいことを確かめる。
fn assert_full_when_sunlit(out: &EnergyBalanceOutput, c: &EnergyBalanceCaseResult, full: f64) {
    let sunlit = &out
        .series
        .channels
        .iter()
        .find(|ch| ch.key == "sunlit")
        .unwrap()
        .values;
    let generation = channel(c, "generation");
    let lit: Vec<f64> = generation
        .iter()
        .zip(sunlit)
        .filter(|(_, s)| **s >= 1.0)
        .map(|(g, _)| *g)
        .collect();
    assert!(!lit.is_empty());
    for g in lit {
        assert!((g - full).abs() < 1e-6 * full, "{g} vs {full}");
    }
}

#[test]
fn sun_pointing_plus_x_lights_the_plus_x_panel_fully() {
    // 太陽指向 {axis: +X, secondary: +Z}: 日照中は +X 面に垂直入射 → A·η·f_pack·Φ = 0.0827·0.295·0.73·1361
    let mut i = input(vec![px_panel()], vec![constant("SAFE", 4.0)]);
    i.timeline.attitude = AttitudeLaw::SunPointing {
        axis: Vec3::X,
        secondary: Vec3::Z,
    };
    let out = EnergyBalance::run(&i).unwrap();
    let full = 0.0827 * 0.295 * 0.73 * 1361.0;
    assert_full_when_sunlit(&out, &out.cases[0], full);
    let avg = out.cases[0].orbit_average_generation.value();
    assert!((avg - full * out.sunlit_fraction.value()).abs() < 0.01 * avg);
    assert!(
        out.cases[0].checks.iter().all(|x| x.verdict == Verdict::Ok),
        "{:?}",
        out.cases[0].checks
    );
}

#[test]
fn inertial_law_toward_the_sun_lights_the_panel_fully() {
    // 慣性固定（機体 = ECI）で、パネルの法線を元期の太陽方向に向ける。1 周では太陽方向はほぼ動かない
    let mut i = input(vec![px_panel()], vec![constant("SAFE", 4.0)]);
    i.timeline.attitude = AttitudeLaw::Inertial {
        q_body_to_eci: sat_astro::math::Quat::IDENTITY,
    };
    i.array.panels[0].normal = sat_astro::ephem::sun_direction_eci(i.timeline.orbit().epoch);
    let out = EnergyBalance::run(&i).unwrap();
    let full = 0.0827 * 0.295 * 0.73 * 1361.0;
    let sunlit = &out
        .series
        .channels
        .iter()
        .find(|ch| ch.key == "sunlit")
        .unwrap()
        .values;
    let generation = channel(&out.cases[0], "generation");
    for (g, s) in generation.iter().zip(sunlit).filter(|(_, s)| **s >= 1.0) {
        // 2 周で太陽方向は約 0.15° 動くだけ（cos で 1e-5 未満）。衛星から見た視差も 1e-4 rad 程度
        assert!((g - full).abs() < 1e-4 * full, "{g} vs {full} (sunlit {s})");
    }
}

#[test]
fn case_attitude_overrides_the_timeline_law() {
    // 時系列は地心指向（+X 面はほぼ軌道面の法線向き）。SAFE だけ太陽指向に替えると SAFE の発電が増える
    let mut i = input(
        vec![px_panel()],
        vec![constant("NADIR", 4.0), constant("SAFE", 4.0)],
    );
    i.timeline.attitude = AttitudeLaw::NadirPointing {
        axis: Vec3::Z,
        along_track: Vec3::Y,
    };
    i.cases[1].attitude = Some(AttitudeLaw::SunPointing {
        axis: Vec3::X,
        secondary: Vec3::Z,
    });
    let out = EnergyBalance::run(&i).unwrap();
    let full = 0.0827 * 0.295 * 0.73 * 1361.0;
    assert_full_when_sunlit(&out, &out.cases[1], full);
    let nadir = out.cases[0].orbit_average_generation.value();
    let sun = out.cases[1].orbit_average_generation.value();
    assert!(sun > 1.5 * nadir, "sun {sun} vs nadir {nadir}");
    assert!((out.orbit_average_generation.value() - nadir).abs() < 1e-12);
}

#[test]
fn battery_combination_by_hand() {
    let a = battery_100wh();
    let mut b = a.clone();
    b.capacity = Qty::from_raw(180_000.0);
    b.voltage = Qty::from_raw(16.0);
    b.charge_efficiency = Some(0.8);
    let total = EnergyBalanceBattery::combine(&[a, b]).unwrap().battery;
    assert_eq!(total.capacity.value(), 540_000.0); // 100 Wh + 50 Wh
    assert_eq!(total.voltage.value(), 8.0); // 先頭の公称電圧
    assert_eq!(total.charge_eff(), 0.9);
    assert!(EnergyBalanceBattery::combine(&[]).is_err());
}

#[test]
fn eclipse_assignment_reaches_balance_loads() {
    use super::case::case_loads;
    let loads: ModeLoadInput = serde_json::from_value(serde_json::json!({
        "parts": [{"id":"HTR", "power":{"by_state":{"off":0,"on":5}}}],
        "modes": [{"id":"HEAT","label":"", "assignments":[{"part":"HTR","state":"off"}],
            "eclipse":[{"part":"HTR","state":"on"}]}]
    }))
    .unwrap();
    let mut i = input(vec![pz_panel()], vec![]);
    i.loads = loads;
    let c: EnergyBalanceCase = serde_json::from_value(serde_json::json!({
        "id":"test", "label":"", "load":{"kind":"mode","mode":"HEAT"}
    }))
    .unwrap();
    let (sun, eclipse) = case_loads(&i, &c, &mut vec![]).unwrap();
    assert_eq!(sun.value(), 0.0);
    assert_eq!(eclipse.value(), 5.0);
}

mod operation;
mod page_outputs;
mod timing;
