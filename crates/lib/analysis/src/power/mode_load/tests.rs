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
    }
}

fn assign(part: &str, state: &str, duty: &[(&str, f64)]) -> StateAssignment {
    StateAssignment {
        part: part.to_owned(),
        state: state.to_owned(),
        duty: duty.iter().map(|(s, f)| ((*s).to_owned(), *f)).collect(),
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
        }],
        default_growth_margin: 0.2,
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
