//! 時系列の集計の部品: 日照の判定、周回の切り出しと閉区間、周回平均、時間の割合、最長の区間、期間不足の注意。
//! **時系列を周回で区切って平均・割合を求めるのはここだけ。** 親の [`crate::spec`] が再公開する。

use super::{Check, Verdict};
use sat_astro::timeline::{TimelineSample, Window, time_mean};
use sat_units::typed::{Fraction, Qty, Time};
use std::ops::{Range, RangeInclusive};

/// 日照とみなす日照率の下限（半影の中央）。これ以上を「日照中」、未満を「日陰」として区間や平均を数える。
/// **日照の閾値はこれだけ**（電源・姿勢で共有する。日陰の区間は `sat_astro::timeline::shadow_windows` にこれを渡す）。
pub const SUNLIT_THRESHOLD: f64 = 0.5;

/// 時系列の 1 点が日照中か（日照率 ≥ [`SUNLIT_THRESHOLD`]）。**日照中かどうかの判定はこれを使う。**
pub fn is_sunlit(s: &TimelineSample) -> bool {
    s.sunlit >= SUNLIT_THRESHOLD
}

/// 完全な周回ごとのサンプルの添字の範囲（半開区間）。k 周目は t0 + k·T ≤ t < t0 + (k+1)·T のサンプル。
/// 回り切っていない最後の周回は含めない（境界の判定には 1e-6·T の許容を持たせる）。
/// `range.end` は次の周回の先頭の添字で、最後の周回では最後のサンプルの添字になることがある。
/// 周回平均・周回ごとの積算に使う。**周回の切り出しはここだけ。**
pub fn orbit_ranges(t: &[f64], period_s: f64) -> Vec<Range<usize>> {
    if t.is_empty() || period_s.is_nan() || period_s <= 0.0 {
        return Vec::new();
    }
    let t0 = t[0];
    let tol = 1e-6 * period_s;
    let mut out = Vec::new();
    let mut start = 0;
    let mut k = 1.0;
    for (i, &ti) in t.iter().enumerate() {
        if ti - t0 >= k * period_s - tol {
            out.push(start..i);
            start = i;
            k += 1.0;
        }
    }
    out
}

/// 周回 `r`（[`orbit_ranges`] の半開区間）を、標本 `n` 個の閉区間 `s..=e` に直す。
/// `e` は次の周回の先頭（最後の周回で末尾を越えるときは最後の標本）。**周回の閉区間はこれを使う。**
pub fn orbit_segment(r: &Range<usize>, n: usize) -> RangeInclusive<usize> {
    let last = n.saturating_sub(1);
    r.start.min(last)..=r.end.min(last)
}

/// 周回ごとの閉区間（[`orbit_segment`]）。周回が 1 つも無ければ全期間を 1 区間にする（標本が無ければ空）。
/// 1 周未満の期間を 1 周に換算する集計（永年成分など）に使う。
pub fn orbit_segments(n: usize, ranges: &[Range<usize>]) -> Vec<RangeInclusive<usize>> {
    if ranges.is_empty() {
        // 全期間を 1 周とみなした区間（0..n を閉区間にしたもの）
        return if n == 0 {
            Vec::new()
        } else {
            std::iter::once(orbit_segment(&(0..n), n)).collect()
        };
    }
    ranges.iter().map(|r| orbit_segment(r, n)).collect()
}

/// 完全な周回（[`orbit_ranges`] の半開区間）にわたる時間平均（台形則、`sat_astro::timeline::time_mean`）。
/// 周回ごとに閉区間 [`orbit_segment`] を積分する。
/// 周回が 1 つも無ければ全期間の時間平均。空なら 0。**周回平均はこれを使う。**
pub fn orbit_mean(t: &[f64], values: &[f64], ranges: &[Range<usize>]) -> f64 {
    let n = t.len().min(values.len());
    if n == 0 {
        return 0.0;
    }
    if ranges.is_empty() {
        let m = time_mean(&t[..n], &values[..n]);
        return if m.is_finite() { m } else { 0.0 };
    }
    let (mut area, mut span) = (0.0, 0.0);
    for r in ranges {
        let seg = orbit_segment(r, n);
        let (s, e) = (*seg.start(), *seg.end());
        let duration = t[e] - t[s];
        if duration > 0.0 {
            area += time_mean(&t[seg.clone()], &values[seg]) * duration;
            span += duration;
        }
    }
    if span > 0.0 {
        area / span
    } else {
        values[ranges[0].start.min(n - 1)]
    }
}

/// `mask` が真の時間だけの時間平均（台形則。`∫ v·m dt / ∫ m dt`、m は 0/1）。真の時間が無ければ `None`。
/// 例: 日照中の平均発電。**条件付きの時間平均はこれを使う。**
pub fn masked_time_mean(t: &[f64], values: &[f64], mask: &[bool]) -> Option<f64> {
    let m = indicator(mask);
    let vm: Vec<f64> = values.iter().zip(&m).map(|(v, w)| v * w).collect();
    let den = time_mean(t, &m);
    if den.is_finite() && den > 0.0 {
        Some(time_mean(t, &vm) / den)
    } else {
        None
    }
}

/// `flags` が真の時間の割合（0/1 の時間平均、[`orbit_mean`]）。`ranges` が空なら全期間で数え、標本が無ければ 0。
/// 例: 日照の割合・禁止角への侵入の割合。**時間の割合はこれを使う。**
pub fn time_fraction(t: &[f64], flags: &[bool], ranges: &[Range<usize>]) -> Fraction {
    Fraction::new(orbit_mean(t, &indicator(flags), ranges))
}

/// 真を 1、偽を 0 にした列。
fn indicator(flags: &[bool]) -> Vec<f64> {
    flags.iter().map(|&b| f64::from(u8::from(b))).collect()
}

/// 区間のうち最も長いものの長さ（区間が無ければ 0）。例: 最長の日陰・最長の侵入。
pub fn longest_window(windows: &[Window]) -> Qty<Time> {
    windows
        .iter()
        .map(|w| w.duration)
        .fold(Qty::ZERO, |a, d| if d > a { d } else { a })
}

/// 期間が 1 周に満たないとき、周回の集計を何で代えたか（[`short_span_check`] の文言）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanFallback {
    /// 軌道平均を全期間の平均で代えた
    OrbitMeanOverSpan,
    /// 1 周あたりの永年成分を全期間から外挿した
    SecularExtrapolated,
}

/// 解析期間が 1 周に満たない（`complete_orbits == 0`）ときの注意（key は `span`）。**期間不足の注意はこれを使う。**
pub fn short_span_check(complete_orbits: usize, fallback: SpanFallback) -> Option<Check> {
    if complete_orbits > 0 {
        return None;
    }
    let consequence = match fallback {
        SpanFallback::OrbitMeanOverSpan => "軌道平均は全期間の平均です",
        SpanFallback::SecularExtrapolated => "永年成分は全期間から外挿しています",
    };
    Some(Check::new(
        "span",
        "解析期間",
        Verdict::Warning,
        format!("解析期間が 1 周に満たないため、{consequence}"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbit_mean_is_time_weighted() {
        // 周期 100 s、刻みが不揃い（0, 10, 100, 110, 200）。v = t mod 100 の鋸波の 1 周の時間平均
        let t = [0.0, 10.0, 100.0, 110.0, 200.0];
        let v = [0.0, 10.0, 0.0, 10.0, 0.0];
        let r = orbit_ranges(&t, 100.0);
        assert_eq!(r.len(), 2);
        // 台形則: [0,10] 平均 5 ×10 + [10,100] 平均 5 ×90 → 5
        assert!((orbit_mean(&t, &v, &r) - 5.0).abs() < 1e-12);
        // 周回が無ければ全期間の時間平均
        assert!((orbit_mean(&t[..2], &v[..2], &[]) - 5.0).abs() < 1e-12);
        assert_eq!(orbit_mean(&[], &[], &[]), 0.0);
    }

    #[test]
    fn masked_mean_by_hand() {
        // 0〜20 s が真、v = 4 → 平均 4。真が無ければ None
        let t = [0.0, 10.0, 20.0, 30.0];
        let v = [4.0, 4.0, 4.0, 100.0];
        let m = [true, true, true, false];
        let got = masked_time_mean(&t, &v, &m).unwrap();
        assert!((got - 4.0).abs() < 1e-12);
        assert!(masked_time_mean(&t, &v, &[false; 4]).is_none());
    }

    #[test]
    fn orbit_ranges_full_orbits_only() {
        let t: Vec<f64> = (0..=25).map(|i| f64::from(i) * 10.0).collect(); // 0..250 s
        let r = orbit_ranges(&t, 100.0);
        assert_eq!(r, vec![0..10, 10..20]);
        assert!(orbit_ranges(&t, 0.0).is_empty());
    }

    #[test]
    fn segments_are_closed_and_fall_back_to_the_whole_span() {
        // 0..250 s（26 標本）: 2 周 [0, 10]・[10, 20]。周回が無ければ全期間 [0, 25]
        let t: Vec<f64> = (0..=25).map(|i| f64::from(i) * 10.0).collect();
        let r = orbit_ranges(&t, 100.0);
        assert_eq!(orbit_segments(t.len(), &r), vec![0..=10, 10..=20]);
        assert_eq!(orbit_segments(t.len(), &[]), vec![0..=25]);
        assert!(orbit_segments(0, &[]).is_empty());
        // 最後の周回の終わりが末尾を越えるときは最後の標本で切る
        assert_eq!(orbit_segment(&(20..30), 26), 20..=25);
    }

    #[test]
    fn time_fraction_by_hand() {
        // 周期 100 s を 10 s 刻みで 250 s。各周の 0〜40 s が真。台形則で 1 周に真は 40 + 5（落ち際）+ 5（立ち上がり）= 50 s
        let t: Vec<f64> = (0..=25).map(|k| f64::from(k) * 10.0).collect();
        let flags: Vec<bool> = t.iter().map(|&x| x % 100.0 < 50.0).collect();
        let r = orbit_ranges(&t, 100.0);
        assert_eq!(r.len(), 2);
        assert!((time_fraction(&t, &flags, &r).value() - 0.5).abs() < 1e-12);
        // 周回を渡さなければ全期間: 2 周の 100 s ＋ 200〜250 s の 40 + 5 s → 145 / 250
        assert!((time_fraction(&t, &flags, &[]).value() - 0.58).abs() < 1e-12);
        assert_eq!(time_fraction(&[], &[], &[]).value(), 0.0);
    }

    #[test]
    fn sunlit_at_the_middle_of_the_penumbra() {
        // 日照率 0.5（半影の中央）からが日照中
        let sample = |sunlit: f64| TimelineSample {
            t_s: 0.0,
            epoch: sat_astro::time::Epoch::parse("2027-04-01T00:00:00Z").unwrap(),
            r_eci: sat_astro::math::Vec3::X,
            v_eci: sat_astro::math::Vec3::Y,
            sun_dir_eci: sat_astro::math::Vec3::X,
            sun_distance_m: sat_astro::constants::AU,
            sunlit,
            q_body_to_eci: sat_astro::math::Quat::IDENTITY,
        };
        assert!(is_sunlit(&sample(1.0)) && is_sunlit(&sample(0.5)));
        assert!(!is_sunlit(&sample(0.49)) && !is_sunlit(&sample(0.0)));
    }

    #[test]
    fn longest_window_and_short_span() {
        let w = |start: f64, end: f64| Window {
            start: Qty::from_raw(start),
            end: Qty::from_raw(end),
            duration: Qty::from_raw(end - start),
        };
        assert_eq!(
            longest_window(&[w(0.0, 30.0), w(50.0, 120.0)]).value(),
            70.0
        );
        assert_eq!(longest_window(&[]).value(), 0.0);
        assert!(short_span_check(1, SpanFallback::OrbitMeanOverSpan).is_none());
        let c = short_span_check(0, SpanFallback::SecularExtrapolated).unwrap();
        assert_eq!((c.key.as_str(), c.verdict), ("span", Verdict::Warning));
    }
}
