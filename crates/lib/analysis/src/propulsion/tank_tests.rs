//! 推薬のタンクごとの内訳を、ロケット方程式の手計算と比べる（主エンジンと姿勢制御用のスラスタが別のタンク・同じタンクから取る場合）。

use super::tests::{DRY, input, maneuver, raise};
use super::*;
use crate::spec::{Analysis, Verdict};
use crate::structure::{MassPropertiesInput, MassPropertiesItem};
use sat_astro::constants::{MU_EARTH, R_EARTH};
use sat_astro::math::Vec3;
use sat_astro::rocket::exhaust_velocity;
use sat_astro::transfer::hohmann;
use sat_units::typed::{Fraction, Qty};

fn close(a: f64, b: f64, tol: f64) {
    assert!(
        (a - b).abs() <= tol,
        "difference between {a} and {b} exceeds {tol}"
    );
}

/// 主エンジン CHEM（1 N・220 s）の上昇 RAISE（400 → 550 km）のあとに、姿勢制御用の RCSX（0.5 N・150 s）で 20 m/s の補正 TRIM。
/// 主タンク MAIN は 1 L、姿勢制御用のタンク RCS_TANK は 0.2 L（どちらもヒドラジン 1000 kg/m³）。ΔV のマージンも残る推薬も 0。
fn two_consumers() -> PropulsionInput {
    let trim = ManeuverInput {
        delta_v: Some(Qty::from_raw(20.0)),
        target: OrbitTarget::default(),
        thruster: Some("RCSX".to_owned()),
        ..maneuver("TRIM", DvCategory::Other, TransferMethod::Impulsive, 0.0)
    };
    let mut inp = input(vec![raise(), trim]);
    let mut rcsx = inp.thrusters[0].clone();
    rcsx.id = "RCSX".to_owned();
    rcsx.thrust = Qty::from_raw(0.5);
    rcsx.isp = Qty::from_raw(150.0);
    inp.thrusters = vec![inp.thrusters[0].clone(), rcsx];
    let mut rcs_tank = inp.tanks[0].clone();
    rcs_tank.id = "RCS_TANK".to_owned();
    rcs_tank.volume = Qty::from_raw(0.2e-3);
    inp.tanks[0].id = "MAIN".to_owned();
    inp.tanks.push(rcs_tank);
    inp.dv_margin = Some(Fraction::new(0.0));
    inp.residual_fraction = Some(Fraction::new(0.0));
    inp
}

/// 手計算の各消費者の推薬 [kg]: 最後の質量は乾燥質量なので、TRIM は `m₁ = m_dry (e^(20/v_e2) − 1)`、
/// RAISE はその前の質量 `m_dry + m₁` に対して `m₂ = (m_dry + m₁)(e^(Δv/v_e1) − 1)`（Δv はホーマン 400 → 550 km）。
fn by_hand() -> (f64, f64) {
    let dv = hohmann(MU_EARTH, R_EARTH + 400e3, R_EARTH + 550e3)
        .unwrap()
        .total_m_s();
    let trim = DRY * ((20.0 / exhaust_velocity(150.0)).exp() - 1.0);
    let raise = (DRY + trim) * ((dv / exhaust_velocity(220.0)).exp() - 1.0);
    (raise, trim)
}

fn row<'a>(out: &'a PropellantOutput, id: &str) -> &'a PropellantTankRow {
    out.tanks.iter().find(|t| t.id == id).unwrap()
}

/// 別のタンク: 各タンクの積む量は、それを取るスラスタのマヌーバの推薬（ロケット方程式）。合計は 2 つの和で、タンクごとの余裕は
/// `タンクの容積 / (積む量 / 密度) − 1`。
#[test]
fn separate_tanks_hold_each_consumers_propellant() {
    let mut inp = two_consumers();
    inp.thruster_tanks = [("CHEM", "MAIN"), ("RCSX", "RCS_TANK")]
        .iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect();
    let out = Propellant::run(&inp).unwrap();
    let (raise, trim) = by_hand();
    let (main, rcs) = (row(&out, "MAIN"), row(&out, "RCS_TANK"));
    close(main.loaded.value(), raise, 1e-9);
    close(rcs.loaded.value(), trim, 1e-9);
    close(out.loaded_propellant.value(), raise + trim, 1e-9);
    close(
        main.volume_margin.unwrap().value(),
        1e-3 / (raise / 1000.0) - 1.0,
        1e-9,
    );
    close(
        rcs.volume_margin.unwrap().value(),
        0.2e-3 / (trim / 1000.0) - 1.0,
        1e-9,
    );
    close(main.capacity.unwrap().value(), 1.0, 1e-12);
    assert_eq!(main.thrusters, ["CHEM"]);
    assert_eq!(rcs.thrusters, ["RCSX"]);
    assert!(main.named && rcs.named);
    // 名指したタンクの組には、タンクごとの容積の判定が付く
    assert_eq!(main.checks[0].verdict, Verdict::Ok);
    assert!(rcs.unloading_propellant.value() == 0.0);
}

/// 名指さないスラスタは、同じ推薬のうちどのスラスタにも名指されていないタンクから取る（RCSX だけが RCS_TANK を名指す）。
#[test]
fn an_unnamed_thruster_draws_from_the_tanks_nobody_names() {
    let mut inp = two_consumers();
    inp.thruster_tanks = [("RCSX".to_owned(), "RCS_TANK".to_owned())].into();
    let out = Propellant::run(&inp).unwrap();
    let (raise, trim) = by_hand();
    close(row(&out, "MAIN").loaded.value(), raise, 1e-9);
    assert!(!row(&out, "MAIN").named);
    close(row(&out, "RCS_TANK").loaded.value(), trim, 1e-9);
}

/// 同じタンク: 1 つのタンクが 2 つの和を積み、合計は変わらない。名指さない場合（これまでの計算: 同じ推薬のタンクへ容積の比）の合計とも一致する。
#[test]
fn a_shared_tank_holds_the_sum_and_the_total_is_unchanged() {
    let mut inp = two_consumers();
    let unnamed = Propellant::run(&inp).unwrap();
    inp.thruster_tanks = [("CHEM", "MAIN"), ("RCSX", "MAIN")]
        .iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect();
    let out = Propellant::run(&inp).unwrap();
    let (raise, trim) = by_hand();
    close(row(&out, "MAIN").loaded.value(), raise + trim, 1e-9);
    close(row(&out, "RCS_TANK").loaded.value(), 0.0, 0.0);
    assert_eq!(
        out.loaded_propellant.value(),
        unnamed.loaded_propellant.value()
    );
    assert_eq!(out.wet_mass.value(), unnamed.wet_mass.value());
    assert_eq!(
        out.groups[0].required.value(),
        unnamed.groups[0].required.value()
    );
}

/// タンクを名指さない設計の結果は、これまでと同じ: 推薬の名前の組の積む量を、組のタンクへ容積の比（`v / Σv`）で分けた数に等しい（ビットまで）。
#[test]
fn without_named_tanks_the_loads_are_the_old_volume_split() {
    let inp = two_consumers();
    let out = Propellant::run(&inp).unwrap();
    let g = &out.groups[0];
    for (id, v) in [("MAIN", 1e-3), ("RCS_TANK", 0.2e-3)] {
        let share = v / (1e-3 + 0.2e-3);
        assert_eq!(row(&out, id).loaded.value(), g.required.value() * share);
        assert!(!row(&out, id).named && row(&out, id).checks.is_empty());
    }
    assert_eq!(out.unloading_propellant.value(), 0.0);
}

/// 名指したタンクが一覧に無ければ入力の誤り。
#[test]
fn a_tank_that_is_not_in_the_list_is_refused() {
    let mut inp = two_consumers();
    inp.thruster_tanks = [("CHEM".to_owned(), "NOPE".to_owned())].into();
    assert!(Propellant::run(&inp).is_err());
}

fn item(id: &str, mass: f64, z: f64) -> MassPropertiesItem {
    MassPropertiesItem {
        id: id.to_owned(),
        mass: Qty::from_raw(mass),
        count: 1,
        position: Vec3::new(0.0, 0.0, z),
        size: None,
        orientation: None,
        inertia: None,
    }
}

/// 質量特性: 2 つのタンクが別の位置にあれば、各タンクの推薬がそのタンクの位置に点質量として入る。
/// 乾燥 BUS 9 kg が z = 0、MAIN 1 kg が z = 0.1 m、RCS_TANK 0.5 kg が z = −0.2 m、推薬 `m₂`（MAIN）・`m₁`（RCS_TANK）なら
/// `z_cm = (0.1 (1 + m₂) − 0.2 (0.5 + m₁)) / (10.5 + m₂ + m₁)`（手計算）。
#[test]
fn center_of_mass_with_two_tanks_at_different_positions() {
    let mut propulsion = two_consumers();
    propulsion.thruster_tanks = [("CHEM", "MAIN"), ("RCSX", "RCS_TANK")]
        .iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect();
    let structure = MassPropertiesInput {
        items: vec![
            item("BUS", 9.0, 0.0),
            item("MAIN", 1.0, 0.1),
            item("RCS_TANK", 0.5, -0.2),
        ],
        max_mass: None,
        com_box: None,
    };
    let out = PropellantMassProperties::run(&PropellantMassInput {
        structure,
        propulsion,
    })
    .unwrap();
    let (m2, m1) = by_hand();
    let z = (0.1 * (1.0 + m2) - 0.2 * (0.5 + m1)) / (10.5 + m2 + m1);
    close(out.bol.total_mass.value(), 10.5 + m2 + m1, 1e-12);
    close(out.bol.center_of_mass.z, z, 1e-12);
    let load = |id: &str| out.tanks.iter().find(|t| t.id == id).unwrap();
    close(load("MAIN").bol.value(), m2, 1e-9);
    close(load("RCS_TANK").bol.value(), m1, 1e-9);
    assert_eq!(load("RCS_TANK").position.z, -0.2);
}
