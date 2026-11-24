//! 標本の値がしきい値を下回っている時間（[`time_below_s`]）。量の中身（角度・距離）は知らない。

/// 標本 `(times, values)` を折れ線とみなし、値が `threshold` を**下回っている**時間 [s]。
/// 隣り合う標本の間は直線で結び、しきい値をまたぐ区間は交点で切る（両端とも下回る区間は全体、両端とも以上なら 0）。
/// `times` は昇順で、`values` と同じ長さ。標本が 2 つ未満なら 0。
pub fn time_below_s(times: &[f64], values: &[f64], threshold: f64) -> f64 {
    times
        .windows(2)
        .zip(values.windows(2))
        .map(|(t, v)| segment_below_s(t[0], t[1], v[0], v[1], threshold))
        .sum()
}

/// 区間 `[t0, t1]`（端の値 `v0`・`v1`、直線）で `threshold` を下回っている時間 [s]。
fn segment_below_s(t0: f64, t1: f64, v0: f64, v1: f64, threshold: f64) -> f64 {
    let dt = t1 - t0;
    match (v0 < threshold, v1 < threshold) {
        (true, true) => dt,
        (false, false) => 0.0,
        // 交点までの割合は、下回っている側の端からしきい値までの差 / 端の値の差
        (true, false) => dt * (threshold - v0) / (v1 - v0),
        (false, true) => dt * (threshold - v1) / (v0 - v1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 全部下回れば全区間、全部以上なら 0。
    #[test]
    fn all_below_is_the_whole_span_and_none_below_is_zero() {
        let t = [0.0, 10.0, 25.0];
        assert_eq!(time_below_s(&t, &[1.0, 2.0, 1.5], 3.0), 25.0);
        assert_eq!(time_below_s(&t, &[4.0, 5.0, 3.0], 3.0), 0.0);
    }

    /// 0 から 10 まで直線で上がる量が 4 を下回るのは、手計算で 4 秒（交点は t = 4）。
    #[test]
    fn a_crossing_is_cut_at_the_linear_intersection() {
        assert!((time_below_s(&[0.0, 10.0], &[0.0, 10.0], 4.0) - 4.0).abs() < 1e-12);
        assert!((time_below_s(&[0.0, 10.0], &[10.0, 0.0], 4.0) - 4.0).abs() < 1e-12);
    }

    /// 谷（10 → 0 → 10）がしきい値 5 を下回るのは、両側の半分ずつで 5 + 5 = 10 秒。
    #[test]
    fn a_valley_is_below_for_half_of_each_side() {
        let t = [0.0, 10.0, 20.0];
        assert!((time_below_s(&t, &[10.0, 0.0, 10.0], 5.0) - 10.0).abs() < 1e-12);
    }

    #[test]
    fn fewer_than_two_samples_give_zero() {
        assert_eq!(time_below_s(&[0.0], &[0.0], 1.0), 0.0);
        assert_eq!(time_below_s(&[], &[], 1.0), 0.0);
    }
}
