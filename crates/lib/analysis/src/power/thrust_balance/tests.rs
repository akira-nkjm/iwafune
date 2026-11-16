//! 噴射の仕方 → 電力収支の負荷のかけ方を、モードの消費電力の手計算と比べる。

use super::*;
use crate::modes::{ModeDef, Redundancy, StateAssignment, StateValues};
use crate::power::energy_balance::{EnergyBalanceBattery, LoadGrowth};
use crate::power::mode_load::{ModeLoadInput, ModeLoadPart};
use crate::power::solar::{SolarArray, SolarLife, SolarPanel};
use crate::power::testkit::{leo_timeline, sun_pointing_z};
use sat_astro::math::Vec3;
use std::collections::BTreeMap;

fn part(id: &str, w: f64) -> ModeLoadPart {
    ModeLoadPart {
        id: id.to_owned(),
        label: String::new(),
        group: String::new(),
        count: 1,
        power: StateValues {
            default: None,
            by_state: [("on".to_owned(), Qty::from_raw(w))].into(),
        },
        growth_margin: Some(0.0),
        redundancy: Redundancy::None,
        states: Vec::new(),
    }
}

fn mode(id: &str, parts: &[&str]) -> ModeDef {
    ModeDef {
        id: id.to_owned(),
        label: String::new(),
        assignments: parts
            .iter()
            .map(|p| StateAssignment {
                part: (*p).to_owned(),
                state: "on".to_owned(),
                duty: Default::default(),
                active: None,
                spare_state: None,
            })
            .collect(),
        eclipse: Vec::new(),
    }
}

/// 太陽指向の +Z 面 1 m²、電池 100 Wh。基本のモード BASE は OBC 2 W、噴射のモード THRUST は OBC 2 W + EPT 30 W。
fn input(base: Option<&str>) -> EnergyBalanceInput {
    EnergyBalanceInput {
        timeline: leo_timeline(sun_pointing_z(), 2.0, 10.0),
        array: SolarArray {
            panels: vec![SolarPanel {
                id: "PZ".to_owned(),
                area: Qty::from_raw(1.0),
                cell_efficiency: 0.3,
                packing_factor: Some(1.0),
                eol_factor: Some(0.8),
                normal: Vec3::Z,
            }],
            solar_flux: Some(Qty::from_raw(1361.0)),
            conversion_efficiency: None,
        },
        life: Some(SolarLife::Bol),
        batteries: vec![EnergyBalanceBattery {
            capacity: Qty::from_raw(360_000.0),
            voltage: Qty::from_raw(8.0),
            charge_efficiency: Some(0.9),
            discharge_efficiency: Some(0.95),
            initial_soc: Some(1.0),
            dod_limit: 0.3,
            max_charge_power: None,
        }],
        loads: ModeLoadInput {
            parts: vec![part("OBC", 2.0), part("EPT", 30.0)],
            modes: vec![mode("BASE", &["OBC"]), mode("THRUST", &["OBC", "EPT"])],
            default_growth_margin: Some(0.0),
        },
        load_growth: Some(LoadGrowth::Included),
        cases: Vec::new(),
        sequence: Vec::new(),
        base_mode: base.map(str::to_owned),
        mode_attitudes: BTreeMap::new(),
    }
}

fn thrust(sunlit_only: bool, duty: Option<f64>) -> ThrustLoad {
    ThrustLoad {
        maneuver: "RAISE".to_owned(),
        mode: "THRUST".to_owned(),
        sunlit_only: Some(sunlit_only),
        duty: duty.map(Fraction::new),
    }
}

fn run(base: Option<&str>, t: ThrustLoad) -> Result<ThrustBalanceResult, AnalysisError> {
    let out = ThrustBalance::run(&ThrustBalanceInput {
        balance: input(base),
        thrusts: vec![t],
    })?;
    Ok(out.thrusts.into_iter().next().unwrap())
}

#[test]
fn sunlit_only_fires_in_sunlight_and_rests_in_eclipse() {
    let r = run(Some("BASE"), thrust(true, None)).unwrap();
    assert!((r.balance.load_sunlit.value() - 32.0).abs() < 1e-9);
    assert!((r.balance.load_eclipse.value() - 2.0).abs() < 1e-9);
    assert_eq!(r.base_mode.as_deref(), Some("BASE"));
    assert_eq!(r.balance.id, "RAISE");
    // 日陰で噴き続けると日陰の負荷は 32 W、DoD は深い
    let all = run(None, thrust(false, None)).unwrap();
    assert!((all.balance.load_eclipse.value() - 32.0).abs() < 1e-9);
    assert!(all.balance.max_dod.value() > r.balance.max_dod.value());
}

#[test]
fn duty_runs_a_block_sequence_with_the_average_load() {
    // デューティ 0.5: 軌道平均の負荷 = 0.5 × 32 + 0.5 × 2 = 17 W（刻み 10 s の丸めで 2 % 以内）
    let r = run(Some("BASE"), thrust(false, Some(0.5))).unwrap();
    let avg = r.balance.orbit_average_load.value();
    assert!((avg - 17.0).abs() < 0.02 * 17.0, "{avg}");
    assert_eq!(r.note, Some(ThrustBalanceNote::DutyAsBlockPerOrbit));
    assert_eq!(r.balance.id, "RAISE");
}

#[test]
fn a_base_mode_is_needed_when_the_thruster_rests() {
    assert!(run(None, thrust(true, None)).is_err());
    assert!(run(Some("BASE"), thrust(false, Some(1.5))).is_err());
}
