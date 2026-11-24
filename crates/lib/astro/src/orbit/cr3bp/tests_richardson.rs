//! Richardson の 3 次の係数の検証。出典に頼らない検査（特性方程式・厳密な運動方程式への代入）と、本が載せている数値との照合。

use super::*;

fn sun_earth_moon() -> Cr3bp {
    Cr3bp::from_bodies(CentralBody::Sun, Secondary::EarthMoonSystem).expect("mu")
}

fn earth_moon() -> Cr3bp {
    Cr3bp::from_bodies(CentralBody::Earth, Secondary::Body(CentralBody::Moon)).expect("mu")
}

/// 面内の振動数 λ は線形化の特性方程式 `λ⁴ + (c2 − 2)λ² − (c2 − 1)(1 + 2c2) = 0` を満たし、`κ = (λ² + 1 + 2c2)/(2λ)` は
/// 線形解 `x = −cos τ`、`y = κ sin τ` を運動方程式の x・y の式に代入して合う比（係数 κ の導出の確認）。
#[test]
fn linear_frequencies_satisfy_the_characteristic_equation() {
    for (cr, p) in [
        (sun_earth_moon(), LibrationPoint::L1),
        (sun_earth_moon(), LibrationPoint::L2),
        (earth_moon(), LibrationPoint::L1),
        (earth_moon(), LibrationPoint::L2),
    ] {
        let c = RichardsonCoefficients::new(&cr, p).expect("coefficients");
        let (l2, c2) = (c.lambda * c.lambda, c.c2);
        let char_eq = l2 * l2 + (c2 - 2.0) * l2 - (c2 - 1.0) * (1.0 + 2.0 * c2);
        assert!(char_eq.abs() < 1e-11 * (1.0 + l2 * l2), "{p:?} {char_eq:e}");
        // x の式 `ẍ − 2ẏ − (1+2c2)x = 0` の cos τ の係数: λ² − 2κλ + (1 + 2c2)
        let residual = l2 - 2.0 * c.k * c.lambda + (1.0 + 2.0 * c2);
        assert!(residual.abs() < 1e-11 * (1.0 + l2), "{p:?} {residual:e}");
        // y の式 `ÿ + 2ẋ + (c2 − 1)y = 0`（ẋ = λ sin τ）の sin τ の係数: κ(c2 − 1 − λ²) + 2λ
        let y_residual = c.k * (c2 - 1.0 - l2) + 2.0 * c.lambda;
        assert!(
            y_residual.abs() < 1e-10 * (1.0 + l2),
            "{p:?} {y_residual:e}"
        );
    }
}

/// 本（Koon ほか §6.6）が太陽–地球 L1 について載せている `s1`・`s2`・`l1`・`l2`（10 桁）との照合（**値の確認であって、調整した期待値ではない**）。
/// 許容は相対 1e-6: 質量比は地球と月を足した GM から作るが、本がどの質量比を使ったかは分からず、
/// 値は質量比に弱く効く（質量比 1% の変化で s1 が 2e-4）ので、質量比の 5 桁目までの違いが 1e-6 の桁になる。
#[test]
fn coefficients_match_the_published_sun_earth_l1_values() {
    let c = RichardsonCoefficients::new(&sun_earth_moon(), LibrationPoint::L1).expect("c");
    for (name, got, published) in [
        ("s1", c.s1, -8.246608317e-1),
        ("s2", c.s2, 1.210985938e-1),
        ("l1", c.l1, -1.596560314e1),
        ("l2", c.l2, 1.740900800),
        ("delta", c.delta, 0.29221),
    ] {
        let tol = if name == "delta" { 1e-4 } else { 1e-6 };
        assert!(
            ((got - published) / published).abs() < tol,
            "{name}: {got} vs {published}"
        );
    }
}

/// 3 次の解を厳密な運動方程式に代入した残差（局所の単位。加速度の差）。解の振幅を `s` 倍にすると、
/// 3 次までを正しく解いていれば残差は `s⁴` で小さくなり（振幅を 2 倍にすると 16 倍）、3 次の係数のどれかが違えば `s³`（8 倍）にしかならない。
///
/// 検査の組み立て（解の形に由来する 2 つの扱い）:
/// - 振幅の関係式 `l1·ax² + l2·az² + Δ = 0` は `Δ` を振幅の 2 乗の量として扱う理論なので、振幅を変えるたびに
///   `Δ_s = −(l1·ax² + l2·az²)` を使い、運動方程式の鉛直の線形項の差 `(Δ_s − Δ)·z` を引く。
/// - 解は基本調和（x の cos τ₁、y の sin τ₁）の 3 次の補正を持たない（振幅と y/x の比の定義に吸収される自由度）。
///   このため x・y の基本調和の残差は 3 乗で残る。ただし線形の演算子は基本調和で階数が 1 つ足りないので、その残差が
///   打ち消せるかどうか（永年項が消えているか）は、演算子の零ベクトル `u = (2λ, −(λ² + 1 + 2c2))` への射影が 4 乗で小さいことで見る
///   （`s1`・`s2` の式の検査）。基本調和の残りは検査から外し、ほかの調和（2・3・4 倍）と z は全部見る。
///
/// 戻りは [`Residual`]。
/// 残差の調和ごとの大きさ。
struct Residual {
    /// 基本調和の永年項（零ベクトルへの射影）
    secular: f64,
    /// 0 倍・2 倍の調和（2 次の係数の検査）の最大成分
    even: f64,
    /// 3 倍の調和（3 次の係数 `a31`〜`d32` の検査）の最大成分。正しければ 4 次の解は 3 倍の調和を持たない（5 次以降）ので 5 乗で小さくなる
    third: f64,
}

fn series_residual(cr: &Cr3bp, p: LibrationPoint, scale: f64) -> Residual {
    let c = RichardsonCoefficients::new(cr, p).expect("c");
    let (ax, az) = (scale, 0.8 * scale);
    let w = c.lambda * (1.0 + c.s1 * ax * ax + c.s2 * az * az);
    let delta_s = -(c.l1 * ax * ax + c.l2 * az * az);
    let x_point = cr.libration_point(p).expect("point")[0];
    let (n, h) = (16, 1e-3);
    let phase = |k: usize| std::f64::consts::TAU * k as f64 / n as f64 + 0.1;
    let diffs: Vec<[f64; 3]> = (0..n)
        .map(|k| {
            let tau = phase(k);
            let v = |t: f64| c.local_state(ax, az, 1.0, t);
            let mut g: State6 = std::array::from_fn(|i| v(tau)[i] * c.gamma);
            g[0] += x_point;
            let exact = cr.derivative(&g);
            std::array::from_fn(|i| {
                // 速度の位相微分（4 次の中心差分）
                let dv = (-v(tau + 2.0 * h)[3 + i] + 8.0 * v(tau + h)[3 + i]
                    - 8.0 * v(tau - h)[3 + i]
                    + v(tau - 2.0 * h)[3 + i])
                    / (12.0 * h);
                let lin = if i == 2 {
                    (delta_s - c.delta) * g[2]
                } else {
                    0.0
                };
                (c.gamma * w * dv - exact[3 + i] - lin) / c.gamma
            })
        })
        .collect();
    // 調和 m の振幅（成分 i）
    let harmonic = |i: usize, m: usize| -> f64 {
        let (mut cc, mut ss) = (0.0, 0.0);
        for (k, d) in diffs.iter().enumerate() {
            let t = m as f64 * phase(k);
            cc += d[i] * t.cos();
            ss += d[i] * t.sin();
        }
        (cc * cc + ss * ss).sqrt() * 2.0 / n as f64
    };
    // 基本調和の係数（x は cos τ₁、y は sin τ₁。位相のずれ 0.1 を戻す）
    let coefficient = |i: usize, f: fn(f64) -> f64| -> f64 {
        diffs
            .iter()
            .enumerate()
            .map(|(k, d)| d[i] * f(phase(k)))
            .sum::<f64>()
            * 2.0
            / n as f64
    };
    let (cx, sy) = (coefficient(0, f64::cos), coefficient(1, f64::sin));
    let u = (2.0 * c.lambda, -(c.lambda * c.lambda + 1.0 + 2.0 * c.c2));
    Residual {
        secular: (u.0 * cx + u.1 * sy).abs(),
        even: (0..3).fold(0.0_f64, |m, i| {
            m.max(harmonic(i, 0) / 2.0).max(harmonic(i, 2))
        }),
        third: (0..3).fold(0.0_f64, |m, i| m.max(harmonic(i, 3))),
    }
}

#[test]
fn third_order_solution_has_a_higher_order_residual() {
    for (cr, p) in [
        (sun_earth_moon(), LibrationPoint::L1),
        (sun_earth_moon(), LibrationPoint::L2),
        (earth_moon(), LibrationPoint::L1),
        (earth_moon(), LibrationPoint::L2),
    ] {
        let r: Vec<Residual> = [0.08, 0.04, 0.02]
            .iter()
            .map(|&s| series_residual(&cr, p, s))
            .collect();
        for pair in r.windows(2) {
            // 2 次の係数: 残差は 4 次（16 倍。誤りなら 2 次で 4 倍）
            let even = pair[0].even / pair[1].even;
            assert!((12.0..20.0).contains(&even), "{p:?} even ratio {even}");
            // 3 次の係数: 3 倍の調和の残差は 5 次以降（32 倍。誤りなら 3 次で 8 倍）
            let third = pair[0].third / pair[1].third;
            assert!(third > 20.0, "{p:?} third-harmonic ratio {third}");
            // 永年項の残り: 4 次以上
            let secular = pair[0].secular / pair[1].secular;
            assert!(secular > 12.0, "{p:?} secular ratio {secular}");
        }
    }
}
