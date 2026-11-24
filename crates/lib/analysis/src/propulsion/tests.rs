//! 推進系の解析を手計算（ロケット方程式・ホーマン・Edelbaum）と比べる。

use super::*;
use crate::spec::{Analysis, Verdict};
use sat_astro::constants::{MU_EARTH, R_EARTH};
use sat_astro::orbit::OrbitSpec;
use sat_astro::rocket::exhaust_velocity;
use sat_astro::time::Epoch;
use sat_astro::transfer::{edelbaum, hohmann};
use sat_units::typed::{Fraction, Qty};

pub(super) const DRY: f64 = 10.0;

fn close(a: f64, b: f64, tol: f64) {
    assert!(
        (a - b).abs() <= tol,
        "difference between {a} and {b} exceeds {tol}"
    );
}

fn thruster(id: &str, thrust: f64, isp: f64) -> PropulsionThruster {
    PropulsionThruster {
        id: id.to_owned(),
        thrust: Qty::from_raw(thrust),
        isp: Qty::from_raw(isp),
        propellant: Some("hydrazine".to_owned()),
        min_impulse_bit: None,
        rated_total_impulse: None,
        rated_firing_time: None,
        rated_firings: None,
        firing_power: None,
    }
}

pub(super) fn maneuver(
    id: &str,
    category: DvCategory,
    method: TransferMethod,
    alt_km: f64,
) -> ManeuverInput {
    ManeuverInput {
        id: id.to_owned(),
        category,
        method,
        thruster: Some("CHEM".to_owned()),
        target: OrbitTarget {
            altitude: Some(Qty::from_raw(alt_km * 1e3)),
            ..OrbitTarget::default()
        },
        via_altitude: None,
        delta_v: None,
        ignored_constraints: Vec::new(),
        thrust: ThrustConstraints::default(),
        transfer: None,
    }
}

/// 高度 400 km・傾斜角 97° の円軌道、乾燥質量 10 kg、化学推進 1 N・220 s、1 L のタンク（1000 kg/m³）。
pub(super) fn input(maneuvers: Vec<ManeuverInput>) -> PropulsionInput {
    let epoch = Epoch::parse("2027-04-01T00:00:00Z").unwrap();
    PropulsionInput {
        initial_orbit: OrbitSpec::circular(
            epoch,
            Qty::from_raw(400e3),
            Qty::from_raw(97f64.to_radians()),
            Qty::ZERO,
        ),
        dry_mass: Qty::from_raw(DRY),
        thrusters: vec![thruster("CHEM", 1.0, 220.0), thruster("EP", 0.01, 1500.0)],
        tanks: vec![PropellantTank {
            id: "TANK".to_owned(),
            volume: Qty::from_raw(1e-3),
            propellant: Some("hydrazine".to_owned()),
            density: Some(Qty::from_raw(1000.0)),
        }],
        maneuvers,
        dv_margin: None,
        residual_fraction: None,
        unloading: None,
        unloading_note: Some(UnloadingNote::NoRcs),
    }
}

pub(super) fn raise() -> ManeuverInput {
    maneuver(
        "RAISE",
        DvCategory::Insertion,
        TransferMethod::Impulsive,
        550.0,
    )
}

fn deorbit() -> ManeuverInput {
    ManeuverInput {
        target: OrbitTarget {
            perigee_altitude: Some(Qty::from_raw(250e3)),
            ..OrbitTarget::default()
        },
        ..maneuver(
            "DEORBIT",
            DvCategory::Deorbit,
            TransferMethod::Impulsive,
            0.0,
        )
    }
}

/// ホーマン 400 → 550 km と、乾燥質量から逆に求めた推薬 `m_p = m_dry (e^(Δv/v_e) − 1)`。
#[test]
fn impulsive_raise_matches_hohmann_and_the_rocket_equation() {
    let out = Maneuvers::run(&input(vec![raise()])).unwrap();
    let m = &out.maneuvers[0];
    let h = hohmann(MU_EARTH, R_EARTH + 400e3, R_EARTH + 550e3).unwrap();
    close(m.delta_v.value(), h.total_m_s(), 1e-9);
    close(m.delta_v.value(), 83.5, 0.1);
    assert_eq!(m.burns.len(), 2);
    let ve = exhaust_velocity(220.0);
    let mp = DRY * ((m.delta_v.value() / ve).exp() - 1.0);
    close(m.propellant.unwrap().value(), mp, 1e-9);
    close(m.mass_before.value(), DRY + mp, 1e-9);
    close(m.mass_after.value(), DRY, 1e-9);
    close(m.after.perigee_altitude.value(), 550e3, 1e-6);
    // 噴射時間 = m_p v_e / F、遷移の時間 = 遷移楕円の半周
    close(m.burn_time.unwrap().value(), mp * ve, 1e-6);
    close(m.transfer_time.unwrap().value(), h.coast_s, 1e-9);
}

/// 2 つのマヌーバは質量を減らしながら順に当てる。同じ v_e なら推薬の合計は `m_dry (e^(ΣΔv/v_e) − 1)`。
#[test]
fn maneuvers_apply_in_order_with_decreasing_mass() {
    let out = Maneuvers::run(&input(vec![raise(), deorbit()])).unwrap();
    let [a, b] = [&out.maneuvers[0], &out.maneuvers[1]];
    close(a.mass_after.value(), b.mass_before.value(), 1e-12);
    assert!(a.mass_before.value() > b.mass_before.value());
    // 離脱は 550 km の円軌道から近地点 250 km（1 回噴射）
    close(b.before.apogee_altitude.value(), 550e3, 1e-6);
    close(b.after.perigee_altitude.value(), 250e3, 1e-6);
    assert_eq!(b.burns.len(), 1);
    let ve = exhaust_velocity(220.0);
    let total = a.delta_v.value() + b.delta_v.value();
    close(out.total_delta_v.value(), total, 1e-9);
    close(
        out.total_propellant.value(),
        DRY * ((total / ve).exp() - 1.0),
        1e-9,
    );
    close(
        out.initial_mass.value(),
        DRY + out.total_propellant.value(),
        1e-9,
    );
}

/// 低推力: Edelbaum の ΔV（高度 400 → 550 km と傾斜角 0.5°）、遷移の時間は連続噴射の時間 `m_p v_e / F`。
#[test]
fn low_thrust_uses_edelbaum() {
    let mut m = maneuver(
        "SPIRAL",
        DvCategory::Insertion,
        TransferMethod::LowThrust,
        550.0,
    );
    m.thruster = Some("EP".to_owned());
    m.target.inclination = Some(Qty::from_raw(97.5f64.to_radians()));
    let out = Maneuvers::run(&input(vec![m])).unwrap();
    let r = &out.maneuvers[0];
    let hand = edelbaum(
        MU_EARTH,
        R_EARTH + 400e3,
        R_EARTH + 550e3,
        0.5f64.to_radians(),
    )
    .unwrap();
    close(r.delta_v.value(), hand, 1e-9);
    let ve = exhaust_velocity(1500.0);
    let mp = DRY * ((hand / ve).exp() - 1.0);
    close(r.propellant.unwrap().value(), mp, 1e-9);
    close(r.transfer_time.unwrap().value(), mp * ve / 0.01, 1e-3);
    assert!(r.notes.is_empty());
}

/// 有限燃焼は推力の小さいほど損失が増え、半周を超える噴射は警告してインパルスの ΔV を使う。
#[test]
fn finite_burn_adds_a_loss() {
    let mut m = raise();
    m.method = TransferMethod::FiniteBurn;
    let quick = Maneuvers::run(&input(vec![m.clone()])).unwrap();
    let r = &quick.maneuvers[0];
    assert!(r.delta_v.value() > r.ideal_delta_v.value());
    assert!(r.delta_v.value() < r.ideal_delta_v.value() * 1.01);
    m.thruster = Some("EP".to_owned());
    let slow = Maneuvers::run(&input(vec![m])).unwrap();
    let r = &slow.maneuvers[0];
    assert!(r.notes.contains(&ManeuverNote::FiniteBurnTooLong));
    close(r.delta_v.value(), r.ideal_delta_v.value(), 1e-12);
    assert_eq!(r.checks[0].verdict, Verdict::Warning);
}

/// スラスタの無いマヌーバは ΔV だけ（推薬は求めず、質量を変えない）。
#[test]
fn maneuvers_without_a_thruster_give_delta_v_only() {
    let mut m = raise();
    m.thruster = None;
    let inp = input(vec![m]);
    let out = Maneuvers::run(&inp).unwrap();
    let r = &out.maneuvers[0];
    assert!(r.propellant.is_none() && r.burn_time.is_none());
    assert_eq!(r.notes, vec![ManeuverNote::NoThruster]);
    close(out.initial_mass.value(), DRY, 1e-12);
    let p = Propellant::run(&inp).unwrap();
    assert_eq!(p.maneuvers_without_thruster, vec!["RAISE".to_owned()]);
    assert_eq!(p.checks[0].verdict, Verdict::Warning);
}

/// ΔV の内訳: 区分ごとに足し、合計に既定のマージン 10 % を掛ける。アンロードを足せない理由はそのまま出す。
#[test]
fn dv_budget_sums_by_category_with_the_default_margin() {
    let out = DvBudget::run(&input(vec![raise(), deorbit()])).unwrap();
    let cats: Vec<DvCategory> = out.rows.iter().map(|r| r.category).collect();
    assert_eq!(cats, [DvCategory::Insertion, DvCategory::Deorbit]);
    let total: f64 = out.rows.iter().map(|r| r.delta_v.value()).sum();
    close(out.total.value(), total, 1e-12);
    close(out.margin.value(), DEFAULT_DV_MARGIN, 1e-12);
    close(out.total_with_margin.value(), total * 1.1, 1e-9);
    close(
        out.rows.iter().map(|r| r.share.value()).sum::<f64>(),
        1.0,
        1e-12,
    );
    assert_eq!(out.unloading_note, Some(UnloadingNote::NoRcs));
    assert!(out.unloading.is_none());
}

/// 推薬: マージン込みの ΔV で、残る推薬 r を見込む。1 回のマヌーバなら
/// `k = e^(1.1 Δv / v_e)`、最後の質量 `m_end = m_dry / (1 − r (k − 1))`、ウェット `m_end k`。
#[test]
fn propellant_with_margin_and_residual_by_hand() {
    let inp = input(vec![raise()]);
    let out = Propellant::run(&inp).unwrap();
    let dv = Maneuvers::run(&inp).unwrap().total_delta_v.value();
    let k = (1.1 * dv / exhaust_velocity(220.0)).exp();
    let r = DEFAULT_RESIDUAL_FRACTION;
    let end = DRY / (1.0 - r * (k - 1.0));
    close(out.wet_mass.value(), end * k, 1e-9);
    close(out.usable_propellant.value(), end * (k - 1.0), 1e-9);
    close(out.residual_propellant.value(), r * end * (k - 1.0), 1e-9);
    close(
        out.loaded_propellant.value(),
        out.wet_mass.value() - DRY,
        1e-9,
    );
    // タンク 1 L・1000 kg/m³ に対して積む推薬は約 0.4 kg → 余裕は大きい
    let g = &out.groups[0];
    assert_eq!(g.propellant.as_deref(), Some("hydrazine"));
    close(g.required.value(), out.loaded_propellant.value(), 1e-9);
    close(
        g.required_volume.unwrap().value(),
        g.required.value() / 1000.0,
        1e-12,
    );
    close(
        g.volume_margin.unwrap().value(),
        1e-3 / g.required_volume.unwrap().value() - 1.0,
        1e-9,
    );
    assert_eq!(g.checks[0].verdict, Verdict::Ok);
}

/// タンクが小さければ容積の判定が violation。
#[test]
fn a_small_tank_is_a_violation() {
    let mut inp = input(vec![raise()]);
    inp.tanks[0].volume = Qty::from_raw(1e-4);
    let out = Propellant::run(&inp).unwrap();
    assert_eq!(out.groups[0].checks[0].verdict, Verdict::Violation);
    assert!(out.groups[0].volume_margin.unwrap().value() < 0.0);
}

/// スラスタの総インパルス = Σ m_p v_e = 推力 × 噴射時間。定格の余裕を判定する。
#[test]
fn thruster_usage_against_ratings() {
    let mut inp = input(vec![raise(), deorbit()]);
    inp.thrusters[0].rated_total_impulse = Some(Qty::from_raw(1000.0));
    inp.thrusters[0].rated_firings = Some(2);
    inp.dv_margin = Some(Fraction::new(0.0));
    inp.residual_fraction = Some(Fraction::new(0.0));
    let out = ThrusterUsage::run(&inp).unwrap();
    let chem = &out.thrusters[0];
    let p = Propellant::run(&inp).unwrap();
    let ve = exhaust_velocity(220.0);
    close(
        chem.total_impulse.value(),
        p.usable_propellant.value() * ve,
        1e-6,
    );
    assert_eq!(chem.firings, 3);
    assert_eq!(chem.maneuvers, ["RAISE", "DEORBIT"]);
    // 総インパルス ≈ 0.75 kg × 2157 m/s ≈ 1.6 kN·s > 定格 1 kN·s、噴射 3 回 > 定格 2 回
    let verdicts: Vec<Verdict> = chem.checks.iter().map(|c| c.verdict).collect();
    assert_eq!(verdicts, [Verdict::Violation, Verdict::Violation]);
    let ep = &out.thrusters[1];
    assert_eq!(ep.firings, 0);
    assert!(ep.checks.is_empty());
}

/// 月の高度 100 km の円軌道へ（目標の天体 `body = moon`）: 400 km の LEO から TLI と LOI の 2 回噴射
/// （`sat_astro::transfer::patched_conic` と同じ値。TLI ≈ 3.1 km/s・LOI ≈ 0.82 km/s）。
/// そのあとのマヌーバは月のまわりで計算する（μ は月。100 → 50 km の近月点下げ）。
#[test]
fn transfer_to_the_moon_then_maneuver_about_the_moon() {
    use sat_astro::constants::R_MOON;
    use sat_astro::ephem::CentralBody;
    use sat_astro::transfer::{Apsides, apse_transfer, patched_conic};
    let tli = ManeuverInput {
        target: OrbitTarget {
            body: Some(CentralBody::Moon),
            altitude: Some(Qty::from_raw(100e3)),
            ..OrbitTarget::default()
        },
        ..maneuver("TLI", DvCategory::Insertion, TransferMethod::Impulsive, 0.0)
    };
    let lower = ManeuverInput {
        target: OrbitTarget {
            perigee_altitude: Some(Qty::from_raw(50e3)),
            ..OrbitTarget::default()
        },
        ..maneuver("LOWER", DvCategory::Other, TransferMethod::Impulsive, 0.0)
    };
    let mut i = input(vec![tli, lower]);
    i.thrusters[0].isp = Qty::from_raw(320.0);
    let out = Maneuvers::run(&i).unwrap();
    let [a, b] = [&out.maneuvers[0], &out.maneuvers[1]];
    let llo = Apsides::circular(R_MOON + 100e3);
    let pc = patched_conic(
        CentralBody::Earth,
        Apsides::circular(R_EARTH + 400e3),
        CentralBody::Moon,
        Some(llo),
    )
    .unwrap();
    close(a.delta_v.value(), pc.total_m_s(), 1e-9);
    close(a.burns[0].delta_v.value() / 1e3, 3.1, 0.05);
    close(a.burns[1].delta_v.value() / 1e3, 0.82, 0.02);
    assert_eq!(a.burns[0].central, None);
    assert_eq!(a.burns[1].central, Some(CentralBody::Moon));
    close(a.burns[1].altitude.value(), 100e3, 1e-6);
    assert!(a.burns[1].v_inf.is_some() && a.notes.contains(&ManeuverNote::PatchedConic));
    assert_eq!(a.after.central, Some(CentralBody::Moon));
    close(a.after.perigee_altitude.value(), 100e3, 1e-6);
    close(a.transfer_time.unwrap().value(), pc.transfer_time_s, 1e-6);
    let down = apse_transfer(
        CentralBody::Moon.gm(),
        llo,
        Apsides::new(R_MOON + 50e3, R_MOON + 100e3),
        0.0,
    )
    .unwrap();
    close(b.delta_v.value(), down.total_m_s(), 1e-9);
    assert_eq!(out.final_orbit.central, Some(CentralBody::Moon));
    // ΔV バジェットにも入る（投入の区分）
    let budget = DvBudget::run(&i).unwrap();
    let insertion: f64 = budget
        .rows
        .iter()
        .filter(|r| r.category == DvCategory::Insertion)
        .map(|r| r.delta_v.value())
        .sum();
    close(insertion, pc.total_m_s(), 1e-6);
}

/// 天体間の遷移は低推力では計算せず、捕獲の軌道を書かなければ止まる。
#[test]
fn interbody_transfer_needs_impulsive_and_a_capture_orbit() {
    use sat_astro::ephem::CentralBody;
    let mut m = maneuver(
        "TMI",
        DvCategory::Insertion,
        TransferMethod::LowThrust,
        300.0,
    );
    m.target.body = Some(CentralBody::Mars);
    assert!(Maneuvers::run(&input(vec![m.clone()])).is_err());
    m.method = TransferMethod::Impulsive;
    m.target.altitude = None;
    assert!(Maneuvers::run(&input(vec![m.clone()])).is_err());
    m.target.altitude = Some(Qty::from_raw(300e3));
    let out = Maneuvers::run(&input(vec![m])).unwrap();
    // 400 km の LEO からの火星遷移の出発は ≈ 3.6 km/s（Curtis Example 8.4 は 300 km から 3.59 km/s）
    close(out.maneuvers[0].burns[0].delta_v.value() / 1e3, 3.6, 0.05);
}
