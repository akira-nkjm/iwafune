//! 弧の連鎖（スイングバイのある巡航）でのミッション幾何: 距離・角は境で連続で、極値の詰めは境をまたがない。
//! 期待値は暦（金星の位置）・Lambert の弧の導出から決め、許容誤差を値に合わせて調整しない。

use super::*;
use crate::timeline::sample_times;
use crate::transfer::{FlybyStop, cruise_trajectory};

fn epoch(s: &str) -> Epoch {
    Epoch::parse(s).expect("epoch")
}

const FLYBY: &str = "2028-12-30T00:00:00Z";

/// 地球 → 金星（通過）→ 火星の 2 本の弧の連鎖（解析式の暦）。
fn chain() -> Trajectory {
    cruise_trajectory(
        CentralBody::Earth,
        CentralBody::Mars,
        epoch("2028-10-01T00:00:00Z"),
        epoch("2030-02-23T00:00:00Z"),
        &[FlybyStop {
            body: CentralBody::Venus,
            date: epoch(FLYBY),
            min_altitude_m: None,
        }],
    )
    .expect("chain")
}

/// 境の日の太陽距離は、その日の金星の日心距離（暦の位置の大きさ）に等しい（両方の弧が金星の位置を通る）。許容 1 m は
/// 連鎖の検査の許容（`BREAK_POSITION_TOL_M`）と同じ。境の前後 1 ms の距離は、速さの上限（1e5 m/s）× 時間の差の範囲で連続。
#[test]
fn the_sun_distance_at_the_flyby_is_the_flyby_bodys_heliocentric_distance() {
    let t = chain();
    let at = t.breakpoints_s()[0];
    let venus = CentralBody::Venus
        .state_about_parent(epoch(FLYBY))
        .expect("venus")
        .r_eci
        .norm();
    let eps = 1e-3;
    let s = geometry_at(&t, t.epoch(), &[at - eps, at, at + eps]).expect("samples");
    assert!((s[1].sun_distance_m - venus).abs() < 1.0);
    for (a, b) in [(0, 1), (1, 2)] {
        let step = |f: fn(&GeometrySample) -> f64| (f(&s[a]) - f(&s[b])).abs();
        assert!(step(|x| x.sun_distance_m) < 1.0e5 * eps);
        assert!(step(|x| x.earth_distance_m) < 1.0e5 * eps);
        // 角は距離の変化を距離で割った量の範囲で連続（SEP・SPE とも、地球・太陽までの最短距離 0.3 au 以上で 1e5 m/s × 1 ms / 4e10 m）
        assert!(step(|x| x.sep_rad) < 1.0e-5 && step(|x| x.spe_rad) < 1.0e-5);
    }
}

/// 極値の詰めは、密な標本の極値と一致する: 詰めた値は標本のどの値より悪くならず、密な標本の値との差は、標本の間隔 `h` の
/// 半分以内にある真の極値からの 2 次の差（`½ a_max (h/2)²`。太陽距離の加速度は `v²/r + GM/r² ≤ 0.1 m/s²`、近日点 0.4 au 以上）以内。
#[test]
fn refined_extremes_agree_with_dense_sampling_of_each_arc() {
    let t = chain();
    let span = t.cycle().length_s();
    let coarse = sample_times(span, span / 200.0).expect("times");
    let samples = geometry_at(&t, t.epoch(), &coarse).expect("samples");
    let refined = refined_extremes(&t, t.epoch(), &samples, Metric::SunDistance)
        .expect("extremes")
        .expect("some");
    let dense_times = sample_times(span, span / 40_000.0).expect("times");
    let dense = body_distances_m(&t, t.epoch(), &dense_times, CentralBody::Sun).expect("dense");
    let (dmin, dmax) = (
        dense.iter().copied().fold(f64::INFINITY, f64::min),
        dense.iter().copied().fold(0.0_f64, f64::max),
    );
    let h = span / 40_000.0;
    let bound = 0.5 * 0.1 * (h / 2.0) * (h / 2.0);
    assert!(refined.min.value <= dmin + 1.0 && refined.max.value >= dmax - 1.0);
    assert!(
        dmin - refined.min.value <= bound,
        "{} {}",
        dmin,
        refined.min.value
    );
    assert!(
        refined.max.value - dmax <= bound,
        "{} {}",
        dmax,
        refined.max.value
    );
}

/// 距離が折れ目で V 字に極小になるとき（折れ目の前は減り、後は増える）、詰めた極値は折れ目そのもの（時刻・値が厳密）。
/// 折れ目を候補にするので、黄金分割の収束の幅に左右されない。折れ目を渡さなければ（従来の `extremes_by`）近傍の値にとどまる。
#[test]
fn a_kink_minimum_is_found_exactly_at_the_break() {
    let times = [0.0_f64, 4.0, 8.0];
    let f = |t: f64| Ok((t - 5.0).abs() + 1.0);
    let values: Vec<f64> = times.iter().map(|t| (t - 5.0).abs() + 1.0).collect();
    let split = extremes_by_split(&times, &values, &[5.0], f)
        .expect("ok")
        .expect("some");
    assert_eq!((split.min.t_s, split.min.value), (5.0, 1.0));
    let plain = extremes_by(&times, &values, f).expect("ok").expect("some");
    assert!((plain.min.t_s - 5.0).abs() < 1e-6 && plain.min.value >= 1.0);
    // 折れ目が区間の外なら従来と同じ
    let outside = extremes_by_split(&times, &values, &[100.0], f)
        .expect("ok")
        .expect("some");
    assert_eq!(outside.min, plain.min);
    assert_eq!(outside.max, plain.max);
}

/// 折れ目は標本の列に昇順で加わる（範囲の外・すでにある時刻は加えない）。折れ目が無ければ列そのまま。
#[test]
fn breaks_are_merged_into_the_sample_times() {
    let times = [0.0, 10.0, 20.0, 30.0];
    assert_eq!(times_with_breaks(&times, &[]), times);
    assert_eq!(
        times_with_breaks(&times, &[12.5, 20.0, 99.0, -1.0]),
        [0.0, 10.0, 12.5, 20.0, 30.0]
    );
    assert!(times_with_breaks(&[], &[1.0]).is_empty());
}

/// 地球からの距離の範囲（回線の距離）は境の標本を含む: 標本が両端と中央しかなくても、境の日の地球距離は最小と最大のあいだにある。
#[test]
fn the_earth_range_includes_the_flyby_sample() {
    let t = chain();
    let span = t.cycle().length_s();
    let at = t.breakpoints_s()[0];
    let r = crate::ground::earth_range(&t, t.epoch(), span, span / 2.0).expect("range");
    let d = geometry_at(&t, t.epoch(), &[at]).expect("sample")[0].earth_distance_m;
    assert!(r.min_m <= d && d <= r.max_m, "{} {d} {}", r.min_m, r.max_m);
}
