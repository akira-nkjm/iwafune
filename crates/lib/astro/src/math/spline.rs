//! 1 変数の滑らかな補間: 自然 3 次スプライン（[`NaturalSpline`]）と、表の端の区間の直線で外へ延ばす折れ線（[`ExtendedLinear`]）。
//!
//! `table.rs` の `interpolate` は表の外を引かない（`None`）のに対し、ここの 2 つは**端の区間の多項式をそのまま延ばして**外も答える
//! （放射線の遮蔽線量の核の表のように、手法の側が端の外挿を前提にする使い方のため。外へどこまで許すかは呼び出し側が決める）。
//! 自然 3 次スプラインは両端の 2 階微分が 0（Reinsch の方法と同じ曲線）。

/// 自然 3 次スプライン。節点 `x`（厳密に増える順、2 点以上）での値 `y` を通り、両端の 2 階微分が 0。
#[derive(Debug, Clone)]
pub struct NaturalSpline {
    x: Vec<f64>,
    y: Vec<f64>,
    /// 区間 j の多項式 `y_j + b (s-x_j) + c (s-x_j)² + d (s-x_j)³` の係数
    b: Vec<f64>,
    c: Vec<f64>,
    d: Vec<f64>,
}

/// 節点が 2 点未満・長さ違い・増える順でない・非有限のとき `None`。
fn valid(x: &[f64], y: &[f64]) -> bool {
    x.len() >= 2
        && x.len() == y.len()
        && x.iter().chain(y).all(|v| v.is_finite())
        && x.windows(2).all(|w| w[1] > w[0])
}

impl NaturalSpline {
    /// 節点から作る。条件を満たさなければ `None`。
    pub fn new(x: &[f64], y: &[f64]) -> Option<Self> {
        if !valid(x, y) {
            return None;
        }
        let n = x.len();
        let h: Vec<f64> = x.windows(2).map(|w| w[1] - w[0]).collect();
        let slope: Vec<f64> = (0..n - 1).map(|j| (y[j + 1] - y[j]) / h[j]).collect();
        // 内部の節点の 2 階微分 m を、三重対角の連立方程式（トーマス法）で求める。両端は 0。
        let mut m = vec![0.0; n];
        if n > 2 {
            let k = n - 2;
            let mut diag = vec![0.0; k];
            let mut rhs = vec![0.0; k];
            for i in 0..k {
                diag[i] = 2.0 * (h[i] + h[i + 1]);
                rhs[i] = 6.0 * (slope[i + 1] - slope[i]);
            }
            for i in 1..k {
                let w = h[i] / diag[i - 1];
                diag[i] -= w * h[i];
                rhs[i] -= w * rhs[i - 1];
            }
            let mut next = 0.0;
            for i in (0..k).rev() {
                let upper = if i + 1 < k { h[i + 1] * next } else { 0.0 };
                next = (rhs[i] - upper) / diag[i];
                m[i + 1] = next;
            }
        }
        let b = (0..n - 1)
            .map(|j| slope[j] - h[j] * (2.0 * m[j] + m[j + 1]) / 6.0)
            .collect();
        let c = (0..n - 1).map(|j| m[j] / 2.0).collect();
        let d = (0..n - 1)
            .map(|j| (m[j + 1] - m[j]) / (6.0 * h[j]))
            .collect();
        Some(Self {
            x: x.to_vec(),
            y: y.to_vec(),
            b,
            c,
            d,
        })
    }

    /// 点 `s` を含む区間の番号（`x_j <= s` の最大の j。端の外は端の区間）。
    fn interval(&self, s: f64) -> usize {
        let last = self.x.len() - 2;
        self.x
            .partition_point(|&v| v <= s)
            .saturating_sub(1)
            .min(last)
    }

    /// 値。節点の外は端の区間の 3 次式を延ばす。
    pub fn eval(&self, s: f64) -> f64 {
        let j = self.interval(s);
        let q = s - self.x[j];
        ((self.d[j] * q + self.c[j]) * q + self.b[j]) * q + self.y[j]
    }

    /// 1 階微分（同じ区間の式の微分。最後の節点では最後の区間の端での値）。
    pub fn derivative(&self, s: f64) -> f64 {
        let j = self.interval(s);
        let q = s - self.x[j];
        (3.0 * self.d[j] * q + 2.0 * self.c[j]) * q + self.b[j]
    }
}

/// 折れ線（区分線形）。節点の外は端の区間の直線を延ばす。
#[derive(Debug, Clone)]
pub struct ExtendedLinear {
    x: Vec<f64>,
    y: Vec<f64>,
}

impl ExtendedLinear {
    /// 節点から作る（条件は [`NaturalSpline::new`] と同じ）。
    pub fn new(x: &[f64], y: &[f64]) -> Option<Self> {
        valid(x, y).then(|| Self {
            x: x.to_vec(),
            y: y.to_vec(),
        })
    }

    /// 値。
    pub fn eval(&self, s: f64) -> f64 {
        let last = self.x.len() - 2;
        let j = self
            .x
            .partition_point(|&v| v <= s)
            .saturating_sub(1)
            .min(last);
        let t = (s - self.x[j]) / (self.x[j + 1] - self.x[j]);
        self.y[j] + t * (self.y[j + 1] - self.y[j])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 節点を通り、3 次式 y = x³ の自然スプラインは両端の 2 階微分が 0 なので内部でだけ厳密に近い。
    /// ここでは手計算できる 3 点の場合: x = 0, 1, 2、y = 0, 1, 0。内部の 2 階微分 m1 は
    /// 2(h0+h1) m1 = 6 (s1 - s0) = 6 (-1 - 1) より m1 = -3。区間 0 の係数 b0 = 1 - (2·0 + (-3))/6 = 1.5, c0 = 0, d0 = -0.5。
    #[test]
    fn natural_spline_matches_the_hand_solution_of_three_points() {
        let s = NaturalSpline::new(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.0]).unwrap();
        assert!((s.eval(0.5) - (1.5 * 0.5 - 0.5 * 0.125)).abs() < 1e-14);
        assert!((s.eval(1.0) - 1.0).abs() < 1e-14);
        assert!((s.derivative(0.0) - 1.5).abs() < 1e-14);
        // 対称なので x = 1.5 は x = 0.5 と同じ値、最後の節点の微分は -1.5
        assert!((s.eval(1.5) - s.eval(0.5)).abs() < 1e-14);
        assert!((s.derivative(2.0) + 1.5).abs() < 1e-14);
    }

    #[test]
    fn natural_spline_reproduces_a_line_and_extends_the_end_polynomial() {
        let s = NaturalSpline::new(&[1.0, 2.0, 4.0, 7.0], &[3.0, 5.0, 9.0, 15.0]).unwrap();
        for x in [0.0, 1.5, 3.0, 8.0] {
            assert!((s.eval(x) - (2.0 * x + 1.0)).abs() < 1e-12, "{x}");
        }
        assert!(NaturalSpline::new(&[1.0], &[1.0]).is_none());
        assert!(NaturalSpline::new(&[1.0, 1.0], &[1.0, 2.0]).is_none());
    }

    #[test]
    fn extended_linear_continues_the_end_segments() {
        let l = ExtendedLinear::new(&[0.0, 1.0, 3.0], &[0.0, 2.0, 3.0]).unwrap();
        assert!((l.eval(0.5) - 1.0).abs() < 1e-15);
        assert!((l.eval(-1.0) + 2.0).abs() < 1e-15);
        assert!((l.eval(5.0) - 4.0).abs() < 1e-15);
        assert!((l.eval(3.0) - 3.0).abs() < 1e-15);
    }
}
