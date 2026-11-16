//! 地球の重力場（EGM96 の球面調和関数、70 次まで）。
//!
//! 係数は `data/egm96_to70.txt`（完全正規化 C̄・S̄）を埋め込み、最初に使うときに 1 度だけ
//! 非正規化して保持する。加速度は Montenbruck & Gill, *Satellite Orbits* §3.2.5 の V/W 漸化式
//! （Cunningham）で ECEF において求める。70 次までは非正規化の係数・V/W とも倍精度の範囲に収まる。

use crate::math::Vec3;
use std::sync::OnceLock;

const EGM96_TEXT: &str = include_str!("../../data/egm96_to70.txt");

/// 埋め込んだ EGM96 の最大次数。
pub const EGM96_MAX_DEGREE: u8 = 70;

/// 非正規化した係数表。
pub(crate) struct GravityTable {
    /// 係数の μ [m³/s²]
    pub(crate) mu: f64,
    /// 係数の基準半径 [m]
    pub(crate) re: f64,
    degree: usize,
    c: Vec<f64>,
    s: Vec<f64>,
}

fn idx(n: usize, m: usize) -> usize {
    n * (n + 1) / 2 + m
}

/// 完全正規化 → 非正規化の係数 N_nm = √((2 − δ_m0)(2n + 1)(n − m)!/(n + m)!)。
fn normalization(n: usize, m: usize) -> f64 {
    let ratio: f64 = ((n - m + 1)..=(n + m)).map(|k| 1.0 / k as f64).product();
    let delta = if m == 0 { 1.0 } else { 2.0 };
    (delta * (2 * n + 1) as f64 * ratio).sqrt()
}

fn parse(text: &str) -> Option<GravityTable> {
    let (mut mu, mut re) = (None, None);
    let mut rows = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut tok = line.split_whitespace();
        match tok.next()? {
            "MU" => mu = Some(tok.next()?.parse::<f64>().ok()?),
            "RE" => re = Some(tok.next()?.parse::<f64>().ok()?),
            head => {
                let n: usize = head.parse().ok()?;
                let m: usize = tok.next()?.parse().ok()?;
                let c: f64 = tok.next()?.parse().ok()?;
                let s: f64 = tok.next()?.parse().ok()?;
                if m > n {
                    return None;
                }
                rows.push((n, m, c, s));
            }
        }
    }
    let degree = rows.iter().map(|r| r.0).max()?;
    let len = idx(degree, degree) + 1;
    let (mut c, mut s) = (vec![0.0; len], vec![0.0; len]);
    c[0] = 1.0;
    for (n, m, cn, sn) in rows {
        let k = normalization(n, m);
        c[idx(n, m)] = cn * k;
        s[idx(n, m)] = if m == 0 { 0.0 } else { sn * k };
    }
    Some(GravityTable {
        mu: mu?,
        re: re?,
        degree,
        c,
        s,
    })
}

/// 埋め込みの EGM96（解釈に失敗したら `None`）。
pub(crate) fn egm96() -> Option<&'static GravityTable> {
    static TABLE: OnceLock<Option<GravityTable>> = OnceLock::new();
    TABLE.get_or_init(|| parse(EGM96_TEXT)).as_ref()
}

impl GravityTable {
    /// 2 次だけの場（完全正規化の `[C̄20, C̄21, S̄21, C̄22, S̄22]`、中心項なし）。固体潮汐の変分に使う。
    pub(crate) fn degree_two(mu: f64, re: f64, cs: [f64; 5]) -> GravityTable {
        let (mut c, mut s) = (vec![0.0; 6], vec![0.0; 6]);
        c[idx(2, 0)] = cs[0] * normalization(2, 0);
        c[idx(2, 1)] = cs[1] * normalization(2, 1);
        s[idx(2, 1)] = cs[2] * normalization(2, 1);
        c[idx(2, 2)] = cs[3] * normalization(2, 2);
        s[idx(2, 2)] = cs[4] * normalization(2, 2);
        GravityTable {
            mu,
            re,
            degree: 2,
            c,
            s,
        }
    }

    #[cfg(test)]
    /// 完全正規化の係数 (C̄_nm, S̄_nm)。表の外は 0。
    pub(crate) fn normalized(&self, n: usize, m: usize) -> (f64, f64) {
        if n > self.degree || m > n {
            return (0.0, 0.0);
        }
        let k = normalization(n, m);
        (self.c[idx(n, m)] / k, self.s[idx(n, m)] / k)
    }

    /// ECEF の位置 [m] での加速度（ECEF）[m/s²]。次数 `min_degree` 以上の項の和（0 なら中心項を含む）。
    pub(crate) fn acceleration_ecef(
        &self,
        r: Vec3,
        degree: usize,
        order: usize,
        min_degree: usize,
    ) -> Vec3 {
        let degree = degree.min(self.degree);
        let order = order.min(degree);
        let r2 = r.norm_squared();
        if r2.is_nan() || r2 <= 0.0 {
            return Vec3::ZERO;
        }
        let nmax = degree + 1;
        let (v, w) = self.vw(r, r2, nmax);
        let mut acc = Vec3::ZERO;
        for m in 0..=order {
            for n in m.max(min_degree)..=degree {
                let (c, s) = (self.c[idx(n, m)], self.s[idx(n, m)]);
                if c == 0.0 && s == 0.0 {
                    continue;
                }
                let k = (n - m + 1) as f64;
                let az = k * (-c * v[idx(n + 1, m)] - s * w[idx(n + 1, m)]);
                let (ax, ay) = if m == 0 {
                    (-c * v[idx(n + 1, 1)], -c * w[idx(n + 1, 1)])
                } else {
                    let f = 0.5 * k * (k + 1.0);
                    let (vp, wp) = (v[idx(n + 1, m + 1)], w[idx(n + 1, m + 1)]);
                    let (vm, wm) = (v[idx(n + 1, m - 1)], w[idx(n + 1, m - 1)]);
                    (
                        0.5 * (-c * vp - s * wp) + f * (c * vm + s * wm),
                        0.5 * (-c * wp + s * vp) + f * (-c * wm + s * vm),
                    )
                };
                acc += Vec3::new(ax, ay, az);
            }
        }
        acc * (self.mu / (self.re * self.re))
    }

    /// V_nm・W_nm（n ≤ nmax）。
    fn vw(&self, r: Vec3, r2: f64, nmax: usize) -> (Vec<f64>, Vec<f64>) {
        let len = idx(nmax, nmax) + 1;
        let (mut v, mut w) = (vec![0.0; len], vec![0.0; len]);
        let re = self.re;
        let (x0, y0, z0) = (re * r.x / r2, re * r.y / r2, re * r.z / r2);
        let rho = re * re / r2;
        v[0] = re / r2.sqrt();
        for m in 0..=nmax {
            if m > 0 {
                let (vp, wp) = (v[idx(m - 1, m - 1)], w[idx(m - 1, m - 1)]);
                let k = (2 * m - 1) as f64;
                v[idx(m, m)] = k * (x0 * vp - y0 * wp);
                w[idx(m, m)] = k * (x0 * wp + y0 * vp);
            }
            if m < nmax {
                let k = (2 * m + 1) as f64 * z0;
                v[idx(m + 1, m)] = k * v[idx(m, m)];
                w[idx(m + 1, m)] = k * w[idx(m, m)];
            }
            for n in (m + 2)..=nmax {
                let a = (2 * n - 1) as f64 * z0;
                let b = (n + m - 1) as f64 * rho;
                let d = (n - m) as f64;
                v[idx(n, m)] = (a * v[idx(n - 1, m)] - b * v[idx(n - 2, m)]) / d;
                w[idx(n, m)] = (a * w[idx(n - 1, m)] - b * w[idx(n - 2, m)]) / d;
            }
        }
        (v, w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::J2_EARTH;

    fn table() -> &'static GravityTable {
        egm96().expect("EGM96 parses")
    }

    #[test]
    fn embedded_table_is_complete() {
        let t = table();
        assert_eq!(t.degree, usize::from(EGM96_MAX_DEGREE));
        assert!((t.mu - 3.986_004_415e14).abs() < 1.0);
        assert!((t.re - 6_378_136.3).abs() < 1e-6);
        let (c70, _) = t.normalized(70, 70);
        assert!((c70 - -4.703_751_388_26e-10).abs() < 1e-22);
    }

    /// EGM96 の C̄20 から J2 = −√5·C̄20 が定数の J2 と一致する。
    #[test]
    fn j2_from_egm96_matches_constant() {
        let (c20, _) = table().normalized(2, 0);
        let j2 = -(5f64).sqrt() * c20;
        assert!((j2 / J2_EARTH - 1.0).abs() < 1e-8, "{j2}");
    }

    /// 2×0 の調和関数の加速度は、閉じた式の点質量＋J2 と一致する（同じ μ・R・J2 で）。
    #[test]
    fn degree_two_zonal_matches_closed_form() {
        let t = table();
        let (c20, _) = t.normalized(2, 0);
        let j2 = -(5f64).sqrt() * c20;
        let r = Vec3::new(4.1e6, -3.3e6, 4.4e6);
        let a = t.acceleration_ecef(r, 2, 0, 0);
        let rn = r.norm();
        let k = 1.5 * j2 * (t.re / rn).powi(2);
        let z2 = (r.z / rn).powi(2);
        let f = -t.mu / rn.powi(3);
        let expected = Vec3::new(
            f * r.x * (1.0 + k * (1.0 - 5.0 * z2)),
            f * r.y * (1.0 + k * (1.0 - 5.0 * z2)),
            f * r.z * (1.0 + k * (3.0 - 5.0 * z2)),
        );
        assert!(
            (a - expected).norm() / expected.norm() < 1e-13,
            "{a:?} vs {expected:?}"
        );
    }

    #[test]
    fn high_degree_terms_are_small_and_converge() {
        let t = table();
        let r = Vec3::new(3.0e6, 5.2e6, 3.4e6)
            .normalized()
            .expect("nonzero")
            * 6.93e6;
        let a = |n: usize| t.acceleration_ecef(r, n, n, 0);
        let d_high = (a(70) - a(2)).norm();
        assert!(d_high > 1e-6 && d_high < 1e-3, "{d_high}");
        assert!((a(70) - a(60)).norm() < (a(20) - a(10)).norm());
        assert!(a(70).x.is_finite());
    }
}
