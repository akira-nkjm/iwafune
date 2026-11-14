//! 手計算、および逆算した設計を既存の電力収支へ渡す比較。
use super::*;
use crate::modes::{ModeDef, StateAssignment, StateValues};
use crate::power::mode_load::ModeLoadPart;
use crate::power::solar::SolarPanel;
use crate::power::testkit::{leo, leo_timeline, nadir_pointing_z, sun_pointing_z};
use sat_astro::constants::AU;
use sat_astro::math::Vec3;
use std::collections::BTreeMap;

fn sap(area: f64, normal: Vec3) -> SolarArray {
    SolarArray {
        panels: vec![SolarPanel {
            id: "SAP".into(),
            area: Qty::from_raw(area),
            cell_efficiency: 0.3,
            packing_factor: Some(0.9),
            eol_factor: Some(0.8),
            normal,
        }],
        solar_flux: None,
        conversion_efficiency: None,
    }
}

fn input() -> SizingInput {
    SizingInput {
        season: EclipseSeasonInput {
            orbit: leo(),
            duration: Qty::from_raw(86400.0),
            step: None,
            maneuvers: None,
        },
        timeline: leo_timeline(sun_pointing_z(), 2.0, 10.0),
        mode_laws: BTreeMap::new(),
        loads: ModeLoadInput {
            parts: vec![ModeLoadPart {
                id: "LOAD".into(),
                label: String::new(),
                group: String::new(),
                count: 1,
                redundancy: Default::default(),
                states: vec!["on".into(), "eclipse".into()],
                power: StateValues {
                    default: None,
                    by_state: BTreeMap::from([
                        ("on".into(), Qty::from_raw(100.0)),
                        ("eclipse".into(), Qty::from_raw(60.0)),
                    ]),
                },
                growth_margin: None,
            }],
            modes: vec![ModeDef {
                id: "NOMINAL".into(),
                label: String::new(),
                assignments: vec![assignment("on")],
                eclipse: vec![assignment("eclipse")],
            }],
            default_growth_margin: 0.0,
        },
        mode: None,
        array: sap(1.0, Vec3::Z),
        cells: vec![SizingCell {
            panel: "SAP".into(),
            voltage: Qty::from_raw(2.0),
            current: Qty::from_raw(0.5),
        }],
        batteries: vec![EnergyBalanceBattery {
            capacity: Qty::from_raw(360_000.0),
            voltage: Qty::from_raw(28.0),
            charge_efficiency: Some(0.9),
            discharge_efficiency: Some(0.9),
            initial_soc: Some(1.0),
            dod_limit: 0.3,
            max_charge_power: None,
        }],
        options: SizingOptions {
            bus_voltage: Some(Qty::from_raw(28.0)),
            ..Default::default()
        },
        max_durations: BTreeMap::new(),
        sequence: Vec::new(),
        base_mode: None,
    }
}

fn assignment(state: &str) -> StateAssignment {
    StateAssignment {
        part: "LOAD".into(),
        state: state.into(),
        active: None,
        spare_state: None,
        duty: BTreeMap::new(),
    }
}

#[test]
fn smad_by_hand() {
    let mut i = input();
    i.season.duration = Qty::from_raw(5.0 * 365.25 * 86400.0);
    i.options.annual_degradation = Some(0.02);
    i.options.inherent_degradation = Some(0.77);
    i.options.battery_strings = Some(2);
    let u = used(&i, &i.batteries[0]).unwrap();
    let cos30 = (std::f64::consts::PI / 6.0).cos();
    let density = 1361.0 * 0.3 * 0.9 * 0.8 * u.life_factor * 0.77 * cos30;
    // Te = 1800 s, Td = 3600 s:
    // P = (60*1800/0.6 + 100*3600/0.8)/3600 = 175 W
    // A = 175/(1361*0.3*0.9*0.8*0.98^5*0.77*cos30°) = 0.987585… m²
    // C_total = 60*1800/(0.3*0.9) = 400000 J、N=2 なら各 200000 J。
    let (power, area, capacity) =
        calculate::smad_requirements(100.0, 60.0, 3600.0, 1800.0, &u, density).unwrap();
    assert!((power - 175.0).abs() < 1e-12);
    assert!((area - 0.9875848908518454).abs() < 1e-12);
    assert!((capacity - 400000.0).abs() < 1e-9);
    let strings = calculate::strings(&i, &u, power, cos30).unwrap().unwrap();
    assert_eq!(strings.series_count, 14);
    assert_eq!(strings.parallel_count, 26);
    assert!((strings.required_current.value() - 6.25).abs() < 1e-12);
}

#[test]
fn shared_season_and_inverse_margins() {
    let mut i = input();
    i.options.battery_strings = Some(2);
    let o = Sizing::run(&i).unwrap();
    assert_eq!(o.basis, SizingBasis::TimeSeries);
    assert_eq!(
        o.required_capacity_per_string.value() * 2.0,
        o.required_capacity.value()
    );
    let season = EclipseSeason::run(&i.season).unwrap();
    assert_eq!(o.eclipse_duration, season.max_eclipse_duration);
    assert_eq!(o.longest.epoch, season.longest.epoch);
    assert_eq!(o.smad.sunlit_load.value(), 100.0);
    assert_eq!(o.smad.eclipse_load.value(), 60.0);
    // 設計を必要量ちょうどにすると余裕は 0（時系列の面積は単位面積あたりの発電から求めるので設計の面積によらない）。
    let mut exact = i.clone();
    exact.array.panels[0].area = o.required_area;
    exact.batteries[0].capacity = o.required_capacity;
    let result = Sizing::run(&exact).unwrap();
    assert!(result.sap_margin.unwrap().value().abs() < 2.0 * AREA_TOLERANCE);
    assert!(result.battery_margin.unwrap().value().abs() < 1e-3);
    assert!(result.smad.sap_margin.is_some());
    assert_eq!(result.used.sunlit_efficiency, 0.8);
    assert_eq!(result.used.eclipse_efficiency, 0.6);
    assert_eq!(result.used.battery_efficiency, 0.9);
}

#[test]
fn incidence_follows_the_attitude() {
    // 太陽指向で +Z 面は日照中つねに垂直入射（円柱影なので半影も無い）→ 入射係数 1。
    let o = Sizing::run(&input()).unwrap();
    assert!((o.modes[0].incidence_factor.value() - 1.0).abs() < 1e-9);
    // 法線を太陽方向から 60° 傾けると cos 60° = 0.5、必要面積は 2 倍。
    let mut tilted = input();
    let t = 60f64.to_radians();
    tilted.array = sap(1.0, Vec3::new(t.sin(), 0.0, t.cos()));
    let ot = Sizing::run(&tilted).unwrap();
    assert!((ot.modes[0].incidence_factor.value() - 0.5).abs() < 1e-9);
    assert!((ot.smad.required_area.value() / o.smad.required_area.value() - 2.0).abs() < 1e-9);
    // モードの姿勢則は上段の姿勢則を替える。地心指向で天頂を向く面は日照中の平均の入射が 1 より小さい。
    let mut nadir = input();
    nadir.array = sap(1.0, -Vec3::Z);
    nadir.mode_laws.insert("NOMINAL".into(), nadir_pointing_z());
    let on = Sizing::run(&nadir).unwrap();
    let k = on.modes[0].incidence_factor.value();
    assert!(k > 0.3 && k < 0.7, "{k}");
    assert!((on.smad.required_area.value() * k - o.smad.required_area.value()).abs() < 1e-9);
}

/// 太陽光の強さは (1 AU/d)² に比例するので、2 AU では SMAD の必要面積が 4 倍（入射係数・容量は変わらない）。
/// 太陽距離の慣例値の無い中心天体（火星）では sun_distance を書かないと止まる。
#[test]
fn sun_distance_scales_the_required_area() {
    let base = Sizing::run(&input()).unwrap();
    let mut far = input();
    far.options.sun_distance = Some(Qty::from_raw(2.0 * AU));
    let far = Sizing::run(&far).unwrap();
    let ratio = far.smad.required_area.value() / base.smad.required_area.value();
    assert!((ratio - 4.0).abs() < 1e-9, "{ratio}");
    assert_eq!(
        far.smad.required_capacity.value(),
        base.smad.required_capacity.value()
    );
    assert_eq!(far.used.sun_distance.value(), 2.0 * AU);
    assert_eq!(base.used.sun_distance.value(), AU);
    let mut mars = input();
    mars.season.orbit.central = sat_astro::ephem::CentralBody::Mars;
    assert!(
        Sizing::run(&mars)
            .unwrap_err()
            .to_string()
            .contains("sun_distance")
    );
}

#[test]
fn rejects_missing_parts_and_invalid_options() {
    let mut i = input();
    i.array.panels.clear();
    assert!(Sizing::run(&i).is_err());
    i = input();
    i.batteries.clear();
    assert!(Sizing::run(&i).is_err());
    for value in [0.0, -1.0, f64::NAN, 1.1] {
        i = input();
        i.options.eclipse_efficiency = Some(value);
        assert!(Sizing::run(&i).is_err());
    }
}

#[test]
fn optional_strings_and_mode_selection() {
    let mut i = input();
    let mut high = i.loads.modes[0].clone();
    high.id = "HIGH".into();
    high.eclipse = vec![assignment("on")];
    i.loads.modes.push(high);
    let worst = Sizing::run(&i).unwrap();
    assert_eq!(worst.smad.eclipse_load.value(), 100.0);
    i.mode = Some("NOMINAL".into());
    i.cells.clear();
    let selected = Sizing::run(&i).unwrap();
    assert_eq!(selected.smad.eclipse_load.value(), 60.0);
    assert!(selected.strings.is_none());
    i.mode = Some("missing".into());
    assert!(Sizing::run(&i).is_err());
    i.mode = None;
    i.loads.modes[0].assignments.clear();
    assert!(Sizing::run(&i).is_err());
}

#[test]
fn per_mode_checks_compare_hand_checkable_requirements() {
    // モード A は必要 2 m²・120 J、設計は 3 m²・100 J → 面積 OK、容量不足。
    // モード B は必要 4 m²・80 J → 面積不足、容量 OK。
    let mode = |id: &str, area, capacity| SizingMode {
        id: id.into(),
        sunlit_load: Qty::from_raw(30.0),
        eclipse_load: Qty::from_raw(15.0),
        incidence_factor: Fraction::new(1.0),
        required_generation: Qty::from_raw(60.0),
        required_area: Qty::from_raw(area),
        required_capacity: Qty::from_raw(capacity),
        series_area: None,
        series_capacity: None,
        area_ratio: None,
        capacity_ratio: None,
    };
    let checks = mode_checks(
        &[mode("A", 2.0, 120.0), mode("B", 4.0, 80.0)],
        SizingBasis::Smad,
        Qty::from_raw(3.0),
        Qty::from_raw(100.0),
    );
    use crate::spec::Verdict::{Ok, Violation};
    assert_eq!(
        checks["A"].iter().map(|c| c.verdict).collect::<Vec<_>>(),
        [Ok, Violation]
    );
    assert_eq!(
        checks["B"].iter().map(|c| c.verdict).collect::<Vec<_>>(),
        [Violation, Ok]
    );
}

mod series;
