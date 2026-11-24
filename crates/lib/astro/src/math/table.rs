//! 1 変数の表（引数が増える順の点）の補間。**表を引く規則（間の結び方と、表の外を外挿しないこと）はここだけ**。
//!
//! 引数は線形、値は線形か対数（[`ValueScale`]）で結ぶ。表の範囲の外は外挿せず `None` を返す
//! （端の値を黙って使わない。呼び出し側が「評価しない」と報告する）。

/// 点の間で値をどう結ぶか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueScale {
    /// 値を引数の一次式で結ぶ（区分線形）
    Linear,
    /// 値の対数を引数の一次式で結ぶ（値が正で、桁をまたいで変わる量。線量の深さ分布など）。値はすべて正であること
    Log,
}

/// 表（`xs` が厳密に増える順、`ys` が同じ長さ）を `x` で引く。範囲 `[xs.first, xs.last]` の外、
/// 点が無い・長さが合わない・非有限・`Log` で値が正でない・`x` が非有限のときは `None`。
/// 点の上では表の値そのものを返す。
pub fn interpolate(xs: &[f64], ys: &[f64], x: f64, scale: ValueScale) -> Option<f64> {
    if xs.is_empty() || xs.len() != ys.len() || !x.is_finite() {
        return None;
    }
    if xs.iter().chain(ys).any(|v| !v.is_finite()) || xs.windows(2).any(|w| w[1] <= w[0]) {
        return None;
    }
    if scale == ValueScale::Log && ys.iter().any(|&y| y <= 0.0) {
        return None;
    }
    let (first, last) = (xs[0], xs[xs.len() - 1]);
    if x < first || x > last {
        return None;
    }
    let hi = xs.partition_point(|&v| v < x);
    if xs[hi] == x {
        return Some(ys[hi]);
    }
    let (x0, x1, y0, y1) = (xs[hi - 1], xs[hi], ys[hi - 1], ys[hi]);
    let t = (x - x0) / (x1 - x0);
    Some(match scale {
        ValueScale::Linear => y0 + t * (y1 - y0),
        ValueScale::Log => (y0.ln() + t * (y1.ln() - y0.ln())).exp(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const XS: [f64; 3] = [1.0, 3.0, 7.0];
    const YS: [f64; 3] = [100.0, 4.0, 1.0];

    #[test]
    fn a_tabulated_point_returns_its_value() {
        for (x, y) in XS.iter().zip(YS) {
            assert_eq!(interpolate(&XS, &YS, *x, ValueScale::Log), Some(y));
            assert_eq!(interpolate(&XS, &YS, *x, ValueScale::Linear), Some(y));
        }
    }

    /// 対数では中点は 2 値の幾何平均: sqrt(100 × 4) = 20。線形では算術平均 52。
    #[test]
    fn the_midpoint_is_the_geometric_mean_for_log_and_the_mean_for_linear() {
        let log = interpolate(&XS, &YS, 2.0, ValueScale::Log).unwrap();
        assert!((log - 20.0).abs() < 1e-12, "{log}");
        assert_eq!(interpolate(&XS, &YS, 2.0, ValueScale::Linear), Some(52.0));
    }

    #[test]
    fn outside_the_table_is_none() {
        assert_eq!(interpolate(&XS, &YS, 0.5, ValueScale::Log), None);
        assert_eq!(interpolate(&XS, &YS, 7.5, ValueScale::Linear), None);
    }

    #[test]
    fn a_bad_table_is_none() {
        assert_eq!(interpolate(&[], &[], 1.0, ValueScale::Linear), None);
        assert_eq!(
            interpolate(&[1.0, 1.0], &[1.0, 2.0], 1.0, ValueScale::Linear),
            None
        );
        assert_eq!(
            interpolate(&[1.0, 2.0], &[1.0, 0.0], 1.5, ValueScale::Log),
            None
        );
        assert_eq!(
            interpolate(&[1.0, 2.0], &[1.0], 1.5, ValueScale::Linear),
            None
        );
    }

    #[test]
    fn a_single_point_table_answers_only_at_that_point() {
        assert_eq!(interpolate(&[2.0], &[5.0], 2.0, ValueScale::Log), Some(5.0));
        assert_eq!(interpolate(&[2.0], &[5.0], 2.1, ValueScale::Log), None);
    }
}
