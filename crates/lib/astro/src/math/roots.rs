//! 1 変数の根と極値の探索（二分法・黄金分割の最大と最小）。**探索の実装はここだけ**（パスの AOS/LOS・最大仰角などで使う）。

/// `f(lo)` と `f(hi)` の符号が違う区間で `f = 0` の点を二分法で求める（区間幅が `tol` 以下で止める）。
///
/// 端で 0 ならその端を返す。符号が同じ・値が有限でない・`lo > hi` なら `None`。
/// `max_iter` 回で収まらなければその時点の中点を返す。
pub fn bisect(
    f: impl Fn(f64) -> f64,
    mut lo: f64,
    mut hi: f64,
    tol: f64,
    max_iter: usize,
) -> Option<f64> {
    let (mut f_lo, f_hi) = (f(lo), f(hi));
    if !(f_lo.is_finite() && f_hi.is_finite()) || lo > hi {
        return None;
    }
    if f_lo == 0.0 {
        return Some(lo);
    }
    if f_hi == 0.0 {
        return Some(hi);
    }
    if (f_lo > 0.0) == (f_hi > 0.0) {
        return None;
    }
    for _ in 0..max_iter {
        if hi - lo <= tol {
            break;
        }
        let mid = 0.5 * (lo + hi);
        let f_mid = f(mid);
        if (f_mid > 0.0) == (f_lo > 0.0) {
            lo = mid;
            f_lo = f_mid;
        } else {
            hi = mid;
        }
    }
    Some(0.5 * (lo + hi))
}

/// 区間 `[lo, hi]` で単峰な `f` の最大の点と値を黄金分割で求める（区間幅が `tol` 以下で止める）。
pub fn golden_section_max(
    f: impl Fn(f64) -> f64,
    mut lo: f64,
    mut hi: f64,
    tol: f64,
    max_iter: usize,
) -> (f64, f64) {
    let g = 0.5 * (5f64.sqrt() - 1.0);
    let (mut x1, mut x2) = (hi - g * (hi - lo), lo + g * (hi - lo));
    let (mut f1, mut f2) = (f(x1), f(x2));
    for _ in 0..max_iter {
        if hi - lo <= tol {
            break;
        }
        if f1 < f2 {
            lo = x1;
            (x1, f1) = (x2, f2);
            x2 = lo + g * (hi - lo);
            f2 = f(x2);
        } else {
            hi = x2;
            (x2, f2) = (x1, f1);
            x1 = hi - g * (hi - lo);
            f1 = f(x1);
        }
    }
    let x = 0.5 * (lo + hi);
    (x, f(x))
}

/// 区間 `[lo, hi]` で単峰な `f` の最小の点と値を黄金分割で求める（区間幅が `tol` 以下で止める）。
/// 値が同じなら区間の下側を捨てる（[`golden_section_max`] は同値なら上側を捨てる。最大を探す関数の符号を反転して代用しないのは、
/// 平らな頂点での同値の扱いが違い、結果が最後の桁・√ε の時刻で変わるため）。
pub fn golden_section_min(
    f: impl Fn(f64) -> f64,
    mut lo: f64,
    mut hi: f64,
    tol: f64,
    max_iter: usize,
) -> (f64, f64) {
    let g = 0.5 * (5f64.sqrt() - 1.0);
    let (mut x1, mut x2) = (hi - g * (hi - lo), lo + g * (hi - lo));
    let (mut f1, mut f2) = (f(x1), f(x2));
    for _ in 0..max_iter {
        if hi - lo <= tol {
            break;
        }
        if f1 < f2 {
            hi = x2;
            (x2, f2) = (x1, f1);
            x1 = hi - g * (hi - lo);
            f1 = f(x1);
        } else {
            lo = x1;
            (x1, f1) = (x2, f2);
            x2 = lo + g * (hi - lo);
            f2 = f(x2);
        }
    }
    let x = 0.5 * (lo + hi);
    (x, f(x))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bisect_finds_root() {
        let r = bisect(|x| x * x - 2.0, 0.0, 2.0, 1e-12, 200).expect("root");
        assert!((r - 2f64.sqrt()).abs() < 1e-11);
        assert_eq!(bisect(|x| x - 1.0, 1.0, 3.0, 1e-9, 100), Some(1.0));
        assert!(bisect(|x| x * x + 1.0, -1.0, 1.0, 1e-9, 100).is_none());
        assert!(bisect(|_| f64::NAN, 0.0, 1.0, 1e-9, 100).is_none());
    }

    #[test]
    fn golden_section_finds_minimum() {
        let (x, fx) = golden_section_min(|x| (x - 0.3).powi(2) - 5.0, -1.0, 2.0, 1e-9, 200);
        assert!(
            (x - 0.3).abs() < 1e-7 && (fx + 5.0).abs() < 1e-13,
            "{x} {fx}"
        );
        // 端が最小（単調）なら端の近くに寄る
        let (x, _) = golden_section_min(|x| x, 0.0, 1.0, 1e-9, 200);
        assert!(x < 1e-8);
    }

    #[test]
    fn golden_section_finds_maximum() {
        let (x, fx) = golden_section_max(|x| -(x - 0.3).powi(2) + 5.0, -1.0, 2.0, 1e-9, 200);
        // 頂点付近は f が x の 2 次でしか変わらないので、x の精度は √ε 程度が限界
        assert!((x - 0.3).abs() < 1e-6 && (fx - 5.0).abs() < 1e-12);
        let (xs, _) = golden_section_max(f64::sin, 0.0, 3.0, 1e-10, 200);
        assert!((xs - core::f64::consts::FRAC_PI_2).abs() < 1e-6);
    }
}
