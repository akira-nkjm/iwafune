//! 軌道維持に使うスラスタと推薬の決め方（推薬の種類をまたいで換算しない）の検査。

use super::tests::{cubesat_3u, propulsion};
use super::*;
use crate::propulsion::{
    DvCategory, ManeuverInput, OrbitTarget, PropellantTank, PropulsionThruster, ThrustConstraints,
    TransferMethod,
};
use crate::spec::Verdict;
use sat_astro::constants::{MU_EARTH, R_EARTH};
use sat_astro::rocket::exhaust_velocity;
use sat_astro::transfer::hohmann;

const MARGIN: f64 = 1.10;
const RESIDUAL: f64 = 0.03;
const DRY_KG: f64 = 4.0;

fn thruster(id: &str, isp: f64, propellant: Option<&str>) -> PropulsionThruster {
    PropulsionThruster {
        id: id.into(),
        thrust: Qty::from_raw(0.01),
        isp: Qty::from_raw(isp),
        propellant: propellant.map(String::from),
        min_impulse_bit: None,
        rated_total_impulse: None,
        rated_firing_time: None,
        rated_firings: None,
        firing_power: None,
    }
}

fn tank(id: &str, propellant: &str) -> PropellantTank {
    PropellantTank {
        id: id.into(),
        volume: Qty::from_raw(1e-3),
        propellant: Some(propellant.into()),
        density: Some(Qty::from_raw(1000.0)),
    }
}

/// 円軌道の高度を `altitude_m` にするインパルスのマヌーバ。
fn burn(id: &str, category: DvCategory, thruster: Option<&str>, altitude_m: f64) -> ManeuverInput {
    ManeuverInput {
        id: id.into(),
        category,
        method: TransferMethod::Impulsive,
        thruster: thruster.map(String::from),
        target: OrbitTarget {
            altitude: Some(Qty::from_raw(altitude_m)),
            ..OrbitTarget::default()
        },
        via_altitude: None,
        ignored_constraints: Vec::new(),
        thrust: ThrustConstraints::default(),
        transfer: None,
    }
}

/// 円軌道 `h1` → `h2` [m] のホーマン遷移の ΔV [m/s]。
fn dv(h1: f64, h2: f64) -> f64 {
    hohmann(MU_EARTH, R_EARTH + h1, R_EARTH + h2)
        .unwrap()
        .total_m_s()
}

/// 電気推進（EP・Xe・Isp 1500 s）とコールドガス（CG・N2・Isp 60 s）の 2 種を積み、
/// 500 → 520 km を EP で上げる推進系。`extra` のマヌーバを続ける。
fn two_propellants(extra: Vec<ManeuverInput>) -> OrbitDecayInput {
    let mut input = cubesat_3u(500e3);
    input.span = Some(Qty::from_raw(3.0 * YEAR_S));
    let mut p = propulsion(&input.orbit, DRY_KG);
    p.thrusters = vec![
        thruster("EP", 1500.0, Some("Xe")),
        thruster("CG", 60.0, Some("N2")),
    ];
    p.tanks = vec![tank("TXE", "Xe"), tank("TN2", "N2")];
    p.maneuvers = vec![burn("RAISE", DvCategory::Insertion, Some("EP"), 520e3)];
    p.maneuvers.extend(extra);
    input.propulsion = Some(p);
    input
}

fn check<'a>(o: &'a OrbitDecayOutput, key: &str) -> Option<&'a Check> {
    o.checks.iter().find(|c| c.key == key)
}

/// (a) 推薬が 1 種類で軌道維持のマヌーバが無い: 従来どおり、最も低い比推力で推進系の使える推薬の全量を換算する。
/// 軌道上げ 500 → 520 km と廃棄 520 → 400 km を同じスラスタ（Isp 220 s）で行うと、
/// 噴射前の質量をウェット質量にしたロケット方程式は、最後の質量が「乾燥 + 残る推薬」なので
/// `v_e ln(m_wet / (m_wet − m_usable)) = 1.1 × (ΔV₁ + ΔV₂)`（マージン 10 %）に戻る。
#[test]
fn single_propellant_keeps_the_previous_formula() {
    let mut input = cubesat_3u(500e3);
    let mut p = propulsion(&input.orbit, DRY_KG);
    p.maneuvers
        .push(burn("DEORBIT", DvCategory::Deorbit, Some("PROP"), 400e3));
    input.propulsion = Some(p);
    let o = OrbitDecay::run(&input).unwrap();
    let expected = MARGIN * (dv(500e3, 520e3) + dv(520e3, 400e3));
    assert!((o.used.propulsion_delta_v.unwrap().value() - expected).abs() < 1e-6);
    assert_eq!(o.used.thruster.as_deref(), Some("PROP"));
    assert!(check(&o, "station_keeping_thruster").is_none());
}

/// マージン込みで 2 つのマヌーバ（EP の軌道上げ `dv_r`、続く比推力 `isp_k` の軌道維持 `dv_k`）が使う推薬と、噴射前のウェット質量。
/// 最後の質量 `m₂ = 乾燥 + 0.03 (m_r + m_k)` から逆にたどる（`m_k = m₂ (e^{1.1 dv_k / v_k} − 1)`、
/// `m₁ = (m₂ + m_k) e^{1.1 dv_r / v_ep}`、`m_r = m₁ − m₂ − m_k`）。残る推薬を固定点の繰り返しで決める。
fn hand_masses(dv_r: f64, isp_k: f64, dv_k: f64) -> (f64, f64, f64) {
    let (ve_ep, ve_cg) = (exhaust_velocity(1500.0), exhaust_velocity(isp_k));
    let mut residual = 0.0;
    let mut out = (0.0, 0.0, 0.0);
    for _ in 0..100 {
        let m2 = DRY_KG + residual;
        let m_k = m2 * ((MARGIN * dv_k / ve_cg).exp() - 1.0);
        let m1 = (m2 + m_k) * (MARGIN * dv_r / ve_ep).exp();
        let m_r = m1 - m2 - m_k;
        residual = RESIDUAL * (m_r + m_k);
        out = (m1, m_r, m_k);
    }
    out
}

/// (b) 推薬が 2 種類（EP と CG）で、CG に軌道維持のマヌーバ（520 → 530 km）がある: CG の組だけで換算する。
/// ΔV は CG の比推力 60 s と、そのマヌーバが使う推薬 `m_k` だけ（EP の推薬を CG の比推力で換算しない）:
/// `v_cg ln(m_wet / (m_wet − m_k))`。
#[test]
fn station_keeping_maneuver_selects_its_own_propellant_group() {
    let input = two_propellants(vec![burn(
        "KEEP",
        DvCategory::StationKeeping,
        Some("CG"),
        530e3,
    )]);
    let (wet, m_r, m_k) = hand_masses(dv(500e3, 520e3), 60.0, dv(520e3, 530e3));
    let expected = exhaust_velocity(60.0) * (wet / (wet - m_k)).ln();
    let o = OrbitDecay::run(&input).unwrap();
    assert_eq!(o.used.thruster.as_deref(), Some("CG"));
    assert!((o.used.propellant.unwrap().value() - m_k).abs() < 1e-9);
    assert!((o.used.propulsion_delta_v.unwrap().value() - expected).abs() < 1e-6);
    // 従来の式（全推薬を最小の比推力で換算）とは違う値になる
    let old = exhaust_velocity(60.0) * (wet / (wet - m_r - m_k)).ln();
    assert!((old - expected).abs() > 0.1);
    assert!(check(&o, "station_keeping_thruster").is_none());
    for c in &o.cases {
        assert!(c.station_keeping.propellant_life.is_some());
    }
}

/// (b') 軌道維持のマヌーバが EP にあれば、比推力は EP の 1500 s、推薬は EP の組の軌道維持の分だけ。
#[test]
fn station_keeping_on_the_high_isp_thruster_uses_its_isp() {
    let input = two_propellants(vec![burn(
        "KEEP",
        DvCategory::StationKeeping,
        Some("EP"),
        530e3,
    )]);
    let o = OrbitDecay::run(&input).unwrap();
    assert_eq!(o.used.thruster.as_deref(), Some("EP"));
    // 軌道上げも軌道維持も EP（1500 s）。軌道維持の推薬 `m_k` だけを噴射前のウェット質量から換算する:
    // `v_ep ln(m_wet / (m_wet − m_k))`（質量は `hand_masses` で逆にたどる）
    let (wet, _, m_k) = hand_masses(dv(500e3, 520e3), 1500.0, dv(520e3, 530e3));
    let expected = exhaust_velocity(1500.0) * (wet / (wet - m_k)).ln();
    assert!((o.used.propulsion_delta_v.unwrap().value() - expected).abs() < 1e-6);
}

/// (c) 推薬が 2 種類で軌道維持のマヌーバが無い: 決めずに、容量を持たず注意を出す。
#[test]
fn two_propellants_without_station_keeping_is_undetermined() {
    let input = two_propellants(Vec::new());
    let o = OrbitDecay::run(&input).unwrap();
    assert!(o.used.propulsion_delta_v.is_none());
    assert!(o.used.propellant.is_none());
    assert!(o.used.thruster.is_none());
    let c = check(&o, "station_keeping_thruster").unwrap();
    assert_eq!(c.verdict, Verdict::Warning);
    assert!(c.message.contains("Xe") && c.message.contains("N2"));
    assert!(c.message.contains("station_keeping"));
    for case in &o.cases {
        assert!(case.station_keeping.propellant_life.is_none());
    }
}

/// (d) 軌道維持のマヌーバが別々の推薬のスラスタにまたがる: 決められない。
#[test]
fn station_keeping_across_two_propellants_is_undetermined() {
    let input = two_propellants(vec![
        burn("KEEP_EP", DvCategory::StationKeeping, Some("EP"), 530e3),
        burn("KEEP_CG", DvCategory::StationKeeping, Some("CG"), 520e3),
    ]);
    let o = OrbitDecay::run(&input).unwrap();
    assert!(o.used.propulsion_delta_v.is_none());
    assert!(check(&o, "station_keeping_thruster").is_some());
}

/// (d') スラスタを書かない軌道維持のマヌーバは推薬を求めないので、決める材料にならない。
/// 推薬が 1 種類ならそのまま全量の換算、2 種類なら決められない。
#[test]
fn station_keeping_without_a_thruster_does_not_decide() {
    let keep = burn("KEEP", DvCategory::StationKeeping, None, 530e3);
    let mut single = cubesat_3u(500e3);
    let mut p = propulsion(&single.orbit, DRY_KG);
    p.maneuvers.push(keep.clone());
    single.propulsion = Some(p);
    let o = OrbitDecay::run(&single).unwrap();
    assert_eq!(o.used.thruster.as_deref(), Some("PROP"));
    assert!((o.used.propulsion_delta_v.unwrap().value() - MARGIN * dv(500e3, 520e3)).abs() < 1e-6);
    let o2 = OrbitDecay::run(&two_propellants(vec![keep])).unwrap();
    assert!(o2.used.propulsion_delta_v.is_none());
    assert!(check(&o2, "station_keeping_thruster").is_some());
}

/// (e) 1 つの推薬の組に Isp の違うスラスタが 2 つ（HI 1500 s・LO 60 s、どちらも Xe）。軌道維持のマヌーバが HI を名指すなら、
/// 比推力は組の最低（LO）ではなく HI の 1500 s を使う。軌道上げ（HI）と軌道維持（HI）の質量の列は
/// `hand_masses` と同じ（軌道維持の Isp も 1500 s）で、ΔV は `v_hi ln(m_wet / (m_wet − m_k))`。
#[test]
fn named_thruster_isp_is_used_not_the_lowest_of_the_group() {
    let mut input = two_propellants(vec![burn(
        "KEEP",
        DvCategory::StationKeeping,
        Some("HI"),
        530e3,
    )]);
    let p = input.propulsion.as_mut().unwrap();
    p.thrusters = vec![
        thruster("HI", 1500.0, Some("Xe")),
        thruster("LO", 60.0, Some("Xe")),
    ];
    p.tanks = vec![tank("TXE", "Xe")];
    p.maneuvers[0].thruster = Some("HI".into());
    let (wet, _, m_k) = hand_masses(dv(500e3, 520e3), 1500.0, dv(520e3, 530e3));
    let expected = exhaust_velocity(1500.0) * (wet / (wet - m_k)).ln();
    let o = OrbitDecay::run(&input).unwrap();
    assert_eq!(o.used.thruster.as_deref(), Some("HI"));
    assert!((o.used.propellant.unwrap().value() - m_k).abs() < 1e-9);
    assert!((o.used.propulsion_delta_v.unwrap().value() - expected).abs() < 1e-6);
}
