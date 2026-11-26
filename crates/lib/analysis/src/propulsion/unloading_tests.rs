//! RCS でのアンロードの推薬を、区間（フェーズ）の長さを通して手計算と比べる。

use super::tests::{input, maneuver};
use super::*;
use crate::aocs::RwUnloadingInput;
use crate::aocs::disturbance::{DisturbanceSpacecraft, FiringThruster, ThrustDisturbance};
use crate::aocs::rw_unloading::RwUnloadingRcsInput;
use crate::aocs::thrusters::Thruster;
use crate::aocs::wheels::RwWheel;
use crate::power::testkit::{leo_timeline, nadir_pointing_z};
use crate::spec::{Analysis, Verdict};
use sat_astro::constants::G0;
use sat_astro::math::{Mat3, Vec3};
use sat_units::typed::{Fraction, Qty};

const DAY: f64 = 86_400.0;
const MASS: f64 = 10.0;

fn rcs_thruster(id: &str, position: Vec3, direction: Vec3) -> Thruster {
    Thruster {
        id: id.to_owned(),
        thrust: Qty::from_raw(0.01),
        isp: Qty::from_raw(200.0),
        position,
        direction,
    }
}

/// 機体の各軸 ±x・±y・±z のトルクを 1 本ずつが出す 6 本（腕 0.1 m、推力 10 mN、Isp 200 s）。
fn six_axis_rcs() -> Vec<Thruster> {
    [
        ("XP", Vec3::Y, Vec3::Z),
        ("XM", Vec3::Y, -Vec3::Z),
        ("YP", Vec3::Z, Vec3::X),
        ("YM", Vec3::Z, -Vec3::X),
        ("ZP", Vec3::X, Vec3::Y),
        ("ZM", Vec3::X, -Vec3::Y),
    ]
    .iter()
    .map(|(id, p, d)| rcs_thruster(id, *p * 0.1, *d))
    .collect()
}

/// 推力誤差だけが 1 µN·m で一定に溜まる機体（ナディア指向。軌道面の法線まわりで、慣性空間でも向きが変わらない）。
/// 10 mN のスラスタの推力の線が重心から 0.01 m ずれ、時間の 1 % 噴射する: τ = 0.01 N × 0.01 m × 0.01 = 1e-6 N·m。
fn rw_input(rcs: Option<Vec<Thruster>>) -> RwUnloadingInput {
    let firing = ThrustDisturbance {
        center_of_mass: Vec3::new(-0.01, 0.0, 0.0),
        firing: vec![FiringThruster {
            thruster: rcs_thruster("T", Vec3::new(0.0, 0.0, -0.1), Vec3::Z),
            firing_fraction: Fraction::new(0.01),
        }],
    };
    RwUnloadingInput {
        timeline: leo_timeline(nadir_pointing_z(), 2.0, 10.0),
        spacecraft: DisturbanceSpacecraft {
            inertia: Mat3::diagonal(Vec3::new(0.05, 0.05, 0.05)),
            aero: None,
            srp: None,
            geometry: None,
            residual_dipole: None,
            atmosphere: Default::default(),
            density_scale: None,
            thrust: Some(firing),
        },
        wheels: [("RW_X", Vec3::X), ("RW_Y", Vec3::Y), ("RW_Z", Vec3::Z)]
            .iter()
            .map(|(id, axis)| RwWheel {
                id: (*id).to_owned(),
                axis: *axis,
                max_momentum: Qty::from_raw(0.02),
                max_torque: Qty::from_raw(0.002),
            })
            .collect(),
        usable_momentum_fraction: None,
        required_unload_interval: None,
        rcs: rcs.map(|thrusters| RwUnloadingRcsInput {
            thrusters,
            center_of_mass: Vec3::ZERO,
            mass: Qty::from_raw(MASS),
        }),
    }
}

fn leg(phase: &str, days: Option<f64>, rcs: Option<Vec<Thruster>>) -> UnloadingLeg {
    UnloadingLeg {
        phase: Some(phase.to_owned()),
        rw_unloading: rw_input(rcs),
        duration: days.map(|d| Qty::from_raw(d * DAY)),
    }
}

fn close(a: f64, b: f64, rel: f64) {
    assert!((a - b).abs() <= rel * b.abs(), "{a} vs {b}");
}

/// 1e-6 N·m を 100 日: ΔH = 1e-6 × 100 × 86 400 = 8.64 N·m·s。腕 0.1 m・Isp 200 s なら
/// m = ΔH / (L · Isp · g₀) = 8.64 / (0.1 × 200 × 9.80665) = 4.405e-2 kg。
#[test]
fn hundred_days_of_a_micro_newton_metre_by_hand() {
    let out = unloading::contribution(&UnloadingInput {
        legs: vec![leg("CRUISE", Some(100.0), Some(six_axis_rcs()))],
    })
    .unwrap();
    let u = out.unloading.unwrap();
    close(u.dumped_momentum.value(), 8.64, 1e-6);
    let expect = 8.64 / (0.1 * 200.0 * G0);
    close(u.propellant.value(), expect, 1e-6);
    assert!((expect - 4.405e-2).abs() < 1e-5);
    // ΔV 換算は 1 日分 v_e ln(m₀ / (m₀ − m_day)) に日数を掛ける（日数に比例とみなす。m_day = m / 100）
    let per_day = 200.0 * G0 * (MASS / (MASS - expect / 100.0)).ln();
    close(u.delta_v.value(), per_day * 100.0, 1e-6);
    assert_eq!(u.legs[0].worst_direction.as_deref().map(str::len), Some(2));
    assert!(out.checks.is_empty() && out.note.is_none());
}

#[test]
fn legs_add_and_a_leg_without_duration_is_listed_but_not_counted() {
    let out = unloading::contribution(&UnloadingInput {
        legs: vec![
            leg("A", Some(100.0), Some(six_axis_rcs())),
            leg("B", None, Some(six_axis_rcs())),
            leg("C", Some(50.0), Some(six_axis_rcs())),
        ],
    })
    .unwrap();
    let u = out.unloading.unwrap();
    assert_eq!(u.legs.len(), 3);
    assert!(u.legs[0].counted && !u.legs[1].counted && u.legs[2].counted);
    assert!(u.legs[1].propellant.is_none() && u.legs[1].duration.is_none());
    // 推薬は日数に比例する: 100 日 + 50 日 = 150 日分
    close(u.covered_duration.value(), 150.0 * DAY, 1e-12);
    close(u.propellant.value(), 1.5 * 8.64 / (0.1 * 200.0 * G0), 1e-6);
    close(
        u.propellant.value(),
        u.legs[0].propellant.unwrap().value() + u.legs[2].propellant.unwrap().value(),
        1e-12,
    );
    assert!(out.checks.is_empty());
}

#[test]
fn a_leg_without_a_thruster_for_the_direction_is_a_warning_not_a_zero() {
    let out = unloading::contribution(&UnloadingInput {
        legs: vec![
            leg("A", Some(100.0), Some(six_axis_rcs())),
            leg("B", Some(10.0), Some(six_axis_rcs()[..1].to_vec())),
        ],
    })
    .unwrap();
    let u = out.unloading.unwrap();
    assert!(u.legs[0].counted && !u.legs[1].counted);
    close(u.propellant.value(), 8.64 / (0.1 * 200.0 * G0), 1e-6);
    assert_eq!(out.checks.len(), 1);
    assert_eq!(out.checks[0].verdict, Verdict::Warning);
    assert!(out.checks[0].message.contains("Phase B"));
    // どの区間も捨てられなければ、足さずに理由
    let none = unloading::contribution(&UnloadingInput {
        legs: vec![leg("B", Some(10.0), Some(six_axis_rcs()[..1].to_vec()))],
    })
    .unwrap();
    assert!(none.unloading.is_none());
    assert_eq!(none.note, Some(UnloadingNote::NotUnloadable));
    assert_eq!(none.checks.len(), 1);
}

#[test]
fn no_duration_anywhere_and_no_thrusters_give_notes() {
    let no_duration = unloading::contribution(&UnloadingInput {
        legs: vec![leg("A", None, Some(six_axis_rcs()))],
    })
    .unwrap();
    assert_eq!(no_duration.note, Some(UnloadingNote::NoMissionDuration));
    // スラスタ（RCS）が無い（磁気トルカでのアンロード）→ 推薬の行は増えない
    let none = unloading::contribution(&UnloadingInput {
        legs: vec![leg("A", Some(100.0), None)],
    })
    .unwrap();
    assert!(none.unloading.is_none());
    assert_eq!(none.note, Some(UnloadingNote::NoRcs));
}

#[test]
fn dv_budget_total_and_margin_include_the_computed_unloading() {
    let mut i = input(vec![maneuver(
        "KEEP",
        DvCategory::Unloading,
        TransferMethod::Impulsive,
        550.0,
    )]);
    let written = DvBudget::run(&i).unwrap();
    let written_dv = written.rows[0].delta_v.value();
    assert!(!written.rows[0].includes_rcs_unloading);
    i.unloading = Some(UnloadingInput {
        legs: vec![leg("CRUISE", Some(100.0), Some(six_axis_rcs()))],
    });
    i.unloading_note = None;
    let out = DvBudget::run(&i).unwrap();
    let u = out.unloading.as_ref().unwrap();
    // 書かれたマヌーバがあれば、同じ区分の行で計算値が足される（置き換えない）
    let row = &out.rows[0];
    assert_eq!(row.category, DvCategory::Unloading);
    assert!(row.includes_rcs_unloading);
    close(row.delta_v.value(), written_dv + u.delta_v.value(), 1e-12);
    close(
        out.total.value(),
        written.total.value() + u.delta_v.value(),
        1e-12,
    );
    close(
        out.total_with_margin.value(),
        out.total.value() * (1.0 + out.margin.value()),
        1e-12,
    );
    assert!(out.unloading_note.is_none());
}

/// アンロードの推薬をタンクに入れる: 100 日の推薬 `m = 8.64 / (0.1 · 200 · g₀)`（上と同じ手計算）を、噴射する 2 本（同じ推力・Isp なので
/// 流量が等しい）が名指した別々のタンクへ半分ずつ。残る推薬 3 % を見込んで積む量は `m/2 · 1.03` ずつ。RCS がタンクを名指さなければ、
/// アンロードの推薬はどのタンクにも入らない（これまでと同じ）。
#[test]
fn unloading_propellant_goes_to_the_tanks_the_firing_thrusters_name() {
    let mut i = input(vec![maneuver(
        "KEEP",
        DvCategory::Unloading,
        TransferMethod::Impulsive,
        550.0,
    )]);
    i.tanks[0].id = "MAIN".to_owned();
    let mut second = i.tanks[0].clone();
    second.id = "RCS_B".to_owned();
    let mut first = i.tanks[0].clone();
    first.id = "RCS_A".to_owned();
    i.tanks.extend([first, second]);
    // 各軸のスラスタを 2 本ずつにする（同じ向きの 2 本が同時に噴射する。推薬は本数によらず `ΔH / (腕 · Isp · g₀)`）
    let doubled: Vec<Thruster> = six_axis_rcs()
        .into_iter()
        .flat_map(|t| {
            let twin = Thruster {
                id: format!("{}B", t.id),
                ..t.clone()
            };
            [t, twin]
        })
        .collect();
    i.unloading = Some(UnloadingInput {
        legs: vec![leg("CRUISE", Some(100.0), Some(doubled.clone()))],
    });
    i.unloading_note = None;
    let without = Propellant::run(&i).unwrap();
    assert_eq!(without.unloading_propellant.value(), 0.0);
    assert!(
        without
            .tanks
            .iter()
            .all(|t| t.unloading_propellant.value() == 0.0)
    );
    // 噴射する 2 本の ID
    let firing = crate::aocs::RwUnloading::run(&rw_input(Some(doubled)))
        .unwrap()
        .rcs_unloading
        .unwrap()
        .per_day
        .thrusters;
    assert_eq!(firing.len(), 2);
    i.thruster_tanks = [
        (firing[0].clone(), "RCS_A".to_owned()),
        (firing[1].clone(), "RCS_B".to_owned()),
    ]
    .into_iter()
    .collect();
    let out = Propellant::run(&i).unwrap();
    let m = 8.64 / (0.1 * 200.0 * G0);
    let each = m / 2.0 * 1.03;
    for id in ["RCS_A", "RCS_B"] {
        let t = out.tanks.iter().find(|t| t.id == id).unwrap();
        close(t.unloading_propellant.value(), each, 1e-6);
        close(t.loaded.value(), each, 1e-6);
        assert_eq!(t.thrusters.len(), 1);
    }
    close(out.unloading_propellant.value(), m * 1.03, 1e-6);
    // 総量: マヌーバの分は変わらず、アンロードの積む分が足される
    let added = out.unloading_propellant.value();
    close(
        out.loaded_propellant.value(),
        without.loaded_propellant.value() + added,
        1e-12,
    );
    close(
        out.wet_mass.value(),
        without.wet_mass.value() + added,
        1e-12,
    );
    // 主タンクはマヌーバの推薬だけ（RCS のタンクが名指されているので、名指さないスラスタは主タンクから取る）
    let main = out.tanks.iter().find(|t| t.id == "MAIN").unwrap();
    assert_eq!(main.unloading_propellant.value(), 0.0);
}

/// RCS のスラスタがタンクを名指しているのに、アンロードの入力が無ければ、推薬は数えず警告する。
#[test]
fn a_named_tank_without_unloading_inputs_is_warned() {
    let mut i = input(vec![maneuver(
        "KEEP",
        DvCategory::Unloading,
        TransferMethod::Impulsive,
        550.0,
    )]);
    i.thruster_tanks = [("XP".to_owned(), "TANK".to_owned())].into();
    let out = Propellant::run(&i).unwrap();
    assert_eq!(out.unloading_propellant.value(), 0.0);
    assert!(
        out.checks
            .iter()
            .any(|c| c.key == "unloading_tank" && c.verdict == Verdict::Warning)
    );
}
