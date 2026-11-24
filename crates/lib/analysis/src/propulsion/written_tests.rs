//! 直接書いた ΔV（余裕）のマヌーバを、ロケット方程式の手計算と比べる。

use super::tests::{DRY, input, maneuver, raise};
use super::*;
use crate::spec::Analysis;
use sat_astro::constants::{MU_EARTH, R_EARTH};
use sat_astro::rocket::exhaust_velocity;
use sat_astro::transfer::hohmann;
use sat_units::typed::Qty;

fn close(a: f64, b: f64, tol: f64) {
    assert!(
        (a - b).abs() <= tol,
        "difference between {a} and {b} exceeds {tol}"
    );
}

/// 書いた ΔV `dv` [m/s] の余裕（軌道は変えない。既定のスラスタは 1 N・220 s の化学推進）。
fn allowance(id: &str, category: DvCategory, dv: f64) -> ManeuverInput {
    ManeuverInput {
        delta_v: Some(Qty::from_raw(dv)),
        target: OrbitTarget::default(),
        ..maneuver(id, category, TransferMethod::Impulsive, 0.0)
    }
}

/// 15 m/s・Isp 220 s・乾燥 10 kg: `v_e = 220 g0`、最後の質量が乾燥質量なので `m0 = 10 e^(15/v_e)`、
/// 推薬は `m0 (1 − e^(−15/v_e))`（= `10 (e^(15/v_e) − 1)`、約 0.0698 kg）。軌道は変わらず、噴射は 1 回。
#[test]
fn a_written_delta_v_gives_the_propellant_of_the_rocket_equation() {
    let out = Maneuvers::run(&input(vec![allowance(
        "TCM",
        DvCategory::StationKeeping,
        15.0,
    )]))
    .unwrap();
    let m = &out.maneuvers[0];
    let ve = exhaust_velocity(220.0);
    close(m.delta_v.value(), 15.0, 1e-12);
    close(m.ideal_delta_v.value(), 15.0, 1e-12);
    let m0 = DRY * (15.0 / ve).exp();
    close(m.mass_before.value(), m0, 1e-9);
    close(
        m.propellant.unwrap().value(),
        m0 * (1.0 - (-15.0 / ve).exp()),
        1e-9,
    );
    close(m.propellant.unwrap().value(), 0.0698, 1e-4);
    close(m.mass_after.value(), DRY, 1e-9);
    assert_eq!(m.burns.len(), 1);
    assert_eq!(m.notes, vec![ManeuverNote::WrittenDeltaV]);
    // 噴射時間 = m_p v_e / F、軌道は変わらない
    close(
        m.burn_time.unwrap().value(),
        m.propellant.unwrap().value() * ve,
        1e-9,
    );
    close(
        m.after.perigee_altitude.value(),
        m.before.perigee_altitude.value(),
        1e-9,
    );
    close(
        m.after.apogee_altitude.value(),
        m.before.apogee_altitude.value(),
        1e-9,
    );
}

/// 2 つの余裕を順に: 2 つ目は 1 つ目のあとの質量から当てる。同じスラスタなら合計は `m_dry (e^((Δv₁+Δv₂)/v_e) − 1)`。
/// 10 m/s と 5 m/s なら 1 つ目の前の質量は `10 e^(15/v_e)`、後は `10 e^(5/v_e)`。
#[test]
fn two_allowances_use_the_mass_after_the_first() {
    let out = Maneuvers::run(&input(vec![
        allowance("A", DvCategory::StationKeeping, 10.0),
        allowance("B", DvCategory::Other, 5.0),
    ]))
    .unwrap();
    let [a, b] = [&out.maneuvers[0], &out.maneuvers[1]];
    let ve = exhaust_velocity(220.0);
    close(a.mass_before.value(), DRY * (15.0 / ve).exp(), 1e-9);
    close(a.mass_after.value(), DRY * (5.0 / ve).exp(), 1e-9);
    close(b.mass_before.value(), a.mass_after.value(), 1e-12);
    close(
        a.propellant.unwrap().value() + b.propellant.unwrap().value(),
        DRY * ((15.0 / ve).exp() - 1.0),
        1e-9,
    );
    close(out.total_delta_v.value(), 15.0, 1e-12);
}

/// 余裕を 2 つのマヌーバの間に入れる: 軌道を変えないので、あとのマヌーバの ΔV と前後の軌道は変わらない。
/// 最後の質量は乾燥質量に決まるので、あとのマヌーバの前の質量は `m_dry e^(Δv_d/v_e)`、余裕の前は `× e^(Δv_a/v_e)`、
/// 最初の推薬は `m_after (e^(Δv_r/v_e) − 1)`（`m_after` は最初のマヌーバのあとの質量）。
#[test]
fn an_allowance_between_maneuvers_follows_the_rocket_equation() {
    let mut deorbit = maneuver(
        "DEORBIT",
        DvCategory::Deorbit,
        TransferMethod::Impulsive,
        0.0,
    );
    deorbit.target.altitude = None;
    deorbit.target.perigee_altitude = Some(Qty::from_raw(250e3));
    let without = Maneuvers::run(&input(vec![raise(), deorbit.clone()])).unwrap();
    let mut tcm = allowance("TCM", DvCategory::StationKeeping, 15.0);
    tcm.thruster = Some("CHEM".to_owned());
    let with = Maneuvers::run(&input(vec![raise(), tcm, deorbit])).unwrap();
    let [r, t, d] = [&with.maneuvers[0], &with.maneuvers[1], &with.maneuvers[2]];
    // あとのマヌーバの ΔV・軌道は同じ
    close(
        d.delta_v.value(),
        without.maneuvers[1].delta_v.value(),
        1e-9,
    );
    close(d.before.apogee_altitude.value(), 550e3, 1e-6);
    close(t.before.apogee_altitude.value(), 550e3, 1e-6);
    close(t.after.apogee_altitude.value(), 550e3, 1e-6);
    // 質量: 後ろから積み上げる
    let ve = exhaust_velocity(220.0);
    let m_before_deorbit = DRY * (d.delta_v.value() / ve).exp();
    close(d.mass_before.value(), m_before_deorbit, 1e-9);
    close(t.mass_after.value(), m_before_deorbit, 1e-9);
    close(
        t.mass_before.value(),
        m_before_deorbit * (15.0 / ve).exp(),
        1e-9,
    );
    close(r.mass_after.value(), t.mass_before.value(), 1e-12);
    let hoh = hohmann(MU_EARTH, R_EARTH + 400e3, R_EARTH + 550e3).unwrap();
    close(r.delta_v.value(), hoh.total_m_s(), 1e-9);
    close(
        r.propellant.unwrap().value(),
        r.mass_after.value() * ((hoh.total_m_s() / ve).exp() - 1.0),
        1e-9,
    );
    // 余裕を入れたぶん、最初のマヌーバの推薬は増える
    assert!(r.propellant.unwrap().value() > without.maneuvers[0].propellant.unwrap().value());
    close(
        with.total_delta_v.value(),
        without.total_delta_v.value() + 15.0,
        1e-9,
    );
}

/// ΔV の内訳は区分の行に足し、既定のマージン 10 % を掛ける（`15 × 1.1 = 16.5` m/s）。
#[test]
fn the_budget_counts_an_allowance_in_its_category() {
    let out = DvBudget::run(&input(vec![allowance(
        "TCM",
        DvCategory::StationKeeping,
        15.0,
    )]))
    .unwrap();
    assert_eq!(out.rows.len(), 1);
    assert_eq!(out.rows[0].category, DvCategory::StationKeeping);
    close(out.total.value(), 15.0, 1e-12);
    close(out.total_with_margin.value(), 16.5, 1e-9);
}

/// 低推力の余裕は連続噴射（噴射時間 = 遷移の時間）、スラスタの無い余裕は ΔV だけ。
#[test]
fn low_thrust_and_thrusterless_allowances() {
    let mut lt = allowance("LT", DvCategory::Other, 15.0);
    lt.method = TransferMethod::LowThrust;
    lt.thruster = Some("EP".to_owned());
    let out = Maneuvers::run(&input(vec![lt])).unwrap();
    let m = &out.maneuvers[0];
    close(m.delta_v.value(), 15.0, 1e-12);
    close(
        m.transfer_time.unwrap().value(),
        m.burn_time.unwrap().value(),
        1e-9,
    );
    let mut none = allowance("NONE", DvCategory::Other, 15.0);
    none.thruster = None;
    let out = Maneuvers::run(&input(vec![none])).unwrap();
    let m = &out.maneuvers[0];
    assert!(m.propellant.is_none());
    assert_eq!(
        m.notes,
        vec![ManeuverNote::WrittenDeltaV, ManeuverNote::NoThruster]
    );
    close(out.initial_mass.value(), DRY, 1e-12);
}

/// 軌道を変える書き方との併用、負の値、数値伝搬は誤り。
#[test]
fn unusable_allowances_are_errors() {
    let mut with_target = allowance("T", DvCategory::Other, 15.0);
    with_target.via_altitude = Some(Qty::from_raw(20e6));
    assert!(Maneuvers::run(&input(vec![with_target])).is_err());
    assert!(Maneuvers::run(&input(vec![allowance("N", DvCategory::Other, -1.0)])).is_err());
    let mut propagated = allowance("P", DvCategory::Other, 15.0);
    propagated.method = TransferMethod::Propagated;
    assert!(Maneuvers::run(&input(vec![propagated])).is_err());
}
