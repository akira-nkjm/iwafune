//! 複数の地上局のパスをまとめて求め、解析の開始からの経過秒の区間に直す。
//! `comm.visibility` と `comm.data_throughput` で共有する。パスの検出そのものは `sat_astro::ground::find_passes`。

use crate::spec::AnalysisError;
use sat_astro::constants::SECONDS_PER_DAY;
use sat_astro::ground::{GroundPass, GroundStation, find_passes};
use sat_astro::orbit::Trajectory;
use sat_astro::time::Epoch;
use sat_astro::timeline::OrbitSampling;

/// パスを探す窓（サンプルの取り方から求めたもの）。
#[derive(Debug, Clone)]
pub struct SearchWindow {
    /// 伝播に使う軌道（β を指定したときは RAAN を置き換えたもの。`OrbitSampling::resolved_orbit`）
    pub orbit: Trajectory,
    /// 探索の開始（軌道の元期）
    pub start: Epoch,
    /// 期間 [s]
    pub span_s: f64,
    /// 探索の刻み [s]（AOS・LOS は二分法で詰める）
    pub step_s: f64,
}

/// サンプルの取り方（期間・刻み）を検査し、パスを探す窓を返す。開始は軌道の元期（時系列と同じ）。
pub fn search_window(sampling: &OrbitSampling) -> Result<SearchWindow, AnalysisError> {
    let (span_s, step_s) = (sampling.span.value(), sampling.step.value());
    if !(span_s > 0.0 && step_s > 0.0 && step_s <= span_s) {
        return Err(AnalysisError::InvalidInput(
            "Duration and step must be positive, and the step must not exceed the duration"
                .to_owned(),
        ));
    }
    let orbit = sampling.resolved_orbit()?;
    Ok(SearchWindow {
        start: orbit.epoch(),
        orbit,
        span_s,
        step_s,
    })
}

/// すべての局のパスを AOS の順に並べて返す。
pub fn find_all_passes(
    orbit: &Trajectory,
    stations: &[GroundStation],
    start: Epoch,
    span_s: f64,
    step_s: f64,
) -> Result<Vec<GroundPass>, AnalysisError> {
    if stations.is_empty() {
        return Err(AnalysisError::InvalidInput("No ground stations".to_owned()));
    }
    let mut all = Vec::new();
    for station in stations {
        all.extend(find_passes(station, orbit, start, span_s, step_s)?);
    }
    all.sort_by(|a, b| a.aos.j2000_s.total_cmp(&b.aos.j2000_s));
    Ok(all)
}

/// パスを解析の開始からの経過秒の区間 `(AOS, LOS)` に直す（窓 `[0, span_s]` に切り詰める）。
pub fn pass_interval(pass: &GroundPass, start: Epoch, span_s: f64) -> (f64, f64) {
    let s = (pass.aos.j2000_s - start.j2000_s).clamp(0.0, span_s);
    let e = (pass.los.j2000_s - start.j2000_s).clamp(0.0, span_s);
    (s, e.max(s))
}

/// 2 つの区間の重なりの長さ [s]。
pub fn overlap_s(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.1.min(b.1) - a.0.max(b.0)).max(0.0)
}

/// 期間に含まれる日の窓 `(開始, 終了)` の一覧（最後の日は期間の終わりで切る）。
pub fn day_windows(span_s: f64) -> Vec<(f64, f64)> {
    let days = (span_s / SECONDS_PER_DAY).ceil().max(1.0) as usize;
    (0..days)
        .map(|d| {
            (
                d as f64 * SECONDS_PER_DAY,
                ((d + 1) as f64 * SECONDS_PER_DAY).min(span_s),
            )
        })
        .collect()
}

/// 昇順に並んだ区間の重なりをつなげる（複数局が同時に見える時間を二重に数えないため）。
pub fn union_sorted(intervals: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::with_capacity(intervals.len());
    for &(s, e) in intervals {
        match out.last_mut() {
            Some(last) if s <= last.1 => last.1 = last.1.max(e),
            _ => out.push((s, e)),
        }
    }
    out
}

/// comm のテストで共有する軌道と局。
#[cfg(test)]
pub(crate) mod fixtures {
    use sat_astro::ground::GroundStation;
    use sat_astro::orbit::{OrbitElements, OrbitSpec, Propagator};
    use sat_astro::time::Epoch;
    use sat_units::typed::{Angle, Qty};

    fn deg(v: f64) -> Qty<Angle> {
        Qty::from_raw(v.to_radians())
    }

    /// 高度 550 km の太陽同期軌道。
    pub(crate) fn sso_orbit() -> OrbitSpec {
        let elements = OrbitElements {
            semi_major_axis: Qty::from_raw(sat_astro::orbit::semi_major_axis_from_altitude(
                sat_astro::ephem::CentralBody::Earth,
                550.0e3,
            )),
            eccentricity: 0.0,
            inclination: deg(97.6),
            raan: deg(0.0),
            arg_perigee: deg(0.0),
            mean_anomaly: deg(0.0),
        };
        OrbitSpec::new(
            Epoch::parse("2027-04-01T00:00:00Z").unwrap(),
            elements,
            Propagator::J2,
        )
    }

    /// 1 日を 30 s 刻みで探すサンプルの取り方。
    pub(crate) fn one_day(
        orbit: impl Into<sat_astro::orbit::Trajectory>,
    ) -> sat_astro::timeline::OrbitSampling {
        sat_astro::timeline::OrbitSampling::new(
            orbit,
            Qty::from_raw(sat_astro::constants::SECONDS_PER_DAY),
            Qty::from_raw(30.0),
        )
    }

    /// 東京付近の局（最低仰角 10°）。
    pub(crate) fn tokyo() -> GroundStation {
        GroundStation {
            id: "TKY".to_owned(),
            latitude: deg(35.7),
            longitude: deg(139.8),
            altitude: Qty::from_raw(40.0),
            min_elevation: deg(10.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_and_overlap() {
        let u = union_sorted(&[(0.0, 10.0), (5.0, 12.0), (20.0, 30.0)]);
        assert_eq!(u, vec![(0.0, 12.0), (20.0, 30.0)]);
        assert_eq!(overlap_s((0.0, 10.0), (8.0, 20.0)), 2.0);
        assert_eq!(overlap_s((0.0, 10.0), (11.0, 20.0)), 0.0);
    }

    #[test]
    fn days_cover_the_span() {
        let d = day_windows(1.5 * SECONDS_PER_DAY);
        assert_eq!(
            d,
            vec![
                (0.0, SECONDS_PER_DAY),
                (SECONDS_PER_DAY, 1.5 * SECONDS_PER_DAY)
            ]
        );
    }
}
