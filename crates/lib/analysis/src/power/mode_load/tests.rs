use super::*;
use crate::modes::StateAssignment;

fn part(id: &str, group: &str, states: &[(&str, f64)], growth: Option<f64>) -> ModeLoadPart {
    ModeLoadPart {
        id: id.to_owned(),
        label: id.to_owned(),
        group: group.to_owned(),
        count: 1,
        power: StateValues {
            default: None,
            by_state: states
                .iter()
                .map(|(s, w)| ((*s).to_owned(), Qty::from_raw(*w)))
                .collect(),
        },
        growth_margin: growth,
        redundancy: crate::modes::Redundancy::None,
        states: Vec::new(),
    }
}

fn assign(part: &str, state: &str, duty: &[(&str, f64)]) -> StateAssignment {
    StateAssignment {
        part: part.to_owned(),
        state: state.to_owned(),
        duty: duty.iter().map(|(s, f)| ((*s).to_owned(), *f)).collect(),
        active: None,
        spare_state: None,
    }
}

fn input() -> ModeLoadInput {
    ModeLoadInput {
        parts: vec![
            part("OBC", "CDH", &[("on", 1.5)], Some(0.1)),
            part("TRX", "COMM", &[("rx", 2.0), ("tx", 10.0)], None),
            part("CAM", "MIS", &[("off", 0.0), ("on", 5.0)], None),
            part("HTR", "TCS", &[("on", 3.0)], None),
        ],
        modes: vec![ModeDef {
            id: "NOMINAL".to_owned(),
            label: "通常".to_owned(),
            assignments: vec![
                assign("OBC", "on", &[]),
                assign("TRX", "rx", &[("tx", 0.1)]),
                assign("CAM", "off", &[("on", 0.2)]),
                assign("GPS", "on", &[]),
            ],
            eclipse: Vec::new(),
        }],
        default_growth_margin: Some(0.2),
    }
}

#[test]
fn duty_and_growth_by_hand() {
    let out = ModeLoad::run(&input()).unwrap();
    let m = &out.modes[0];
    // OBC 1.5、TRX 0.9×2 + 0.1×10 = 2.8、CAM 0.2×5 = 1.0 → 5.3 W
    assert!((m.total.value() - 5.3).abs() < 1e-12);
    // 成長余裕: 1.5×1.1 + 2.8×1.2 + 1.0×1.2 = 1.65 + 3.36 + 1.2 = 6.21 W
    assert!((m.total_with_growth.value() - 6.21).abs() < 1e-12);
    // 最大: 1.5×1.1 + 10×1.2 + 5×1.2 = 19.65 W
    assert!((m.peak_with_growth.value() - 19.65).abs() < 1e-12);
    assert_eq!(m.groups.len(), 3);
    assert_eq!(m.groups[0].group, "CDH");
}

#[test]
fn unassigned_part_is_warned_not_zero() {
    let out = ModeLoad::run(&input()).unwrap();
    let m = &out.modes[0];
    assert_eq!(m.unassigned, vec!["HTR".to_owned()]);
    assert!(m.parts.iter().all(|p| p.part != "HTR"));
    assert!(m.checks.iter().any(|c| c.key == "unassigned"));
    assert!(m.checks.iter().any(|c| c.key == "unknown_part"));
}

#[test]
fn missing_state_power_is_reported() {
    let mut i = input();
    i.parts[2].power.by_state.remove("on");
    let m = &ModeLoad::run(&i).unwrap().modes[0];
    assert_eq!(m.missing_power.len(), 1);
    assert_eq!(m.missing_power[0].state, "on");
    let cam = m.parts.iter().find(|p| p.part == "CAM").unwrap();
    assert!(cam.mean.is_none());
    assert!((m.total.value() - 4.3).abs() < 1e-12);
}

#[test]
fn count_multiplies_the_unit_power() {
    // TRX を 2 台: 0.9×2×2 + 0.1×10×2 = 5.6 W、成長余裕 20 % で 6.72 W。状態の電力も 2 台分を出す
    let mut i = input();
    i.parts[1].count = 2;
    let m = &ModeLoad::run(&i).unwrap().modes[0];
    let trx = m.parts.iter().find(|p| p.part == "TRX").unwrap();
    assert!((trx.mean.unwrap().value() - 5.6).abs() < 1e-12);
    assert!((trx.mean_with_growth.unwrap().value() - 6.72).abs() < 1e-12);
    assert_eq!(trx.states[1].power.map(Qty::value), Some(20.0));
    // 合計は 1.5 + 5.6 + 1.0 = 8.1 W
    assert!((m.total.value() - 8.1).abs() < 1e-12);
}

fn redundant_input(redundancy: modes::Redundancy) -> ModeLoadInput {
    let mut rw = part(
        "RW",
        "AOCS",
        &[("off", 0.0), ("on", 10.0), ("standby", 2.0)],
        Some(0.0),
    );
    rw.count = 4;
    rw.redundancy = redundancy;
    rw.states = ["off", "on", "standby"].map(str::to_owned).into();
    let mut assignment = assign("RW", "on", &[]);
    assignment.active = Some(3);
    ModeLoadInput {
        parts: vec![rw],
        modes: vec![ModeDef {
            id: "TEST".into(),
            label: String::new(),
            assignments: vec![assignment],
            eclipse: vec![],
        }],
        default_growth_margin: Some(0.0),
    }
}

#[test]
fn cold_and_warm_units_by_hand() {
    for (redundancy, expected) in [
        (modes::Redundancy::Cold, 30.0),
        (modes::Redundancy::Warm, 32.0),
    ] {
        let i = redundant_input(redundancy);
        let m = evaluate_mode(&i, &i.modes[0]).unwrap();
        assert_eq!(m.total.value(), expected); // 3 × 10 + 1 × (0 または 2)
        assert_eq!(m.peak_with_growth.value(), expected);
        assert_eq!(m.total_eclipse.value(), expected);
    }
}

#[test]
fn spare_is_constant_while_active_units_follow_duty() {
    let mut i = redundant_input(modes::Redundancy::Warm);
    i.modes[0].assignments[0].state = "off".into();
    i.modes[0].assignments[0].duty.insert("on".into(), 0.2);
    let m = evaluate_mode(&i, &i.modes[0]).unwrap();
    assert_eq!(m.total.value(), 8.0); // 3 × 10 × 0.2 + 1 × 2
    assert_eq!(m.peak_with_growth.value(), 32.0);
}

#[test]
fn eclipse_overrides_only_the_heater() {
    let mut i = redundant_input(modes::Redundancy::Cold);
    i.parts
        .push(part("HTR", "TCS", &[("off", 0.0), ("on", 5.0)], Some(0.0)));
    i.modes[0].assignments.push(assign("HTR", "off", &[]));
    i.modes[0].eclipse.push(assign("HTR", "on", &[]));
    let m = evaluate_mode(&i, &i.modes[0]).unwrap();
    assert_eq!(m.total.value(), 30.0);
    assert_eq!(m.total_eclipse.value(), 35.0);
}

#[test]
fn invalid_population_is_rejected() {
    let mut i = redundant_input(modes::Redundancy::Warm);
    i.modes[0].assignments[0].active = Some(5);
    assert!(evaluate_mode(&i, &i.modes[0]).is_err());
    i.modes[0].assignments[0].active = Some(3);
    i.parts[0].states = vec!["off".into(), "on".into()];
    assert!(evaluate_mode(&i, &i.modes[0]).is_err());
    i.modes[0].assignments[0].spare_state = Some("on".into());
    assert_eq!(evaluate_mode(&i, &i.modes[0]).unwrap().total.value(), 40.0);
}

#[test]
fn eclipse_parts_population_duty_growth_and_inheritance_by_hand() {
    let mut input = input();
    input.parts[1].count = 2;
    input.modes[0].eclipse = vec![assign("TRX", "rx", &[("tx", 0.25)])];
    let mode = evaluate_mode(&input, &input.modes[0]).unwrap();
    let trx = mode.parts.iter().find(|p| p.part == "TRX").unwrap();
    // 日照 2 × (2 × 0.9 + 10 × 0.1) = 5.6 W、蝕 2 × (2 × 0.75 + 10 × 0.25) = 8 W
    assert!((trx.mean.unwrap().value() - 5.6).abs() < 1e-12);
    assert_eq!(trx.mean_eclipse.unwrap().value(), 8.0);
    assert!((trx.mean_eclipse_with_growth.unwrap().value() - 9.6).abs() < 1e-12);
    let obc = mode.parts.iter().find(|p| p.part == "OBC").unwrap();
    assert_eq!(obc.mean_eclipse, obc.mean);
    assert_eq!(obc.mean_eclipse_with_growth, obc.mean_with_growth);
}

#[test]
fn missing_eclipse_value_is_preserved_per_part() {
    // 蝕の上書きで値の無い状態にした部品は、蝕の値だけ None（日照の値は残す）
    let mut input = input();
    input.modes[0].eclipse = vec![assign("TRX", "missing", &[])];
    let mode = evaluate_mode(&input, &input.modes[0]).unwrap();
    let trx = mode.parts.iter().find(|p| p.part == "TRX").unwrap();
    assert!(trx.mean.is_some());
    assert_eq!(trx.mean_eclipse, None);
    input.modes[0].eclipse.clear();
    let inherited = evaluate_mode(&input, &input.modes[0]).unwrap();
    assert!(inherited.parts.iter().all(|p| p.mean == p.mean_eclipse));
}

/// 成長余裕の既定値を書かなければ `DEFAULT_GROWTH_MARGIN`（0）、書けばその値。
#[test]
fn default_growth_margin_is_applied_by_the_analysis() {
    let mut input = ModeLoadInput::default();
    assert_eq!(input.default_growth_margin(), DEFAULT_GROWTH_MARGIN);
    assert_eq!(DEFAULT_GROWTH_MARGIN, 0.0);
    input.default_growth_margin = Some(0.2);
    assert_eq!(input.default_growth_margin(), 0.2);
}
