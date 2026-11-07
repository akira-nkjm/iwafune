use super::*;

fn assign(state: &str, duty: &[(&str, f64)]) -> StateAssignment {
    StateAssignment {
        part: "TRX".to_owned(),
        state: state.to_owned(),
        duty: duty.iter().map(|(s, f)| ((*s).to_owned(), *f)).collect(),
    }
}

#[test]
fn fractions_sum_to_one() {
    let f = assign("rx", &[("tx", 0.1)]).fractions().unwrap();
    assert_eq!(f[0].0, "rx");
    assert!((f[0].1 - 0.9).abs() < 1e-12);
    assert_eq!(f[1], ("tx".to_owned(), 0.1));
    let total: f64 = f.iter().map(|(_, x)| x).sum();
    assert!((total - 1.0).abs() < 1e-12);
}

#[test]
fn fractions_reject_bad_duty() {
    assert!(
        assign("rx", &[("tx", 0.7), ("beacon", 0.4)])
            .fractions()
            .is_err()
    );
    assert!(assign("rx", &[("tx", -0.1)]).fractions().is_err());
    assert!(assign("rx", &[("rx", 0.1)]).fractions().is_err());
}

#[test]
fn weighted_mean_by_hand() {
    // rx 2 W を 90 %、tx 10 W を 10 % → 0.9×2 + 0.1×10 = 2.8 W
    let a = assign("rx", &[("tx", 0.1)]);
    let p = |s: &str| match s {
        "rx" => Some(2.0),
        "tx" => Some(10.0),
        _ => None,
    };
    assert!((a.weighted_mean(p).unwrap().unwrap() - 2.8).abs() < 1e-12);
}

#[test]
fn weighted_mean_missing_value_is_none() {
    let a = assign("rx", &[("tx", 0.1)]);
    assert_eq!(
        a.weighted_mean(|s| (s == "rx").then_some(2.0)).unwrap(),
        None
    );
    // 割合 0 の状態は値が無くてもよい
    let full = assign("rx", &[("tx", 1.0)]);
    assert_eq!(
        full.weighted_mean(|s| (s == "tx").then_some(5.0)).unwrap(),
        Some(5.0)
    );
}

#[test]
fn duplicate_assignment_is_rejected() {
    let m = ModeDef {
        id: "NOMINAL".to_owned(),
        label: "通常".to_owned(),
        assignments: vec![assign("rx", &[]), assign("tx", &[])],
    };
    assert!(m.validate().is_err());
    assert!(m.assignment("TRX").is_some());
    assert!(m.assignment("OBC").is_none());
}

#[test]
fn evaluate_excludes_unassigned_and_missing_with_warnings() {
    // TRX: rx 2 W × 0.9 + tx 10 W × 0.1 = 2.8 W。CAM は未割り当て、GPS は値が無い、HTR は一覧に無い
    let mode = ModeDef {
        id: "NOMINAL".to_owned(),
        label: String::new(),
        assignments: vec![
            assign("rx", &[("tx", 0.1)]),
            StateAssignment {
                part: "GPS".to_owned(),
                state: "on".to_owned(),
                duty: BTreeMap::new(),
            },
            StateAssignment {
                part: "HTR".to_owned(),
                state: "on".to_owned(),
                duty: BTreeMap::new(),
            },
        ],
    };
    let value = |part: &str, state: &str| match (part, state) {
        ("TRX", "rx") => Some(2.0),
        ("TRX", "tx") => Some(10.0),
        _ => None,
    };
    let ev = evaluate(&mode, ["TRX", "CAM", "GPS"], value).unwrap();
    assert!((ev.total() - 2.8).abs() < 1e-12);
    assert_eq!(ev.unassigned, vec!["CAM".to_owned()]);
    assert_eq!(ev.missing, vec![("GPS".to_owned(), "on".to_owned())]);
    assert_eq!(ev.unknown_parts, vec!["HTR".to_owned()]);
    let keys: Vec<String> = ev.checks("消費電力").into_iter().map(|c| c.key).collect();
    assert_eq!(keys, ["unassigned", "missing_value", "unknown_part"]);
    assert!(find_mode(std::slice::from_ref(&mode), "NOMINAL").is_ok());
    assert!(find_mode(std::slice::from_ref(&mode), "SAFE").is_err());
}
