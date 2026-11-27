//! モードの並びと最長継続時間の評価を手計算と比べる。

use super::{input, pz_panel};
use crate::modes::{ModeDef, StateAssignment, StateValues};
use crate::power::energy_balance::{
    EnergyBalance, EnergyBalanceCase, EnergyBalanceDuty, EnergyBalanceEvaluation,
    EnergyBalanceInput, EnergyBalanceLoad, EnergyBalanceSegment,
};
use crate::power::mode_load::{ModeLoadInput, ModeLoadPart};
use crate::spec::{Analysis, Verdict};
use sat_units::typed::Qty;

/// 電池から見た電力 [W] = バス側 / 放電効率 0.95。
pub(super) fn from_battery(bus_w: f64) -> f64 {
    bus_w / 0.95
}

/// 部品 OBC（off 0 W・on 9.5 W）と CAM（on 9.5 W）、モード IDLE（OBC off）・A（OBC on）・
/// B（OBC と CAM on）・DUTY（OBC を on 半分）。
pub(super) fn loads() -> ModeLoadInput {
    let part = |id: &str, states: &[(&str, f64)]| ModeLoadPart {
        id: id.to_owned(),
        label: String::new(),
        group: String::new(),
        count: 1,
        power: StateValues {
            default: None,
            by_state: states
                .iter()
                .map(|(s, w)| ((*s).to_owned(), Qty::from_raw(*w)))
                .collect(),
        },
        growth_margin: Some(0.0),
        redundancy: crate::modes::Redundancy::None,
        states: Vec::new(),
    };
    let set = |p: &str, state: &str, duty: &[(&str, f64)]| StateAssignment {
        part: p.to_owned(),
        state: state.to_owned(),
        duty: duty.iter().map(|(s, f)| ((*s).to_owned(), *f)).collect(),
        active: None,
        spare_state: None,
    };
    let mode = |id: &str, a: Vec<StateAssignment>| ModeDef {
        id: id.to_owned(),
        label: String::new(),
        assignments: a,
        eclipse: Vec::new(),
    };
    ModeLoadInput {
        parts: vec![
            part("OBC", &[("off", 0.0), ("on", 9.5)]),
            part("CAM", &[("off", 0.0), ("on", 9.5)]),
        ],
        modes: vec![
            mode("IDLE", vec![set("OBC", "off", &[]), set("CAM", "off", &[])]),
            mode("A", vec![set("OBC", "on", &[]), set("CAM", "off", &[])]),
            mode("B", vec![set("OBC", "on", &[]), set("CAM", "on", &[])]),
            mode(
                "DUTY",
                vec![set("OBC", "off", &[("on", 0.5)]), set("CAM", "off", &[])],
            ),
        ],
        default_growth_margin: Some(0.0),
    }
}

pub(super) fn mode_case(mode: &str, max_duration: Option<f64>) -> EnergyBalanceCase {
    EnergyBalanceCase {
        id: mode.to_owned(),
        label: String::new(),
        load: EnergyBalanceLoad::Mode {
            mode: mode.to_owned(),
        },
        attitude: None,
        max_duration: max_duration.map(Qty::from_raw),
        timing: Default::default(),
    }
}

pub(super) fn segment(mode: &str, seconds: f64) -> EnergyBalanceSegment {
    EnergyBalanceSegment {
        mode: mode.to_owned(),
        duration: Qty::from_raw(seconds),
    }
}

pub(super) fn with_loads(mut i: EnergyBalanceInput) -> EnergyBalanceInput {
    i.loads = loads();
    i
}

#[test]
fn sequence_of_two_alternating_modes_by_hand() {
    // 発電なし。A（9.5 W）600 s → B（19 W）300 s を繰り返す。電池からは 10 W・20 W。
    // 1 周期（900 s）で 10·600 + 20·300 = 12 000 J 減る（区間の境目は刻み 10 s のサンプルに揃う）
    let mut i = with_loads(input(Vec::new(), vec![]));
    i.sequence = vec![segment("A", 600.0), segment("B", 300.0)];
    let out = EnergyBalance::run(&i).unwrap();
    let s = out.sequence.as_ref().unwrap();
    assert_eq!(s.cycle.value(), 900.0);
    let b = &s.balance;
    assert_eq!(b.evaluation, EnergyBalanceEvaluation::Sequence);
    assert_eq!(b.duty, EnergyBalanceDuty::NotUsed);
    let soc = &b
        .series
        .channels
        .iter()
        .find(|c| c.key == "soc")
        .unwrap()
        .values;
    let per_cycle = from_battery(9.5) * 600.0 + from_battery(19.0) * 300.0;
    for n in 1..=10 {
        let k = (b.series.t.iter())
            .position(|&t| (t - 900.0 * f64::from(n)).abs() < 1e-6)
            .unwrap();
        let expect = 1.0 - f64::from(n) * per_cycle / 360_000.0;
        assert!(
            (soc[k] - expect).abs() < 1e-9,
            "cycle {n}: {} vs {expect}",
            soc[k]
        );
    }
    // 1 周目（周期 P）: 6 周期と、残り（< 600 s）は A
    let p = out.period.value();
    let rest = p - 6.0 * 900.0;
    assert!(rest > 0.0 && rest < 600.0);
    let expect = -(6.0 * per_cycle + from_battery(9.5) * rest);
    let margin = b.orbits[0].margin.value();
    assert!((margin - expect).abs() < 100.0, "{margin} vs {expect}");
    // 日照・日陰の別なく負荷は 9.5 W か 19 W
    assert!(b.load_eclipse.value() > 9.5 && b.load_eclipse.value() < 19.0);
    let dod = b.checks.iter().find(|c| c.key == "dod").unwrap();
    assert!(
        dod.message.contains("along the mode sequence"),
        "{}",
        dod.message
    );
}

#[test]
fn duration_without_generation_by_hand() {
    // 発電なし・基本のモード IDLE（0 W）。A（9.5 W → 電池から 10 W）を 600 s だけ → DoD = 6000 J / 360 000 J
    let mut i = with_loads(input(
        Vec::new(),
        vec![mode_case("IDLE", None), mode_case("A", Some(600.0))],
    ));
    i.base_mode = Some("IDLE".to_owned());
    let out = EnergyBalance::run(&i).unwrap();
    let a = &out.cases[1];
    assert_eq!(a.evaluation, EnergyBalanceEvaluation::MaxDuration);
    let d = a.by_max_duration.as_ref().unwrap();
    assert_eq!(d.base_mode, "IDLE");
    let expect = from_battery(9.5) * 600.0 / 360_000.0;
    assert!((d.max_dod.value() - expect).abs() < 1e-9, "{:?}", d.max_dod);
    // 発電が無ければ開始によらない
    assert!((d.best_max_dod.value() - expect).abs() < 1e-9);
    assert_eq!(d.base_max_dod.value(), 0.0);
    // 連続（2 周）なら上限 30 % を超えるが、最長継続時間で判定すると OK
    assert!(a.max_dod.value() > 0.3 && d.max_dod.value() < a.max_dod.value());
    let verdict = |k: &str| a.checks.iter().find(|x| x.key == k).unwrap().verdict;
    assert_eq!(verdict("dod"), Verdict::Ok);
    // 1 周に 1 回入れた収支は −6000 J
    let margin = d.energy_margin_per_orbit.unwrap().value();
    assert!((margin + 6000.0).abs() < 1e-6, "{margin}");
    assert_eq!(verdict("energy_balance"), Verdict::Violation);
    // 最長継続時間の無い IDLE は連続のまま
    assert_eq!(out.cases[0].evaluation, EnergyBalanceEvaluation::Continuous);
}

#[test]
fn worst_start_puts_the_window_in_eclipse() {
    // +Z 面 1 m² の太陽指向。日照中は発電が負荷を大きく上回るので、最悪は窓がまるごと日陰に入る開始。
    // 基本のモード IDLE（0 W）の上で B（19 W → 電池から 20 W）を 600 s → DoD = 12 000 J / 360 000 J
    let mut i = with_loads(input(vec![pz_panel()], vec![mode_case("B", Some(600.0))]));
    i.sequence = vec![segment("IDLE", 3000.0), segment("A", 600.0)];
    let out = EnergyBalance::run(&i).unwrap();
    let c = &out.cases[0];
    let d = c.by_max_duration.as_ref().unwrap();
    assert_eq!(d.base_mode, "IDLE"); // 並びの最初の区間
    let expect = from_battery(19.0) * 600.0 / 360_000.0;
    assert!((d.max_dod.value() - expect).abs() < 1e-9, "{:?}", d.max_dod);
    assert!((d.eclipse_overlap.value() - 600.0).abs() < 1e-6);
    // 日照中に置けば DoD は 0、連続で回せば日陰ごとに 20 W を引く
    assert!(d.best_max_dod.value() < 1e-12);
    assert!(c.max_dod.value() > d.max_dod.value());
    assert!(d.starts > 100);
    assert!(d.worst_start_in_orbit.value() < out.period.value());
}

#[test]
fn modes_without_a_base_or_being_the_base_stay_continuous() {
    let mut i = with_loads(input(
        Vec::new(),
        vec![mode_case("A", Some(600.0)), mode_case("IDLE", Some(600.0))],
    ));
    let out = EnergyBalance::run(&i).unwrap();
    assert_eq!(out.cases[0].evaluation, EnergyBalanceEvaluation::NoBaseMode);
    assert!(out.cases[0].by_max_duration.is_none());
    let dod = out.cases[0].checks.iter().find(|c| c.key == "dod").unwrap();
    assert!(
        dod.message
            .contains("when sustained over the whole analysis span"),
        "{}",
        dod.message
    );
    i.base_mode = Some("IDLE".to_owned());
    let out = EnergyBalance::run(&i).unwrap();
    assert_eq!(out.cases[1].evaluation, EnergyBalanceEvaluation::BaseMode);
    assert_eq!(
        out.cases[0].evaluation,
        EnergyBalanceEvaluation::MaxDuration
    );
    assert!(out.sequence.is_none());
}

#[test]
fn duty_is_reported_as_averaged() {
    // DUTY は OBC を半分 on → 平均 4.75 W を一定にかける
    let i = with_loads(input(
        Vec::new(),
        vec![mode_case("DUTY", None), mode_case("A", None)],
    ));
    let out = EnergyBalance::run(&i).unwrap();
    assert_eq!(out.cases[0].duty, EnergyBalanceDuty::Averaged);
    assert!((out.cases[0].load_sunlit.value() - 4.75).abs() < 1e-12);
    assert_eq!(out.cases[1].duty, EnergyBalanceDuty::NotUsed);
}

#[test]
fn invalid_sequence_and_duration_are_rejected() {
    let mut i = with_loads(input(Vec::new(), vec![mode_case("A", Some(0.0))]));
    i.base_mode = Some("IDLE".to_owned());
    assert!(EnergyBalance::run(&i).is_err());
    i.cases.clear();
    i.sequence = vec![segment("A", -1.0)];
    assert!(EnergyBalance::run(&i).is_err());
    i.sequence = vec![segment("NOPE", 60.0)];
    assert!(EnergyBalance::run(&i).is_err());
    i.sequence.clear();
    i.base_mode = Some("NOPE".to_owned());
    assert!(EnergyBalance::run(&i).is_err());
}

/// 電池の値の不一致は、ケースと並びがあっても出力全体で 1 つだけ（判定を平らにしたときに重ならない）。
#[test]
fn battery_mismatch_appears_once_with_cases_and_a_sequence() {
    let mut i = with_loads(input(
        Vec::new(),
        vec![mode_case("A", None), mode_case("B", None)],
    ));
    i.sequence = vec![segment("A", 600.0), segment("B", 300.0)];
    let mut other = i.batteries[0].clone();
    other.voltage = Qty::from_raw(16.0);
    i.batteries.push(other);
    let out = EnergyBalance::run(&i).unwrap();
    assert!(out.sequence.is_some());
    let json = serde_json::to_string(&out).unwrap();
    assert_eq!(json.matches("\"battery_mismatch\"").count(), 1);
    assert_eq!(out.checks[0].key, "battery_mismatch");
}
