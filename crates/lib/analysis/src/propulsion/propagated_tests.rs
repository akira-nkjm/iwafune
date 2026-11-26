//! 数値伝播（`propagated`）のマヌーバを、低推力の解析式（Edelbaum）の結果と比べる（推力の小さい極限）。

use super::tests::{input, maneuver};
use super::*;
use crate::orbit::{EclipseSeason, EclipseSeasonInput};
use crate::spec::Analysis;
use sat_astro::constants::R_EARTH;
use sat_units::typed::Qty;

/// 電気推進 10 mN・1500 s で 400 → 450 km（乾燥 10 kg、1e-3 m/s²）。
fn raise(method: TransferMethod, sunlit_only: bool) -> ManeuverInput {
    let mut m = maneuver("RAISE", DvCategory::Insertion, method, 450.0);
    m.thruster = Some("EP".to_owned());
    m.thrust.sunlit_only = Some(sunlit_only);
    m
}

/// ΔV は Edelbaum と 1 % 以内、推薬はその ΔV のロケット方程式、時間は噴射時間（噴き続けるので）。
#[test]
fn propagated_raise_matches_low_thrust_within_one_percent() {
    let low = Maneuvers::run(&input(vec![raise(TransferMethod::LowThrust, false)])).unwrap();
    let prop = Maneuvers::run(&input(vec![raise(TransferMethod::Propagated, false)])).unwrap();
    let (l, p) = (&low.maneuvers[0], &prop.maneuvers[0]);
    let rel = (p.delta_v.value() - l.delta_v.value()).abs() / l.delta_v.value();
    assert!(
        rel < 0.01,
        "{} vs {} ({rel})",
        p.delta_v.value(),
        l.delta_v.value()
    );
    let detail = p.propagation.as_ref().unwrap();
    assert!(detail.reached);
    assert!((detail.final_altitude.value() - 450e3).abs() < 100.0);
    // 推薬: 伝播の質量と列の質量は同じ（乾燥質量 + 推薬）なので、伝播の推薬と列の推薬は一致する
    let mp = p.propellant.unwrap().value();
    assert!(
        (detail.propellant.value() - mp).abs() < 1e-4 * mp,
        "{} vs {mp}",
        detail.propellant.value()
    );
    assert!((p.transfer_time.unwrap().value() - detail.duration.value()).abs() < 1e-9);
    assert!((detail.firing_fraction.value() - 1.0).abs() < 1e-9);
    assert!(p.ignored_constraints.is_empty());
    // 終わりの軌道は周回平均の要素: 平均の高度は目標、離心率は J2 の強制の分（~5e-4）だけ
    let f = &prop.final_orbit;
    let mid = 0.5 * (f.apogee_altitude.value() + f.perigee_altitude.value());
    assert!((mid - 450e3).abs() < 100.0, "{mid}");
    assert!(
        f.apogee_altitude.value() - f.perigee_altitude.value() < 2.0 * 1e-3 * (R_EARTH + 450e3)
    );
    assert!(low.maneuvers[0].propagation.is_none());
}

/// 日照中だけ噴くと時間が延び（噴く割合 ~0.6）、ΔV はほぼ同じ。
#[test]
fn sunlit_only_propagation_takes_longer() {
    let all = Maneuvers::run(&input(vec![raise(TransferMethod::Propagated, false)])).unwrap();
    let lit = Maneuvers::run(&input(vec![raise(TransferMethod::Propagated, true)])).unwrap();
    let (a, s) = (
        all.maneuvers[0].propagation.as_ref().unwrap(),
        lit.maneuvers[0].propagation.as_ref().unwrap(),
    );
    assert!(
        s.duration.value() > 1.3 * a.duration.value(),
        "{} vs {}",
        s.duration.value(),
        a.duration.value()
    );
    assert!(
        (0.5..0.8).contains(&s.firing_fraction.value()),
        "{}",
        s.firing_fraction.value()
    );
    let dv = |o: &ManeuversOutput| o.maneuvers[0].delta_v.value();
    assert!((dv(&lit) - dv(&all)).abs() < 0.02 * dv(&all));
}

/// 軌道の推移: 伝播の間は ~1 周ごとの区間、終わりは 450 km。季節解析はマヌーバのあとのサンプル日をその軌道で求める。
#[test]
fn orbit_history_follows_the_propagated_raise() {
    let inp = input(vec![raise(TransferMethod::Propagated, false)]);
    let out = OrbitHistoryAnalysis::run(&inp).unwrap();
    let segs = &out.history.segments;
    assert!(segs.len() > 4, "{}", segs.len());
    assert!(segs[1..segs.len() - 1].iter().all(|s| s.thrusting));
    let last = segs.last().unwrap();
    assert!(!last.thrusting);
    assert!(
        (last.orbit.altitude_m() - 450e3).abs() < 100.0,
        "{}",
        last.orbit.altitude_m()
    );
    assert!((out.maneuvers_end.value() - last.start.value()).abs() < 1e-6);
    assert_eq!(out.propagated, ["RAISE"]);
    let alt = &out.series.channels[0].values;
    assert!(
        alt.windows(2).all(|w| w[1] >= w[0] - 50.0),
        "altitude keeps rising"
    );
    // 3 日の季節: 2 日目以降のサンプル日は 450 km の軌道（周期が長い）
    let season = |maneuvers| {
        EclipseSeason::run(&EclipseSeasonInput {
            orbit: inp.initial_orbit.clone(),
            duration: Qty::from_raw(3.0 * 86_400.0),
            step: None,
            maneuvers,
        })
        .unwrap()
    };
    let with = season(Some(inp.clone()));
    let without = season(None);
    assert_eq!(with.maneuvers, ["RAISE"]);
    assert!(without.maneuvers.is_empty());
    assert_eq!(with.points.len(), without.points.len());
    // 1 日目（元期）は同じ、高い軌道では本影が短い（地球の見かけの大きさが小さい）
    assert!((with.points[0].umbra.value() - without.points[0].umbra.value()).abs() < 1e-9);
    assert!(with.points[2].umbra.value() < without.points[2].umbra.value());
}
