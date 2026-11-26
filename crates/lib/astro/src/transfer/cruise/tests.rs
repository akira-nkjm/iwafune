//! 巡航の多体の確認の検証。暦は解析式（このクレートのテストは DE440 を登録しない）。
//! 遷移は Curtis Example 8.8（1996-11-07 → 1997-09-12、地球 → 火星）。

use super::*;
use crate::constants::{GM_SUN, R_EARTH, R_MARS};
use crate::force::{GravityModel, acceleration_breakdown_about};
use crate::transfer::{Apsides, dated_transfer};

fn epoch(s: &str) -> Epoch {
    Epoch::parse(s).expect("epoch")
}

fn depart() -> Epoch {
    epoch("1996-11-07T00:00:00Z")
}

fn curtis() -> DatedTransfer {
    dated_transfer(
        CentralBody::Earth,
        Apsides::circular(R_EARTH + 300e3),
        CentralBody::Mars,
        Some(Apsides::circular(R_MARS + 300e3)),
        depart(),
        epoch("1997-09-12T00:00:00Z"),
    )
    .expect("transfer")
}

/// 太陽の点質量だけ（第三体なし、地球も外す）。
fn sun_only() -> ForceConfig {
    ForceConfig {
        omit_earth: true,
        ..ForceConfig::gravity_only(GravityModel::PointMass)
    }
}

fn sun_only_states(t: &DatedTransfer, times: &[f64]) -> Vec<OrbitState> {
    propagate_states(
        CentralBody::Sun,
        &sun_only(),
        depart(),
        t.cruise_state,
        times,
        &CRUISE_INTEGRATOR,
    )
    .expect("propagation")
}

/// (a) 太陽だけの力で Lambert の出発の状態を飛行時間だけ伝播すると、Lambert の到着の位置と速度に戻る
/// （Lambert の解は太陽の点質量の円錐曲線そのもの）。許容は、積分器の相対許容誤差 1e-11 に飛行時間の間のステップ数
/// （~10²）を掛けた 1e-9 を、到着の距離（~2.3e11 m）に当てた 230 m と、速度は 1e-9 × 3e4 m/s = 3e-5 m/s の倍。
#[test]
fn sun_only_propagation_reproduces_lambert_arrival() {
    let t = curtis();
    let arrive = depart().plus_seconds(t.transfer_time_s);
    let end = sun_only_states(&t, &[t.transfer_time_s])[0];
    let target = CentralBody::Mars.state_about_parent(arrive).expect("mars");
    let dr = (end.r_eci - target.r_eci).norm();
    let v_inf_lambert = t.v_inf_arrival_m_s;
    let dv = (end.v_eci - target.v_eci - v_inf_lambert).norm();
    assert!(dr < 230.0, "position differs by {dr} m");
    assert!(dv < 6e-5, "velocity differs by {dv} m/s");
}

/// (c) 太陽だけなら比エネルギー `v²/2 − μ/r` と角運動量が保存する（相対変化 1e-8 以内: 局所の許容 1e-11 × ステップ数 ~10²
/// の 1e-9 の 10 倍の余裕）。
#[test]
fn sun_only_conserves_energy_and_angular_momentum() {
    let t = curtis();
    let n = (t.transfer_time_s / 864_000.0).floor() as u32;
    let times: Vec<f64> = (0..=n).map(|i| f64::from(i) * 864_000.0).collect();
    let states = sun_only_states(&t, &times);
    let energy = |s: &OrbitState| 0.5 * s.v_eci.norm_squared() - GM_SUN / s.r_eci.norm();
    let h = |s: &OrbitState| s.r_eci.cross(s.v_eci);
    let (e0, h0) = (energy(&t.cruise_state), h(&t.cruise_state));
    for s in &states {
        assert!(((energy(s) - e0) / e0).abs() < 1e-8);
        assert!(((h(s) - h0).norm() / h0.norm()) < 1e-8);
    }
}

/// 太陽だけの伝播が通る Lambert の軌道の上で、惑星の摂動の加速度の最大 [m/s²]（太陽中心の第三体の項の和の大きさ）。
fn max_perturbation(t: &DatedTransfer, forces: &ForceConfig) -> f64 {
    let times: Vec<f64> = (0..=30)
        .map(|i| f64::from(i) * t.transfer_time_s / 30.0)
        .collect();
    sun_only_states(t, &times)
        .iter()
        .zip(&times)
        .map(|(s, &tt)| {
            acceleration_breakdown_about(CentralBody::Sun, forces, depart().plus_seconds(tt), s)
                .iter()
                .filter(|(term, _)| *term != crate::force::ForceTerm::CentralBody)
                .fold(Vec3::ZERO, |a, (_, v)| a + *v)
                .norm()
        })
        .fold(0.0, f64::max)
}

/// (b) 惑星（木星・金星・土星など）を入れても、太陽だけの伝播（(a)。到着のずれは ~0）からのずれは小さい。
/// 見積もり: 摂動の加速度の最大 `a_max`（上の関数）が飛行時間 `T` の間はたらくと、速度の変化 `a T`、位置の変化
/// `a T²/2`。軌道の力学では along-track に ~6 倍（速度の変化が長半径を変えて周期がずれる: `δr ≈ 3 δv T`）まで
/// 増えるので、上限を `3 a_max T²` とする（上限であって、実際は加速度の符号の変化と平均で 1 桁以上小さい）。
/// 下限は「惑星の影響が見える」だけを確かめる（1 km 以上）。
#[test]
fn planets_deviate_from_sun_only_within_the_perturbation_estimate() {
    let t = curtis();
    let check = check_cruise(&t, CentralBody::Earth, CentralBody::Mars, depart()).expect("check");
    let forces = ForceConfig::heliocentric_point_masses(&[CentralBody::Earth, CentralBody::Mars]);
    let a_max = max_perturbation(&t, &forces);
    let bound = 3.0 * a_max * t.transfer_time_s.powi(2);
    let miss = check.miss_distance_m();
    assert!(miss > 1e3, "planets had no visible effect: {miss} m");
    assert!(
        miss < bound,
        "deviation {miss} m exceeds the estimate {bound} m (a_max = {a_max} m/s²)"
    );
}

/// 力モデルの組み立て: 端（地球・火星）を外し、地球が端のときは地球の自動の第三体も外す。
#[test]
fn heliocentric_force_model_excludes_the_endpoints() {
    let f = ForceConfig::heliocentric_point_masses(&[CentralBody::Earth, CentralBody::Mars]);
    assert!(f.omit_earth);
    assert!(!f.third_bodies.contains(&crate::ephem::Body::Mars));
    assert!(f.third_bodies.contains(&crate::ephem::Body::Jupiter));
    assert_eq!(f.gravity, GravityModel::PointMass);
    let g = ForceConfig::heliocentric_point_masses(&[CentralBody::Venus, CentralBody::Mars]);
    assert!(!g.omit_earth);
    assert!(!g.third_bodies.contains(&crate::ephem::Body::Venus));
}

/// 結果の形: 解析式の暦なので出所は解析式、軌跡は出発の天体の中心から始まり到着の日で終わる、最接近は到着のずれ以下。
#[test]
fn check_reports_arrival_closest_approach_and_trajectory() {
    let t = curtis();
    let c = check_cruise(&t, CentralBody::Earth, CentralBody::Mars, depart()).expect("check");
    assert_eq!(c.ephemeris, EphemerisSource::Analytic);
    assert_eq!(
        c.perturbers,
        vec![
            CentralBody::Mercury,
            CentralBody::Venus,
            CentralBody::Jupiter,
            CentralBody::Saturn,
            CentralBody::Uranus,
            CentralBody::Neptune,
        ]
    );
    assert!(c.closest.distance_m <= c.miss_distance_m() + 1e-6);
    assert!(!c.closest.at_boundary);
    // 影響圏の半径は火星の ~5.8e8 m（a (m/M)^(2/5)）
    let soi = c.arrival_soi_m.expect("soi");
    assert!((5.0e8..7.0e8).contains(&soi), "{soi}");
    let first = c.samples.first().expect("first");
    assert_eq!(first.t_s, 0.0);
    assert_eq!(first.spacecraft_xy_m, first.from_xy_m);
    let last = c.samples.last().expect("last");
    assert_eq!(last.t_s, t.transfer_time_s);
    assert!(c.samples.len() <= CHART_POINTS + 2 && c.samples.len() > 50);
    // 超過速度は Lambert のものに近い（惑星の摂動の分だけの差。数 m/s 以下）
    let dv = (c.v_inf_arrival_m_s - t.v_inf_arrival_m_s).norm();
    assert!(dv < 50.0, "{dv}");
}

/// 太陽だけの伝播の、火星の中心からの距離 [m]。
fn sun_only_distance(t: &DatedTransfer, times: &[f64]) -> Vec<f64> {
    sun_only_states(t, times)
        .iter()
        .zip(times)
        .map(|(s, &tt)| {
            let mars = heliocentric_position(
                CentralBody::Mars,
                CentralBody::Sun,
                depart().plus_seconds(tt),
            );
            (s.r_eci - mars).norm()
        })
        .collect()
}

/// 太陽だけの力なら、探査機は到着天体の中心を通り、最接近は到着の日（距離は 0 に近い）。
#[test]
fn sun_only_passes_through_the_target_at_arrival() {
    let t = curtis();
    let arrive = depart().plus_seconds(t.transfer_time_s);
    let (times, ai) = time_grid(arrive, t.transfer_time_s);
    let d = sun_only_distance(&t, &times);
    let c = closest_approach(&times, &d, |ts| Ok(sun_only_distance(&t, ts))).expect("closest");
    assert!((c.t_s - times[ai]).abs() < 600.0, "{}", c.t_s - times[ai]);
    assert!(c.distance_m < 1.0e3, "{}", c.distance_m);
}
