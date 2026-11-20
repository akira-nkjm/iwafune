//! 軸の目盛り（きりのよい刻み）と、目盛りの数値の文字列。数値の整形は `sat_units::format::format_number`。

use sat_units::format::format_number;

/// 10 進の桁（`floor(log10(|v|))`。0 は 0）。対数は使わず、指数表記の文字列から読む。
fn decade(v: f64) -> i32 {
    if v == 0.0 || !v.is_finite() {
        return 0;
    }
    format!("{:e}", v.abs())
        .split('e')
        .nth(1)
        .and_then(|e| e.parse().ok())
        .unwrap_or(0)
}

/// 目盛りを付けた軸の範囲。
#[derive(Debug, Clone, PartialEq)]
pub struct Axis {
    pub lo: f64,
    pub hi: f64,
    pub step: f64,
}

impl Axis {
    /// データの範囲 `[lo, hi]` を含むきりのよい範囲と刻み（目盛りは 5 本前後）。幅が 0 なら前後に広げる。
    pub fn nice(lo: f64, hi: f64, target: usize) -> Axis {
        let (lo, hi) = if hi - lo < f64::EPSILON * lo.abs().max(1.0) {
            let pad = if lo == 0.0 { 1.0 } else { lo.abs() * 0.1 };
            (lo - pad, hi + pad)
        } else {
            (lo, hi)
        };
        let raw = (hi - lo) / target.max(1) as f64;
        let mag = 10f64.powi(decade(raw));
        let norm = raw / mag;
        let step = mag
            * if norm < 1.5 {
                1.0
            } else if norm < 3.0 {
                2.0
            } else if norm < 7.0 {
                5.0
            } else {
                10.0
            };
        Axis {
            lo: (lo / step).floor() * step,
            hi: (hi / step).ceil() * step,
            step,
        }
    }

    /// 目盛りの値（`lo` から `hi` まで）。
    pub fn ticks(&self) -> Vec<f64> {
        let n = ((self.hi - self.lo) / self.step).round() as usize;
        (0..=n).map(|k| self.lo + k as f64 * self.step).collect()
    }

    /// 目盛りの値の文字列。小数の桁数は刻みで決め、すべての目盛りでそろえる。
    pub fn label(&self, v: f64) -> String {
        let decimals = (-decade(self.step)).max(0);
        let clean = (v / self.step).round() * self.step;
        if clean == 0.0 {
            return format_number(0.0, 1);
        }
        let digits = decade(clean) + decimals + 1;
        format_number(clean, digits.clamp(1, 9) as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_axis_covers_the_data_with_round_steps() {
        let a = Axis::nice(-9.84, 20.3, 6);
        assert_eq!((a.lo, a.hi, a.step), (-10.0, 25.0, 5.0));
        assert_eq!(
            a.ticks(),
            vec![-10.0, -5.0, 0.0, 5.0, 10.0, 15.0, 20.0, 25.0]
        );
        assert_eq!(a.label(-10.0), "-10");
        assert_eq!(a.label(0.0), "0");
    }

    #[test]
    fn flat_data_gets_a_width() {
        let a = Axis::nice(3.0, 3.0, 5);
        assert!(a.lo < 3.0 && a.hi > 3.0);
    }

    #[test]
    fn fractional_steps_keep_the_needed_digits() {
        let a = Axis::nice(0.0, 1.0, 5);
        assert_eq!(a.step, 0.2);
        assert_eq!(a.label(0.4), "0.4");
        assert_eq!(a.label(1.0), "1.0");
        assert_eq!(Axis::nice(0.0, 24.0, 5).label(5.0), "5");
        assert_eq!(Axis::nice(0.0, 24.0, 5).label(10.0), "10");
    }
}
