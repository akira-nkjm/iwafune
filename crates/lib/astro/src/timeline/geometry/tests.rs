//! ミッション幾何のテスト。期待値は導出（手計算・恒等式・上下限）だけから決め、許容誤差を値に合わせて調整しない。

use super::*;
use crate::constants::{AU, SECONDS_PER_DAY};
use crate::ephem::{Body, eci_to_ecliptic_j2000, ecliptic_normal_eci, position_eci_m};
use crate::orbit::{OrbitElements, OrbitSpec, Propagator};
use crate::timeline::sample_times;
use sat_units::typed::Qty;
use std::f64::consts::PI;

/// 地球の軌道離心率の上限（J2000 付近 0.0167。上下限は 1 ± e）。
const E_EARTH: f64 = 0.0168;

fn epoch() -> Epoch {
    Epoch::parse("2027-04-01T00:00:00Z").expect("epoch")
}

/// 黄道面（傾斜角 ε₀・昇交点が春分点方向）の日心円軌道。黄道経度 `lambda` の位置から始まる。
fn ecliptic_circle(a_m: f64, lambda: f64) -> Trajectory {
    let eps = ecliptic_normal_eci().z.acos();
    let elements = OrbitElements {
        semi_major_axis: Qty::from_raw(a_m),
        eccentricity: 0.0,
        inclination: Qty::from_raw(eps),
        raan: Qty::from_raw(0.0),
        arg_perigee: Qty::from_raw(0.0),
        mean_anomaly: Qty::from_raw(lambda),
    };
    let mut o = OrbitSpec::new(epoch(), elements, Propagator::Kepler);
    o.central = CentralBody::Sun;
    Trajectory::from(o)
}

/// 元期の地球の日心黄道経度（太陽の地心位置の反対向き）。
fn earth_longitude() -> f64 {
    let s = position_eci_m(Body::Sun, epoch());
    let e = eci_to_ecliptic_j2000(-s);
    e.y.atan2(e.x)
}

fn leo() -> (Trajectory, f64) {
    let o = OrbitSpec::circular(
        epoch(),
        Qty::from_raw(550.0e3),
        Qty::from_raw(0.9),
        Qty::from_raw(0.3),
    );
    let a = o.elements.semi_major_axis.value();
    (Trajectory::from(o), a)
}

/// 地球を回る円軌道: 地球までの距離は軌道半径そのもの（暦の桁落ちを入れない）、光の伝搬時間は半径 / 光速、
/// 太陽距離は地球の日心距離 ± 軌道半径（地球の近日点・遠日点 AU(1 ∓ e) の範囲）。
#[test]
fn earth_orbit_distances_are_the_orbit_radius() {
    let (orbit, a) = leo();
    let times = sample_times(365.25 * SECONDS_PER_DAY, 6.0 * 3600.0).expect("times");
    let g = geometry_at(&orbit, epoch(), &times).expect("geometry");
    for s in &g {
        assert!(
            (s.earth_distance_m / a - 1.0).abs() < 1e-9,
            "{}",
            s.earth_distance_m
        );
        assert!((s.light_time_s * C_LIGHT / a - 1.0).abs() < 1e-9);
        assert!(
            s.sun_distance_m >= AU * (1.0 - E_EARTH) - a,
            "{}",
            s.sun_distance_m
        );
        assert!(
            s.sun_distance_m <= AU * (1.0 + E_EARTH) + a,
            "{}",
            s.sun_distance_m
        );
        assert!((0.0..=PI).contains(&s.sep_rad) && (0.0..=PI).contains(&s.spe_rad));
    }
}

/// 地球中心でも、幾何の距離は軌道の状態から直接求めた半径と一致し、軽い入口 [`body_distances_m`] とビットまで同じ。
#[test]
fn distance_entry_points_agree_bit_for_bit() {
    let (orbit, _) = leo();
    let times = sample_times(3.0 * 3600.0, 60.0).expect("times");
    let g = geometry_at(&orbit, epoch(), &times).expect("geometry");
    let d = body_distances_m(&orbit, epoch(), &times, CentralBody::Earth).expect("distances");
    let v = body_views_at(&orbit, epoch(), &times, CentralBody::Earth).expect("views");
    let states = orbit.states_at(&times).expect("states");
    for i in 0..times.len() {
        assert_eq!(g[i].earth_distance_m, d[i]);
        assert_eq!(v[i].distance_m, d[i]);
        assert_eq!(d[i], states[i].r_eci.norm());
    }
}

/// 地球までの距離は、従来の式 `|中心天体の地心位置 + 軌道の相対位置|`（`ground::earth_range` の元の計算）とビットまで同じ。
/// 地球・月・太陽を回る軌道で確かめる。
#[test]
fn earth_distance_equals_the_former_earth_range_formula() {
    let moon = OrbitSpec::circular_about(
        CentralBody::Moon,
        epoch(),
        Qty::from_raw(100.0e3),
        Qty::from_raw(0.5),
        Qty::from_raw(0.0),
    );
    let orbits = [
        leo().0,
        Trajectory::from(moon),
        ecliptic_circle(1.5 * AU, 2.0),
    ];
    let times = sample_times(30.0 * SECONDS_PER_DAY, 6.0 * 3600.0).expect("times");
    for orbit in &orbits {
        let d = body_distances_m(orbit, epoch(), &times, CentralBody::Earth).expect("distances");
        let states = orbit.states_at(&times).expect("states");
        for ((t, s), d) in times.iter().zip(&states).zip(&d) {
            let former =
                (orbit.central().position_eci_m(epoch().plus_seconds(*t)) + s.r_eci).norm();
            assert_eq!(*d, former);
        }
    }
}

/// 太陽中心の軌道の太陽距離は軌道半径 `|r|` そのもの（ビットまで同じ）。
#[test]
fn sun_distance_of_a_heliocentric_orbit_is_the_orbit_radius() {
    let orbit = ecliptic_circle(1.5 * AU, 0.4);
    let times = sample_times(200.0 * SECONDS_PER_DAY, 5.0 * SECONDS_PER_DAY).expect("times");
    let d = body_distances_m(&orbit, epoch(), &times, CentralBody::Sun).expect("distances");
    let states = orbit.states_at(&times).expect("states");
    for (d, s) in d.iter().zip(&states) {
        assert_eq!(*d, s.r_eci.norm());
    }
}

/// 日心 1.5 AU の円軌道: 太陽距離は一定で 1.5 AU（伝搬の誤差 1e-9）。地球距離は 1 周（1 年以上）で
/// 1.5 AU − (地球の日心距離) 〜 1.5 AU + (地球の日心距離)、地球の日心距離は AU(1 ± e) に入るので、
/// 範囲は [0.5 − e, 2.5 + e] AU に収まる。
#[test]
fn heliocentric_circle_bounds() {
    let orbit = ecliptic_circle(1.5 * AU, 0.0);
    let times = sample_times(2.0 * 365.25 * SECONDS_PER_DAY, SECONDS_PER_DAY).expect("times");
    let g = geometry_at(&orbit, epoch(), &times).expect("geometry");
    for s in &g {
        assert!((s.sun_distance_m / (1.5 * AU) - 1.0).abs() < 1e-9);
        assert!(
            s.earth_distance_m >= (0.5 - E_EARTH) * AU,
            "{}",
            s.earth_distance_m / AU
        );
        assert!(
            s.earth_distance_m <= (2.5 + E_EARTH) * AU,
            "{}",
            s.earth_distance_m / AU
        );
    }
    // 1 周で合と衝の近くも通るので、下限・上限の近くまで届く（e の上限の 2 倍の余裕）
    let min = g
        .iter()
        .map(|s| s.earth_distance_m)
        .fold(f64::MAX, f64::min);
    let max = g.iter().map(|s| s.earth_distance_m).fold(0.0, f64::max);
    assert!(min < (0.5 + 2.0 * E_EARTH) * AU && max > (2.5 - 2.0 * E_EARTH) * AU);
}

/// 地球と同じ黄道経度の 1.5 AU の円軌道（地球の外側、太陽–地球–探査機が一直線）: 元期で衝。
/// SEP = 180°（太陽と探査機は地球から反対向き）、SPE = 0°（探査機から太陽と地球は同じ向き）、
/// 地球距離 = 1.5 AU − (地球の日心距離)。許容 1e-4 rad は地球の黄道緯度（J2000 の黄道面から 1e-5 rad 程度）の上限に余裕を見た値。
#[test]
fn opposition_has_sep_180_and_spe_0() {
    let orbit = ecliptic_circle(1.5 * AU, earth_longitude());
    let g = &geometry_at(&orbit, epoch(), &[0.0]).expect("geometry")[0];
    let d_earth_sun = position_eci_m(Body::Sun, epoch()).norm();
    assert!((g.sep_rad - PI).abs() < 1e-4, "{}", g.sep_rad);
    assert!(g.spe_rad.abs() < 1e-4, "{}", g.spe_rad);
    assert!((g.earth_distance_m / (1.5 * AU - d_earth_sun) - 1.0).abs() < 1e-6);
}

/// 地球の反対側（黄道経度 + 180°）の 1.5 AU の円軌道: 元期で外合（太陽が地球と探査機の間）。
/// SEP = 0°、SPE = 0°、地球距離 = 1.5 AU + (地球の日心距離)。
#[test]
fn superior_conjunction_has_sep_0_and_spe_0() {
    let orbit = ecliptic_circle(1.5 * AU, earth_longitude() + PI);
    let g = &geometry_at(&orbit, epoch(), &[0.0]).expect("geometry")[0];
    let d_earth_sun = position_eci_m(Body::Sun, epoch()).norm();
    assert!(g.sep_rad.abs() < 1e-4, "{}", g.sep_rad);
    assert!(g.spe_rad.abs() < 1e-4, "{}", g.spe_rad);
    assert!((g.earth_distance_m / (1.5 * AU + d_earth_sun) - 1.0).abs() < 1e-6);
}

/// 三角形（太陽・地球・探査機）: 内角の和 SEP + SPE + (太陽での角) = π、余弦定理が 3 辺すべてで成り立ち、
/// 光の伝搬時間 × 光速 = 地球距離。1 周（1.2 年）の全標本で確かめる。
#[test]
fn triangle_identities_hold_at_every_sample() {
    let orbit = ecliptic_circle(1.5 * AU, 1.0);
    let times = sample_times(1.2 * 365.25 * SECONDS_PER_DAY, 3.0 * SECONDS_PER_DAY).expect("times");
    let g = geometry_at(&orbit, epoch(), &times).expect("geometry");
    let states = orbit.states_at(&times).expect("states");
    for (s, st) in g.iter().zip(&states) {
        let sun = position_eci_m(Body::Sun, s.epoch);
        // 太陽から見た地球・探査機の向き
        let earth_from_sun = -sun;
        let probe_from_sun = st.r_eci;
        let theta_sun = earth_from_sun.angle_to(probe_from_sun);
        assert!(
            (s.sep_rad + s.spe_rad + theta_sun - PI).abs() < 1e-9,
            "{}",
            s.sep_rad + s.spe_rad + theta_sun - PI
        );
        // 3 辺: 太陽–地球 d_se、地球–探査機 d_ep（= 地球距離）、太陽–探査機 d_sp（= 太陽距離）
        let (d_se, d_ep, d_sp) = (sun.norm(), s.earth_distance_m, s.sun_distance_m);
        let cos = |a: f64, b: f64, c: f64, ang: f64| {
            (a * a + b * b - 2.0 * a * b * ang.cos() - c * c).abs() / (c * c)
        };
        assert!(cos(d_se, d_ep, d_sp, s.sep_rad) < 1e-9);
        assert!(cos(d_sp, d_ep, d_se, s.spe_rad) < 1e-9);
        assert!(cos(d_se, d_sp, d_ep, theta_sun) < 1e-9);
        assert!((s.light_time_s * C_LIGHT / d_ep - 1.0).abs() < 1e-12);
    }
}

/// 任意の天体: 月を回る円軌道（高度 100 km）の月までの距離は軌道半径、地球までの距離は月の地心距離 ± 軌道半径。
/// 太陽の幾何は地球の見方と同じ三角形の恒等式を満たす。
#[test]
fn body_views_of_a_lunar_orbiter() {
    let o = OrbitSpec::circular_about(
        CentralBody::Moon,
        epoch(),
        Qty::from_raw(100.0e3),
        Qty::from_raw(0.5),
        Qty::from_raw(0.0),
    );
    let a = o.elements.semi_major_axis.value();
    let orbit = Trajectory::from(o);
    let times = sample_times(3.0 * 3600.0, 120.0).expect("times");
    let moon = body_views_at(&orbit, epoch(), &times, CentralBody::Moon).expect("moon");
    let earth = body_views_at(&orbit, epoch(), &times, CentralBody::Earth).expect("earth");
    let sun = body_views_at(&orbit, epoch(), &times, CentralBody::Sun).expect("sun");
    for (i, &t) in times.iter().enumerate() {
        assert!((moon[i].distance_m / a - 1.0).abs() < 1e-9);
        let d_moon = CentralBody::Moon
            .position_eci_m(epoch().plus_seconds(t))
            .norm();
        assert!((earth[i].distance_m - d_moon).abs() <= a * (1.0 + 1e-9));
        assert!(sun[i].angles.is_none());
        let ang = earth[i].angles.expect("angles");
        assert!(ang.at_body_rad + ang.at_probe_rad < PI);
    }
}

/// 標本の始めが軌道の元期と違っても、同じ時刻の値は同じ（`start` と元期の差を経過時間に直して伝搬する）。
#[test]
fn start_may_differ_from_the_orbit_epoch() {
    let orbit = ecliptic_circle(1.5 * AU, 0.2);
    let offset = 10.0 * SECONDS_PER_DAY;
    let a = geometry_at(&orbit, epoch(), &[offset]).expect("a");
    let b = geometry_at(&orbit, epoch().plus_seconds(offset), &[0.0]).expect("b");
    assert!((a[0].earth_distance_m / b[0].earth_distance_m - 1.0).abs() < 1e-12);
    assert!((a[0].sep_rad - b[0].sep_rad).abs() < 1e-12);
}

/// 極値: `f(t) = 1 + cos(t − 0.3)` を刻み 0.5 で標本する。最大は t = 0.3、最小は t = 0.3 + π。
/// 標本の最大は 0.5（真値から 0.2 = 0.4 刻み）、最小は 3.5（0.058）で、刻みが決める誤差の桁。詰めた極値の時刻の誤差は、
/// 頂点付近の 2 次の平らさによる限界 √(2ε·f/|f″|) ≈ √(2 × 2.2e-16 × 2 / 1) ≈ 3e-8 より小さい（上限 1e-7 とする）。
#[test]
fn refinement_beats_the_sample_spacing() {
    let f = |t: f64| 1.0 + (t - 0.3).cos();
    let times: Vec<f64> = (0..=12).map(|i| i as f64 * 0.5).collect();
    let values: Vec<f64> = times.iter().map(|&t| f(t)).collect();
    let (imin, imax) = extreme_indices(&values).expect("indices");
    assert_eq!((times[imin], times[imax]), (3.5, 0.5));
    let x = extremes_by(&times, &values, |t| Ok(f(t)))
        .expect("ok")
        .expect("some");
    let (t_max, t_min) = (0.3, 0.3 + PI);
    assert!((x.max.t_s - t_max).abs() < 1e-7, "{}", x.max.t_s);
    assert!((x.min.t_s - t_min).abs() < 1e-7, "{}", x.min.t_s);
    // 標本の最大・最小の誤差（0.2・0.058）より桁違いに小さい
    assert!((times[imax] - t_max).abs() > 0.1 && (times[imin] - t_min).abs() > 0.05);
    assert!((x.max.value - 2.0).abs() < 1e-14 && x.min.value.abs() < 1e-14);
}

/// 極値が端の標本なら端のまま（単調な関数）。評価の失敗はそのまま返る。空の標本は `None`。
#[test]
fn refinement_keeps_end_samples_and_propagates_errors() {
    let times: Vec<f64> = (0..=10).map(f64::from).collect();
    let values: Vec<f64> = times.iter().map(|&t| 2.0 * t).collect();
    let x = extremes_by(&times, &values, |t| Ok(2.0 * t))
        .expect("ok")
        .expect("some");
    assert_eq!((x.min.t_s, x.max.t_s), (0.0, 10.0));
    let err = extremes_by(&times, &values, |_| {
        Err(AstroError::InvalidInput("x".into()))
    });
    assert!(err.is_err());
    assert!(extremes_by(&[], &[], Ok).expect("ok").is_none());
}

/// 幾何の量での極値: 日心 1.5 AU の円軌道の地球距離の最小は、標本より細かく詰めた値が標本の最小以下。
#[test]
fn refined_geometry_extreme_is_not_worse_than_the_samples() {
    let orbit = ecliptic_circle(1.5 * AU, 1.0);
    let times =
        sample_times(2.0 * 365.25 * SECONDS_PER_DAY, 10.0 * SECONDS_PER_DAY).expect("times");
    let g = geometry_at(&orbit, epoch(), &times).expect("geometry");
    let r = refined_extremes(&orbit, epoch(), &g, Metric::EarthDistance)
        .expect("ok")
        .expect("some");
    let coarse = g
        .iter()
        .map(|s| s.earth_distance_m)
        .fold(f64::MAX, f64::min);
    assert!(r.min.value <= coarse && r.min.value > (0.5 - E_EARTH) * AU);
    let coarse_max = g.iter().map(|s| s.earth_distance_m).fold(0.0, f64::max);
    assert!(r.max.value >= coarse_max && r.max.value < (2.5 + E_EARTH) * AU);
}
