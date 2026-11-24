//! 最接近（2 つの動く点の距離の最小）の探索。**距離の時系列の最小を求めるのはここだけ。**
//!
//! 粗い格子で測った距離から最小の前後 1 区間を選び、その区間を細かい格子で測り直して、最小の近くの 3 点の
//! 距離の 2 乗に放物線を当てて時刻と距離を求める。相対運動が直線に近い（影響圏の外を通る）間は距離の 2 乗が
//! 時刻の 2 次式（`b² + v²(t − t_c)²`）なので、放物線で厳密に求まる。距離をどう測るか（伝搬・暦）は呼び出し側の
//! 関数に任せる（巡航の伝搬 `crate::transfer::cruise` と、同じ伝搬で作る地球・月の最接近などが共有する）。

use crate::AstroError;

/// 細かい格子の点の数（最小の前後 1 区間を等分する）。
const FINE_POINTS: usize = 401;

/// 最接近。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClosestApproach {
    /// 時刻（`times_s` と同じ基準の秒）
    pub t_s: f64,
    /// 距離 [m]
    pub distance_m: f64,
    /// 最小が探索した期間の端（まだ近づいている・もう離れている）で、期間の中に極小が無い
    pub at_boundary: bool,
}

/// 粗い格子 `times_s`（昇順）での距離 `distance_m` [m] から最接近を求める。`measure(times)` は任意の時刻の列での
/// 距離を返す関数（細かい格子の測り直しに 1 回だけ呼ぶ）。時刻と距離の長さが違う・2 点に満たないときは `InvalidInput`。
pub fn closest_approach(
    times_s: &[f64],
    distance_m: &[f64],
    measure: impl Fn(&[f64]) -> Result<Vec<f64>, AstroError>,
) -> Result<ClosestApproach, AstroError> {
    if times_s.len() != distance_m.len() || times_s.len() < 2 {
        return Err(AstroError::InvalidInput(
            "Closest approach needs matching time and distance arrays of at least two samples"
                .into(),
        ));
    }
    let last = times_s.len() - 1;
    let k = argmin(distance_m);
    let (lo, hi) = (times_s[k.saturating_sub(1)], times_s[(k + 1).min(last)]);
    let fine: Vec<f64> = (0..FINE_POINTS)
        .map(|i| lo + (hi - lo) * i as f64 / (FINE_POINTS - 1) as f64)
        .collect();
    let d = measure(&fine)?;
    if d.len() != fine.len() {
        return Err(AstroError::InvalidInput(
            "The distance function returned a different number of samples".into(),
        ));
    }
    let j = argmin(&d);
    let touches_end = (k == 0 && j == 0) || (k == last && j == FINE_POINTS - 1);
    if j == 0 || j == FINE_POINTS - 1 {
        return Ok(ClosestApproach {
            t_s: fine[j],
            distance_m: d[j],
            at_boundary: touches_end,
        });
    }
    let (t, dist) = parabola_vertex(fine[j], fine[1] - fine[0], [d[j - 1], d[j], d[j + 1]]);
    Ok(ClosestApproach {
        t_s: t,
        distance_m: dist,
        at_boundary: false,
    })
}

/// 最小の添字（NaN は大きいものとして扱う）。
fn argmin(v: &[f64]) -> usize {
    v.iter()
        .enumerate()
        .fold(
            (0, f64::INFINITY),
            |(bi, bv), (i, &x)| {
                if x < bv { (i, x) } else { (bi, bv) }
            },
        )
        .0
}

/// 中心 `t0`・間隔 `h` の 3 点の距離から、距離の 2 乗への放物線の頂点の時刻と距離。曲率が正でなければ中心の点。
fn parabola_vertex(t0: f64, h: f64, d: [f64; 3]) -> (f64, f64) {
    let y = d.map(|x| x * x);
    let curvature = y[0] - 2.0 * y[1] + y[2];
    if curvature <= 0.0 || !curvature.is_finite() {
        return (t0, d[1]);
    }
    let delta = 0.5 * (y[0] - y[2]) / curvature;
    let y_min = y[1] - 0.25 * (y[0] - y[2]) * delta;
    (t0 + delta * h, y_min.max(0.0).sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2 点が相対速度 `v` [m/s] の直線で、時刻 `tc` に距離 `b` まで近づく。
    fn straight(b: f64, v: f64, tc: f64) -> impl Fn(f64) -> f64 {
        move |t| (b * b + v * v * (t - tc) * (t - tc)).sqrt()
    }

    fn measure_fn<'a>(
        f: &'a impl Fn(f64) -> f64,
    ) -> impl Fn(&[f64]) -> Result<Vec<f64>, AstroError> + 'a {
        |ts| Ok(ts.iter().map(|&t| f(t)).collect())
    }

    /// 直線の相対運動は距離の 2 乗が 2 次式なので、粗い格子の間に最小があっても厳密に求まる。
    #[test]
    fn straight_line_flyby_is_exact_between_coarse_samples() {
        let f = straight(1.2e6, 3.0e3, 8_640.0);
        let times: Vec<f64> = (0..=10).map(|i| f64::from(i) * 3_600.0).collect();
        let d: Vec<f64> = times.iter().map(|&t| f(t)).collect();
        let c = closest_approach(&times, &d, measure_fn(&f)).unwrap();
        assert!((c.t_s - 8_640.0).abs() < 1e-6, "{}", c.t_s);
        assert!((c.distance_m - 1.2e6).abs() < 1e-6, "{}", c.distance_m);
        assert!(!c.at_boundary);
    }

    /// 期間の端でまだ近づいている（極小が期間の外）なら、端を返して `at_boundary` を立てる。
    #[test]
    fn still_approaching_is_reported_as_boundary() {
        let f = straight(1.0e6, 3.0e3, 50_000.0);
        let times: Vec<f64> = (0..=10).map(|i| f64::from(i) * 3_600.0).collect();
        let d: Vec<f64> = times.iter().map(|&t| f(t)).collect();
        let c = closest_approach(&times, &d, measure_fn(&f)).unwrap();
        assert!(c.at_boundary);
        assert_eq!(c.t_s, 36_000.0);
    }

    #[test]
    fn rejects_mismatched_input() {
        let f = |ts: &[f64]| Ok(ts.to_vec());
        assert!(closest_approach(&[0.0, 1.0], &[1.0], f).is_err());
        assert!(closest_approach(&[0.0], &[1.0], f).is_err());
    }
}
