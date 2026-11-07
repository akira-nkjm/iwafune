//! 球面調和展開による変動項（原典 C の `globe7` / `glob7s`）。
//!
//! 緯度・季節・地方時・経度・UT・地磁気活動・F10.7 の各変動を、Legendre 多項式と
//! 三角関数の積の和として表す。原典 `nrlmsise-00.c` からの 1 対 1 移植で、係数の添字は
//! 原典のまま（[`super::coeffs`] 参照）。**添字をずらすと原典との突き合わせができなくなる**。

use super::{Input, State};

const DGTR: f64 = 1.74533E-2;
const DR: f64 = 1.72142E-2;
const HR: f64 = 0.2618;
const SR: f64 = 7.2722E-5;

/// 原典 Eq. A24d — 3 時間 ap から地磁気活動の寄与を作る。
fn g0(a: f64, p: &[f64]) -> f64 {
    let p24 = (p[24] * p[24]).sqrt();
    a - 4.0 + (p[25] - 1.0) * (a - 4.0 + ((-p24 * (a - 4.0)).exp() - 1.0) / p24)
}

/// 原典 Eq. A24c。
fn sumex(ex: f64) -> f64 {
    1.0 + (1.0 - ex.powf(19.0)) / (1.0 - ex) * ex.powf(0.5)
}

/// 原典 Eq. A24a — 3 時間 ap 配列の重み付き和。
fn sg0(ex: f64, p: &[f64], ap: &[f64; 7]) -> f64 {
    (g0(ap[1], p)
        + (g0(ap[2], p) * ex
            + g0(ap[3], p) * ex * ex
            + g0(ap[4], p) * ex.powf(3.0)
            + (g0(ap[5], p) * ex.powf(4.0) + g0(ap[6], p) * ex.powf(12.0)) * (1.0 - ex.powf(8.0))
                / (1.0 - ex)))
        / sumex(ex)
}

impl State {
    /// 原典 `globe7` — 上部熱圏パラメータ `G(L)`。
    ///
    /// Legendre 多項式・地方時の三角関数・`dfa`・`apdf`/`apt` を [`State`] に残す。
    /// `glob7s` はそれらを**再計算せずに読む**ので、原典と同じ順序で呼ぶ必要がある。
    pub(super) fn globe7(&mut self, p: &[f64], input: &Input) -> f64 {
        let mut t = [0.0_f64; 15];
        let tloc = input.local_solar_time_h;

        // Legendre 多項式
        let c = (input.latitude_deg * DGTR).sin();
        let s = (input.latitude_deg * DGTR).cos();
        let c2 = c * c;
        let c4 = c2 * c2;
        let s2 = s * s;

        let plg = &mut self.plg;
        plg[0][1] = c;
        plg[0][2] = 0.5 * (3.0 * c2 - 1.0);
        plg[0][3] = 0.5 * (5.0 * c * c2 - 3.0 * c);
        plg[0][4] = (35.0 * c4 - 30.0 * c2 + 3.0) / 8.0;
        plg[0][5] = (63.0 * c2 * c2 * c - 70.0 * c2 * c + 15.0 * c) / 8.0;
        plg[0][6] = (11.0 * c * plg[0][5] - 5.0 * plg[0][4]) / 6.0;
        plg[1][1] = s;
        plg[1][2] = 3.0 * c * s;
        plg[1][3] = 1.5 * (5.0 * c2 - 1.0) * s;
        plg[1][4] = 2.5 * (7.0 * c2 * c - 3.0 * c) * s;
        plg[1][5] = 1.875 * (21.0 * c4 - 14.0 * c2 + 1.0) * s;
        plg[1][6] = (11.0 * c * plg[1][5] - 6.0 * plg[1][4]) / 5.0;
        plg[2][2] = 3.0 * s2;
        plg[2][3] = 15.0 * s2 * c;
        plg[2][4] = 7.5 * (7.0 * c2 - 1.0) * s2;
        plg[2][5] = 3.0 * c * plg[2][4] - 2.0 * plg[2][3];
        plg[2][6] = (11.0 * c * plg[2][5] - 7.0 * plg[2][4]) / 4.0;
        plg[2][7] = (13.0 * c * plg[2][6] - 8.0 * plg[2][5]) / 5.0;
        plg[3][3] = 15.0 * s2 * s;
        plg[3][4] = 105.0 * s2 * s * c;
        plg[3][5] = (9.0 * c * plg[3][4] - 7.0 * plg[3][3]) / 2.0;
        plg[3][6] = (11.0 * c * plg[3][5] - 8.0 * plg[3][4]) / 3.0;

        // 地方時の三角関数（原典は sw[7]/sw[8]/sw[14] が全て 0 のときだけ省略する）
        self.stloc = (HR * tloc).sin();
        self.ctloc = (HR * tloc).cos();
        self.s2tloc = (2.0 * HR * tloc).sin();
        self.c2tloc = (2.0 * HR * tloc).cos();
        self.s3tloc = (3.0 * HR * tloc).sin();
        self.c3tloc = (3.0 * HR * tloc).cos();

        let doy = f64::from(input.day_of_year);
        let cd32 = (DR * (doy - p[31])).cos();
        let cd18 = (2.0 * DR * (doy - p[17])).cos();
        let cd14 = (DR * (doy - p[13])).cos();
        let cd39 = (2.0 * DR * (doy - p[38])).cos();

        let plg = self.plg;
        let swc = self.swc;
        let sw = self.sw;

        // F10.7 の効果
        let df = input.f107_daily - input.f107_81day_avg;
        self.dfa = input.f107_81day_avg - 150.0;
        let dfa = self.dfa;
        t[0] = p[19] * df * (1.0 + p[59] * dfa)
            + p[20] * df * df
            + p[21] * dfa
            + p[29] * dfa.powf(2.0);
        let f1 = 1.0 + (p[47] * dfa + p[19] * df + p[20] * df * df) * swc[1];
        let f2 = 1.0 + (p[49] * dfa + p[19] * df + p[20] * df * df) * swc[1];

        // 時間非依存
        t[1] = (p[1] * plg[0][2] + p[2] * plg[0][4] + p[22] * plg[0][6])
            + (p[14] * plg[0][2]) * dfa * swc[1]
            + p[26] * plg[0][1];

        // 対称年周期 / 対称半年周期
        t[2] = p[18] * cd32;
        t[3] = (p[15] + p[16] * plg[0][2]) * cd18;

        // 非対称年周期 / 非対称半年周期
        t[4] = f1 * (p[9] * plg[0][1] + p[10] * plg[0][3]) * cd14;
        t[5] = p[37] * plg[0][1] * cd39;

        // 日周期
        if sw[7] != 0.0 {
            let t71 = (p[11] * plg[1][2]) * cd14 * swc[5];
            let t72 = (p[12] * plg[1][2]) * cd14 * swc[5];
            t[6] = f2
                * ((p[3] * plg[1][1] + p[4] * plg[1][3] + p[27] * plg[1][5] + t71) * self.ctloc
                    + (p[6] * plg[1][1] + p[7] * plg[1][3] + p[28] * plg[1][5] + t72) * self.stloc);
        }

        // 半日周期
        if sw[8] != 0.0 {
            let t81 = (p[23] * plg[2][3] + p[35] * plg[2][5]) * cd14 * swc[5];
            let t82 = (p[33] * plg[2][3] + p[36] * plg[2][5]) * cd14 * swc[5];
            t[7] = f2
                * ((p[5] * plg[2][2] + p[41] * plg[2][4] + t81) * self.c2tloc
                    + (p[8] * plg[2][2] + p[42] * plg[2][4] + t82) * self.s2tloc);
        }

        // 三分日周期
        if sw[14] != 0.0 {
            t[13] = f2
                * ((p[39] * plg[3][3] + (p[93] * plg[3][4] + p[46] * plg[3][6]) * cd14 * swc[5])
                    * self.s3tloc
                    + (p[40] * plg[3][3]
                        + (p[94] * plg[3][4] + p[48] * plg[3][6]) * cd14 * swc[5])
                        * self.c3tloc);
        }

        // 地磁気活動
        if sw[9] == -1.0 {
            let ap = &input.ap_array;
            if p[51] != 0.0 {
                let mut exp1 = (-10800.0 * (p[51] * p[51]).sqrt()
                    / (1.0 + p[138] * (45.0 - (input.latitude_deg * input.latitude_deg).sqrt())))
                .exp();
                if exp1 > 0.99999 {
                    exp1 = 0.99999;
                }
                // 原典はここで p[24] を 1e-4 未満なら書き換えるが、同梱の係数表では
                // どの p でも p[24] >= 0.05 なので発火しない（[`super::coeffs`] を変えたら
                // ここを見直すこと）。
                debug_assert!(
                    p[24] >= 1.0E-4,
                    "p[24] のクランプが必要な係数表になっている"
                );
                self.apt = sg0(exp1, p, ap);
                if sw[9] != 0.0 {
                    t[8] = self.apt
                        * (p[50]
                            + p[96] * plg[0][2]
                            + p[54] * plg[0][4]
                            + (p[125] * plg[0][1] + p[126] * plg[0][3] + p[127] * plg[0][5])
                                * cd14
                                * swc[5]
                            + (p[128] * plg[1][1] + p[129] * plg[1][3] + p[130] * plg[1][5])
                                * swc[7]
                                * (HR * (tloc - p[131])).cos());
                }
            }
        } else {
            let apd = input.ap_daily - 4.0;
            let p44 = if p[43] < 0.0 { 1.0E-5 } else { p[43] };
            let p45 = p[44];
            self.apdf = apd + (p45 - 1.0) * (apd + ((-p44 * apd).exp() - 1.0) / p44);
            if sw[9] != 0.0 {
                t[8] = self.apdf
                    * (p[32]
                        + p[45] * plg[0][2]
                        + p[34] * plg[0][4]
                        + (p[100] * plg[0][1] + p[101] * plg[0][3] + p[102] * plg[0][5])
                            * cd14
                            * swc[5]
                        + (p[121] * plg[1][1] + p[122] * plg[1][3] + p[123] * plg[1][5])
                            * swc[7]
                            * (HR * (tloc - p[124])).cos());
            }
        }

        if sw[10] != 0.0 && input.longitude_deg > -1000.0 {
            // 経度依存
            if sw[11] != 0.0 {
                t[10] = (1.0 + p[80] * dfa * swc[1])
                    * ((p[64] * plg[1][2]
                        + p[65] * plg[1][4]
                        + p[66] * plg[1][6]
                        + p[103] * plg[1][1]
                        + p[104] * plg[1][3]
                        + p[105] * plg[1][5]
                        + swc[5]
                            * (p[109] * plg[1][1] + p[110] * plg[1][3] + p[111] * plg[1][5])
                            * cd14)
                        * (DGTR * input.longitude_deg).cos()
                        + (p[90] * plg[1][2]
                            + p[91] * plg[1][4]
                            + p[92] * plg[1][6]
                            + p[106] * plg[1][1]
                            + p[107] * plg[1][3]
                            + p[108] * plg[1][5]
                            + swc[5]
                                * (p[112] * plg[1][1] + p[113] * plg[1][3] + p[114] * plg[1][5])
                                * cd14)
                            * (DGTR * input.longitude_deg).sin());
            }

            // UT と UT×経度の混合
            if sw[12] != 0.0 {
                t[11] = (1.0 + p[95] * plg[0][1])
                    * (1.0 + p[81] * dfa * swc[1])
                    * (1.0 + p[119] * plg[0][1] * swc[5] * cd14)
                    * ((p[68] * plg[0][1] + p[69] * plg[0][3] + p[70] * plg[0][5])
                        * (SR * (input.sec_ut - p[71])).cos());
                t[11] += swc[11]
                    * (p[76] * plg[2][3] + p[77] * plg[2][5] + p[78] * plg[2][7])
                    * (SR * (input.sec_ut - p[79]) + 2.0 * DGTR * input.longitude_deg).cos()
                    * (1.0 + p[137] * dfa * swc[1]);
            }

            // UT・経度・地磁気活動の混合
            if sw[13] != 0.0 {
                if sw[9] == -1.0 {
                    if p[51] != 0.0 {
                        t[12] = self.apt
                            * swc[11]
                            * (1.0 + p[132] * plg[0][1])
                            * ((p[52] * plg[1][2] + p[98] * plg[1][4] + p[67] * plg[1][6])
                                * (DGTR * (input.longitude_deg - p[97])).cos())
                            + self.apt
                                * swc[11]
                                * swc[5]
                                * (p[133] * plg[1][1] + p[134] * plg[1][3] + p[135] * plg[1][5])
                                * cd14
                                * (DGTR * (input.longitude_deg - p[136])).cos()
                            + self.apt
                                * swc[12]
                                * (p[55] * plg[0][1] + p[56] * plg[0][3] + p[57] * plg[0][5])
                                * (SR * (input.sec_ut - p[58])).cos();
                    }
                } else {
                    t[12] = self.apdf
                        * swc[11]
                        * (1.0 + p[120] * plg[0][1])
                        * ((p[60] * plg[1][2] + p[61] * plg[1][4] + p[62] * plg[1][6])
                            * (DGTR * (input.longitude_deg - p[63])).cos())
                        + self.apdf
                            * swc[11]
                            * swc[5]
                            * (p[115] * plg[1][1] + p[116] * plg[1][3] + p[117] * plg[1][5])
                            * cd14
                            * (DGTR * (input.longitude_deg - p[118])).cos()
                        + self.apdf
                            * swc[12]
                            * (p[83] * plg[0][1] + p[84] * plg[0][3] + p[85] * plg[0][5])
                            * (SR * (input.sec_ut - p[75])).cos();
                }
            }
        }

        // 未使用の係数: 82, 89, 99, 139-149
        let mut tinf = p[30];
        for i in 0..14 {
            tinf += sw[i + 1].abs() * t[i];
        }
        tinf
    }

    /// 原典 `glob7s` — 下層大気版の `globe7`。
    ///
    /// Legendre 多項式・地方時の三角関数・`dfa`・`apdf`/`apt` は **[`Self::globe7`] が
    /// 直前に計算したものを読む**（原典が静的変数で共有しているのと同じ）。
    pub(super) fn glob7s(&self, p: &[f64], input: &Input) -> f64 {
        // 原典は p[99] が 0 なら 2.0 を書き込み、2.0 以外ならエラーを返す
        // （パラメータセットの取り違え検出）。同梱の表はすべて 2.0 なので読むだけでよい。
        debug_assert!(
            (p[99] - 2.0).abs() < f64::EPSILON,
            "glob7s に渡す係数表のパラメータセットが 2 でない"
        );

        let mut t = [0.0_f64; 14];
        let doy = f64::from(input.day_of_year);
        let cd32 = (DR * (doy - p[31])).cos();
        let cd18 = (2.0 * DR * (doy - p[17])).cos();
        let cd14 = (DR * (doy - p[13])).cos();
        let cd39 = (2.0 * DR * (doy - p[38])).cos();

        let plg = self.plg;
        let sw = self.sw;
        let swc = self.swc;

        // F10.7
        t[0] = p[21] * self.dfa;

        // 時間非依存
        t[1] = p[1] * plg[0][2]
            + p[2] * plg[0][4]
            + p[22] * plg[0][6]
            + p[26] * plg[0][1]
            + p[14] * plg[0][3]
            + p[59] * plg[0][5];

        // 対称年周期 / 対称半年周期
        t[2] = (p[18] + p[47] * plg[0][2] + p[29] * plg[0][4]) * cd32;
        t[3] = (p[15] + p[16] * plg[0][2] + p[30] * plg[0][4]) * cd18;

        // 非対称年周期 / 非対称半年周期
        t[4] = (p[9] * plg[0][1] + p[10] * plg[0][3] + p[20] * plg[0][5]) * cd14;
        t[5] = (p[37] * plg[0][1]) * cd39;

        // 日周期
        if sw[7] != 0.0 {
            let t71 = p[11] * plg[1][2] * cd14 * swc[5];
            let t72 = p[12] * plg[1][2] * cd14 * swc[5];
            t[6] = (p[3] * plg[1][1] + p[4] * plg[1][3] + t71) * self.ctloc
                + (p[6] * plg[1][1] + p[7] * plg[1][3] + t72) * self.stloc;
        }

        // 半日周期
        if sw[8] != 0.0 {
            let t81 = (p[23] * plg[2][3] + p[35] * plg[2][5]) * cd14 * swc[5];
            let t82 = (p[33] * plg[2][3] + p[36] * plg[2][5]) * cd14 * swc[5];
            t[7] = (p[5] * plg[2][2] + p[41] * plg[2][4] + t81) * self.c2tloc
                + (p[8] * plg[2][2] + p[42] * plg[2][4] + t82) * self.s2tloc;
        }

        // 三分日周期
        if sw[14] != 0.0 {
            t[13] = p[39] * plg[3][3] * self.s3tloc + p[40] * plg[3][3] * self.c3tloc;
        }

        // 地磁気活動
        if sw[9] != 0.0 {
            if sw[9] == 1.0 {
                t[8] = self.apdf * (p[32] + p[45] * plg[0][2] * swc[2]);
            }
            if sw[9] == -1.0 {
                t[8] = p[50] * self.apt + p[96] * plg[0][2] * self.apt * swc[2];
            }
        }

        // 経度依存
        if !(sw[10] == 0.0 || sw[11] == 0.0 || input.longitude_deg <= -1000.0) {
            t[10] = (1.0
                + plg[0][1]
                    * (p[80] * swc[5] * (DR * (doy - p[81])).cos()
                        + p[85] * swc[6] * (2.0 * DR * (doy - p[86])).cos())
                + p[83] * swc[3] * (DR * (doy - p[84])).cos()
                + p[87] * swc[4] * (2.0 * DR * (doy - p[88])).cos())
                * ((p[64] * plg[1][2]
                    + p[65] * plg[1][4]
                    + p[66] * plg[1][6]
                    + p[74] * plg[1][1]
                    + p[75] * plg[1][3]
                    + p[76] * plg[1][5])
                    * (DGTR * input.longitude_deg).cos()
                    + (p[90] * plg[1][2]
                        + p[91] * plg[1][4]
                        + p[92] * plg[1][6]
                        + p[77] * plg[1][1]
                        + p[78] * plg[1][3]
                        + p[79] * plg[1][5])
                        * (DGTR * input.longitude_deg).sin());
        }

        let mut tt = 0.0;
        for i in 0..14 {
            tt += sw[i + 1].abs() * t[i];
        }
        tt
    }
}
