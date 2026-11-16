//! 温度・密度プロファイル（原典 C の `densu` / `densm` とその補助関数）。
//!
//! 原典 `nrlmsise-00.c` の該当関数を 1 対 1 で移植したもの。**式も評価順序も変えていない**
//! ——変えると原典の検証出力と f64 のビットレベルで合わなくなり、`tests`
//! の突き合わせが「移植の誤り」と「並べ替えによる丸め差」を区別できなくなるため。
//! そのぶん Rust としては不自然な箇所（早期 return の多用、`powf` の多用）が残る。

use super::State;

/// 気体定数（原典の `rgas`）`[erg/(mol·K)]` 相当。原典の CGS 単位系のまま使う。
const RGAS: f64 = 831.4;

impl State {
    /// 原典 `zeta(zz, zl)` — 幾何高度差を geopotential 的な高度差へ直す。
    fn zeta(&self, zz: f64, zl: f64) -> f64 {
        (zz - zl) * (self.re + zl) / (self.re + zz)
    }
}

/// 原典 `glatf` — 緯度による重力加速度と実効地球半径。
///
/// 戻り値は `(gsurf [cm/s²], re `[km]`)`。
pub(super) fn glatf(lat_deg: f64) -> (f64, f64) {
    let dgtr = 1.74533E-2;
    let c2 = (2.0 * dgtr * lat_deg).cos();
    let gv = 980.616 * (1.0 - 0.0026373 * c2);
    let reff = 2.0 * gv / (3.085_462E-6 + 2.27E-9 * c2) * 1.0E-5;
    (gv, reff)
}

/// 原典 `ccor` — 化学/解離の補正係数。
pub(super) fn ccor(alt: f64, r: f64, h1: f64, zh: f64) -> f64 {
    let e = (alt - zh) / h1;
    if e > 70.0 {
        return 1.0; // exp(0)
    }
    if e < -70.0 {
        return r.exp();
    }
    let ex = e.exp();
    (r / (1.0 + ex)).exp()
}

/// 原典 `ccor2` — 遷移長を 2 本持つ版の化学補正。
pub(super) fn ccor2(alt: f64, r: f64, h1: f64, zh: f64, h2: f64) -> f64 {
    let e1 = (alt - zh) / h1;
    let e2 = (alt - zh) / h2;
    if e1 > 70.0 || e2 > 70.0 {
        return 1.0; // exp(0)
    }
    if e1 < -70.0 && e2 < -70.0 {
        return r.exp();
    }
    let ccor2v = r / (1.0 + 0.5 * (e1.exp() + e2.exp()));
    ccor2v.exp()
}

/// 原典 `dnet` — 乱流圏界面（turbopause）で拡散平衡密度と完全混合密度をつなぐ。
///
/// 原典は `dm <= 0 || dd <= 0` のとき `printf` で警告して縮退処理に落ちる。ここでは
/// 出力せずに同じ縮退値を返す（ライブラリが標準出力を汚さないため）。到達するのは
/// 密度がアンダーフローした極端な入力だけで、値の意味は原典と同じ。
pub(super) fn dnet(dd: f64, dm: f64, zhm: f64, xmm: f64, xm: f64) -> f64 {
    let a = zhm / (xmm - xm);
    if !(dm > 0.0 && dd > 0.0) {
        let dd = if dd == 0.0 && dm == 0.0 { 1.0 } else { dd };
        if dm == 0.0 {
            return dd;
        }
        if dd == 0.0 {
            return dm;
        }
    }
    let ylog = a * (dm / dd).ln();
    if ylog < -10.0 {
        return dd;
    }
    if ylog > 10.0 {
        return dm;
    }
    dd * (1.0 + ylog.exp()).powf(1.0 / a)
}

/// 原典 `spline` — 3 次スプラインの 2 階微分を求める（Numerical Recipes 由来）。
///
/// `n <= 5` でしか呼ばれない（`densu` は 5 ノード、`densm` は 4〜5 ノード）ので、
/// 原典の `malloc` は固定長の作業配列で置き換える。
fn spline(x: &[f64], y: &[f64], yp1: f64, ypn: f64, y2: &mut [f64]) {
    let n = x.len();
    debug_assert!(
        (2..=5).contains(&n),
        "spline node count is assumed to be 5 or fewer"
    );
    let mut u = [0.0_f64; 5];

    if yp1 > 0.99E30 {
        y2[0] = 0.0;
        u[0] = 0.0;
    } else {
        y2[0] = -0.5;
        u[0] = (3.0 / (x[1] - x[0])) * ((y[1] - y[0]) / (x[1] - x[0]) - yp1);
    }
    for i in 1..(n - 1) {
        let sig = (x[i] - x[i - 1]) / (x[i + 1] - x[i - 1]);
        let p = sig * y2[i - 1] + 2.0;
        y2[i] = (sig - 1.0) / p;
        u[i] = (6.0
            * ((y[i + 1] - y[i]) / (x[i + 1] - x[i]) - (y[i] - y[i - 1]) / (x[i] - x[i - 1]))
            / (x[i + 1] - x[i - 1])
            - sig * u[i - 1])
            / p;
    }
    let (qn, un) = if ypn > 0.99E30 {
        (0.0, 0.0)
    } else {
        (
            0.5,
            (3.0 / (x[n - 1] - x[n - 2])) * (ypn - (y[n - 1] - y[n - 2]) / (x[n - 1] - x[n - 2])),
        )
    };
    y2[n - 1] = (un - qn * u[n - 2]) / (qn * y2[n - 2] + 1.0);
    for k in (0..=(n - 2)).rev() {
        y2[k] = y2[k] * y2[k + 1] + u[k];
    }
}

/// 原典 `splint` — 3 次スプラインの内挿値。
fn splint(xa: &[f64], ya: &[f64], y2a: &[f64], x: f64) -> f64 {
    let n = xa.len();
    let mut klo = 0_usize;
    let mut khi = n - 1;
    while (khi - klo) > 1 {
        let k = (khi + klo) / 2;
        if xa[k] > x {
            khi = k;
        } else {
            klo = k;
        }
    }
    let h = xa[khi] - xa[klo];
    let a = (xa[khi] - x) / h;
    let b = (x - xa[klo]) / h;
    a * ya[klo]
        + b * ya[khi]
        + ((a * a * a - a) * y2a[klo] + (b * b * b - b) * y2a[khi]) * h * h / 6.0
}

/// 原典 `splini` — スプラインを `xa[0]` から `x` まで積分する。
fn splini(xa: &[f64], ya: &[f64], y2a: &[f64], x: f64) -> f64 {
    let n = xa.len();
    let mut yi = 0.0;
    let mut klo = 0_usize;
    let mut khi = 1_usize;
    while x > xa[klo] && khi < n {
        let mut xx = x;
        if khi < (n - 1) && x >= xa[khi] {
            xx = xa[khi];
        }
        let h = xa[khi] - xa[klo];
        let a = (xa[khi] - xx) / h;
        let b = (xx - xa[klo]) / h;
        let a2 = a * a;
        let b2 = b * b;
        yi += ((1.0 - a2) * ya[klo] / 2.0
            + b2 * ya[khi] / 2.0
            + ((-(1.0 + a2 * a2) / 4.0 + a2 / 2.0) * y2a[klo]
                + (b2 * b2 / 4.0 - b2 / 2.0) * y2a[khi])
                * h
                * h
                / 6.0)
            * h;
        klo += 1;
        khi += 1;
    }
    yi
}

impl State {
    /// 原典 `densu` — 熱圏の温度・密度プロファイル（Bates 温度 + 下部のスプライン）。
    ///
    /// `zn1` は結合高度のノード。原典と同じく `meso_tn1[0]` / `meso_tgn1[0]` を書き換える
    /// （下端の温度と勾配は Bates プロファイルから決まるため、呼び出しごとに更新される）。
    #[allow(clippy::too_many_arguments)]
    pub(super) fn densu(
        &mut self,
        alt: f64,
        dlb: f64,
        tinf: f64,
        tlb: f64,
        xm: f64,
        alpha: f64,
        tz: &mut f64,
        zlb: f64,
        s2: f64,
        zn1: &[f64; 5],
    ) -> f64 {
        let mut densu_temp;
        let mut x = 0.0;
        let mut z1 = 0.0;
        let mut t1 = 0.0;
        let mut zgdif = 0.0;
        let mut xs = [0.0_f64; 5];
        let mut ys = [0.0_f64; 5];
        let mut y2out = [0.0_f64; 5];
        let mut mn = 0_usize;

        // Bates プロファイルとスプラインの結合高度
        let za = zn1[0];
        let z = if alt > za { alt } else { za };

        // ZLB からの geopotential 高度差
        let zg2 = self.zeta(z, zlb);

        // Bates 温度
        let tt = tinf - (tinf - tlb) * (-s2 * zg2).exp();
        let ta = tt;
        *tz = tt;
        densu_temp = *tz;

        if alt < za {
            // ZA より下の温度: ZA における Bates プロファイルの勾配を境界条件にする
            let dta = (tinf - ta) * s2 * ((self.re + zlb) / (self.re + za)).powf(2.0);
            self.meso_tgn1[0] = dta;
            self.meso_tn1[0] = ta;
            let mn1 = zn1.len();
            let z = if alt > zn1[mn1 - 1] {
                alt
            } else {
                zn1[mn1 - 1]
            };
            mn = mn1;
            z1 = zn1[0];
            let z2 = zn1[mn - 1];
            t1 = self.meso_tn1[0];
            let t2 = self.meso_tn1[mn - 1];
            let zg = self.zeta(z, z1);
            zgdif = self.zeta(z2, z1);
            for k in 0..mn {
                xs[k] = self.zeta(zn1[k], z1) / zgdif;
                ys[k] = 1.0 / self.meso_tn1[k];
            }
            let yd1 = -self.meso_tgn1[0] / (t1 * t1) * zgdif;
            let yd2 = -self.meso_tgn1[1] / (t2 * t2)
                * zgdif
                * ((self.re + z2) / (self.re + z1)).powf(2.0);
            spline(&xs[..mn], &ys[..mn], yd1, yd2, &mut y2out[..mn]);
            x = zg / zgdif;
            let y = splint(&xs[..mn], &ys[..mn], &y2out[..mn], x);
            *tz = 1.0 / y;
            densu_temp = *tz;
        }
        if xm == 0.0 {
            return densu_temp;
        }

        // ZA より上の密度
        let glb = self.gsurf / (1.0 + zlb / self.re).powf(2.0);
        let gamma = xm * glb / (s2 * RGAS * tinf);
        let mut expl = (-s2 * gamma * zg2).exp();
        if expl > 50.0 {
            expl = 50.0;
        }
        if tt <= 0.0 {
            expl = 50.0;
        }

        let densa = dlb * (tlb / tt).powf(1.0 + alpha + gamma) * expl;
        densu_temp = densa;
        if alt >= za {
            return densu_temp;
        }

        // ZA より下の密度: スプライン温度を積分する
        let glb = self.gsurf / (1.0 + z1 / self.re).powf(2.0);
        let gamm = xm * glb * zgdif / RGAS;
        let yi = splini(&xs[..mn], &ys[..mn], &y2out[..mn], x);
        let mut expl = gamm * yi;
        if expl > 50.0 {
            expl = 50.0;
        }
        if *tz <= 0.0 {
            expl = 50.0;
        }

        densu_temp * (t1 / *tz).powf(1.0 + alpha) * (-expl).exp()
    }

    /// 原典 `densm` — 成層圏・中間圏・対流圏の温度と密度。
    ///
    /// `xm == 0.0` のときは密度ではなく**温度**を返す（原典の仕様。呼び出し側が温度取得に使う）。
    pub(super) fn densm(
        &mut self,
        alt: f64,
        d0: f64,
        xm: f64,
        tz: &mut f64,
        zn3: &[f64; 5],
        zn2: &[f64; 4],
    ) -> f64 {
        let mut xs = [0.0_f64; 5];
        let mut ys = [0.0_f64; 5];
        let mut y2out = [0.0_f64; 5];
        let mut densm_tmp = d0;

        if alt > zn2[0] {
            return if xm == 0.0 { *tz } else { d0 };
        }

        // 成層圏 / 中間圏の温度
        let mn2 = zn2.len();
        let z = if alt > zn2[mn2 - 1] {
            alt
        } else {
            zn2[mn2 - 1]
        };
        let mn = mn2;
        let z1 = zn2[0];
        let z2 = zn2[mn - 1];
        let t1 = self.meso_tn2[0];
        let t2 = self.meso_tn2[mn - 1];
        let zg = self.zeta(z, z1);
        let zgdif = self.zeta(z2, z1);

        for k in 0..mn {
            xs[k] = self.zeta(zn2[k], z1) / zgdif;
            ys[k] = 1.0 / self.meso_tn2[k];
        }
        let yd1 = -self.meso_tgn2[0] / (t1 * t1) * zgdif;
        let yd2 =
            -self.meso_tgn2[1] / (t2 * t2) * zgdif * ((self.re + z2) / (self.re + z1)).powf(2.0);

        spline(&xs[..mn], &ys[..mn], yd1, yd2, &mut y2out[..mn]);
        let x = zg / zgdif;
        let y = splint(&xs[..mn], &ys[..mn], &y2out[..mn], x);
        *tz = 1.0 / y;

        if xm != 0.0 {
            let glb = self.gsurf / (1.0 + z1 / self.re).powf(2.0);
            let gamm = xm * glb * zgdif / RGAS;
            let yi = splini(&xs[..mn], &ys[..mn], &y2out[..mn], x);
            let mut expl = gamm * yi;
            if expl > 50.0 {
                expl = 50.0;
            }
            densm_tmp = densm_tmp * (t1 / *tz) * (-expl).exp();
        }

        if alt > zn3[0] {
            return if xm == 0.0 { *tz } else { densm_tmp };
        }

        // 対流圏 / 下部成層圏の温度
        let mn3 = zn3.len();
        let z = alt;
        let mn = mn3;
        let z1 = zn3[0];
        let z2 = zn3[mn - 1];
        let t1 = self.meso_tn3[0];
        let t2 = self.meso_tn3[mn - 1];
        let zg = self.zeta(z, z1);
        let zgdif = self.zeta(z2, z1);

        for k in 0..mn {
            xs[k] = self.zeta(zn3[k], z1) / zgdif;
            ys[k] = 1.0 / self.meso_tn3[k];
        }
        let yd1 = -self.meso_tgn3[0] / (t1 * t1) * zgdif;
        let yd2 =
            -self.meso_tgn3[1] / (t2 * t2) * zgdif * ((self.re + z2) / (self.re + z1)).powf(2.0);

        spline(&xs[..mn], &ys[..mn], yd1, yd2, &mut y2out[..mn]);
        let x = zg / zgdif;
        let y = splint(&xs[..mn], &ys[..mn], &y2out[..mn], x);
        *tz = 1.0 / y;

        if xm != 0.0 {
            let glb = self.gsurf / (1.0 + z1 / self.re).powf(2.0);
            let gamm = xm * glb * zgdif / RGAS;
            let yi = splini(&xs[..mn], &ys[..mn], &y2out[..mn], x);
            let mut expl = gamm * yi;
            if expl > 50.0 {
                expl = 50.0;
            }
            densm_tmp = densm_tmp * (t1 / *tz) * (-expl).exp();
        }

        if xm == 0.0 { *tz } else { densm_tmp }
    }

    /// 原典 `scalh` — スケールハイト `[km]`。
    pub(super) fn scalh(&self, alt: f64, xm: f64, temp: f64) -> f64 {
        let g = self.gsurf / (1.0 + alt / self.re).powf(2.0);
        RGAS * temp / (g * xm)
    }
}
