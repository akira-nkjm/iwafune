//! 熱圏部分（原典 C の `gts7`）。72.5 km より上を担当する。
//!
//! 各成分（N₂ / He / O / O₂ / Ar / H / N / 異常酸素）について
//!
//! 1. 下端境界 `Zlb` での密度に球面調和の変動 `exp(G(L))` を掛け、
//! 2. [`State::densu`] で高度まで拡散平衡プロファイルを積み、
//! 3. 乱流圏界面（turbopause）より下では完全混合密度と [`profile::dnet`] で接続し、
//! 4. 地表混合比・化学反応の補正（[`profile::ccor`] / [`profile::ccor2`]）を掛ける
//!
//! という同じ流れをたどる。原典 `nrlmsise-00.c` からの 1 対 1 移植。
//!
//! [`profile::dnet`]: super::profile::dnet
//! [`profile::ccor`]: super::profile::ccor
//! [`profile::ccor2`]: super::profile::ccor2

use super::coeffs::{PD, PDL, PDM, PMA, PS, PT, PTL, PTM};
use super::profile::{ccor, ccor2, dnet};
use super::{Input, RawOutput, State};

const DGTR: f64 = 1.74533E-2;
const DR: f64 = 1.72142E-2;

/// 各成分の熱拡散係数 α（原典 `alpha[9]`）。
const ALPHA: [f64; 9] = [-0.38, 0.0, 0.0, 0.0, 0.17, 0.0, -0.38, 0.0, 0.0];

/// 混合密度の補正を掛ける上限高度 `[km]`（原典 `altl[8]`）。
const ALTL: [f64; 8] = [200.0, 300.0, 160.0, 250.0, 240.0, 450.0, 320.0, 450.0];

impl State {
    /// 原典 `gts7` — 熱圏の温度と各成分数密度（CGS: `cm⁻³`, `g/cm³`）。
    ///
    /// 呼び出し後 `self.dm28`（N₂ の完全混合密度）が [`State::gtd7`] から読める。
    #[allow(clippy::similar_names, clippy::too_many_lines)]
    pub(super) fn gts7(&mut self, input: &Input) -> RawOutput {
        let mut out = RawOutput::default();

        // Bates プロファイルとスプラインの結合高度は係数から来る
        let za = PDL[1][15];
        let zn1 = [za, 110.0, 100.0, 90.0, 72.5];
        let alt = input.altitude_km;
        let sw = self.sw;

        // Tinf の変動は ZA より下では効かない
        let tinf = if alt > zn1[0] {
            PTM[0] * PT[0] * (1.0 + sw[16] * self.globe7(&PT, input))
        } else {
            PTM[0] * PT[0]
        };
        out.t_inf = tinf;

        // 温度勾配の変動は zn1[4] より下では効かない
        let g0 = if alt > zn1[4] {
            PTM[3] * PS[0] * (1.0 + sw[19] * self.globe7(&PS, input))
        } else {
            PTM[3] * PS[0]
        };
        let tlb = PTM[1] * (1.0 + sw[17] * self.globe7(&PD[3], input)) * PD[3][0];
        let s = g0 / (tinf - tlb);

        // 下部熱圏の温度変動は 300 km より上の密度には効かない
        if alt < 300.0 {
            self.meso_tn1[1] = PTM[6] * PTL[0][0] / (1.0 - sw[18] * self.glob7s(&PTL[0], input));
            self.meso_tn1[2] = PTM[2] * PTL[1][0] / (1.0 - sw[18] * self.glob7s(&PTL[1], input));
            self.meso_tn1[3] = PTM[7] * PTL[2][0] / (1.0 - sw[18] * self.glob7s(&PTL[2], input));
            self.meso_tn1[4] =
                PTM[4] * PTL[3][0] / (1.0 - sw[18] * sw[20] * self.glob7s(&PTL[3], input));
            self.meso_tgn1[1] = PTM[8]
                * PMA[8][0]
                * (1.0 + sw[18] * sw[20] * self.glob7s(&PMA[8], input))
                * self.meso_tn1[4]
                * self.meso_tn1[4]
                / (PTM[4] * PTL[3][0]).powf(2.0);
        } else {
            self.meso_tn1[1] = PTM[6] * PTL[0][0];
            self.meso_tn1[2] = PTM[2] * PTL[1][0];
            self.meso_tn1[3] = PTM[7] * PTL[2][0];
            self.meso_tn1[4] = PTM[4] * PTL[3][0];
            self.meso_tgn1[1] = PTM[8] * PMA[8][0] * self.meso_tn1[4] * self.meso_tn1[4]
                / (PTM[4] * PTL[3][0]).powf(2.0);
        }

        // Zlb における N2 の変動係数
        let g28 = sw[21] * self.globe7(&PD[2], input);

        // 乱流圏界面高度の変動
        let zhf = PDL[1][24]
            * (1.0
                + sw[5]
                    * PDL[0][24]
                    * (DGTR * input.latitude_deg).sin()
                    * (DR * (f64::from(input.day_of_year) - PT[13])).cos());
        out.t_inf = tinf;
        let xmm = PDM[2][4];
        let z = alt;
        let zlb = PTM[5];
        let mut tz = 0.0;

        // ---- N2 ----
        let db28 = PDM[2][0] * g28.exp() * PD[2][0];
        out.d[2] = self.densu(z, db28, tinf, tlb, 28.0, ALPHA[2], &mut tz, zlb, s, &zn1);
        let zh28 = PDM[2][2] * zhf;
        let zhm28 = PDM[2][3] * PDL[1][5];
        let xmd = 28.0 - xmm;
        let b28 = self.densu(
            zh28,
            db28,
            tinf,
            tlb,
            xmd,
            ALPHA[2] - 1.0,
            &mut tz,
            zlb,
            s,
            &zn1,
        );
        if sw[15] != 0.0 && z <= ALTL[2] {
            self.dm28 = self.densu(z, b28, tinf, tlb, xmm, ALPHA[2], &mut tz, zlb, s, &zn1);
            out.d[2] = dnet(out.d[2], self.dm28, zhm28, xmm, 28.0);
        }

        // ---- He ----
        let g4 = sw[21] * self.globe7(&PD[0], input);
        let db04 = PDM[0][0] * g4.exp() * PD[0][0];
        out.d[0] = self.densu(z, db04, tinf, tlb, 4.0, ALPHA[0], &mut tz, zlb, s, &zn1);
        if sw[15] != 0.0 && z < ALTL[0] {
            let zh04 = PDM[0][2];
            let b04 = self.densu(
                zh04,
                db04,
                tinf,
                tlb,
                4.0 - xmm,
                ALPHA[0] - 1.0,
                &mut tz,
                zlb,
                s,
                &zn1,
            );
            let dm04 = self.densu(z, b04, tinf, tlb, xmm, 0.0, &mut tz, zlb, s, &zn1);
            out.d[0] = dnet(out.d[0], dm04, zhm28, xmm, 4.0);
            // 地表混合比への補正
            let rl = (b28 * PDM[0][1] / b04).ln();
            let zc04 = PDM[0][4] * PDL[1][0];
            let hc04 = PDM[0][5] * PDL[1][1];
            out.d[0] *= ccor(z, rl, hc04, zc04);
        }

        // ---- O ----
        let g16 = sw[21] * self.globe7(&PD[1], input);
        let db16 = PDM[1][0] * g16.exp() * PD[1][0];
        out.d[1] = self.densu(z, db16, tinf, tlb, 16.0, ALPHA[1], &mut tz, zlb, s, &zn1);
        if sw[15] != 0.0 && z <= ALTL[1] {
            let zh16 = PDM[1][2];
            let b16 = self.densu(
                zh16,
                db16,
                tinf,
                tlb,
                16.0 - xmm,
                ALPHA[1] - 1.0,
                &mut tz,
                zlb,
                s,
                &zn1,
            );
            let dm16 = self.densu(z, b16, tinf, tlb, xmm, 0.0, &mut tz, zlb, s, &zn1);
            out.d[1] = dnet(out.d[1], dm16, zhm28, xmm, 16.0);
            let rl = PDM[1][1]
                * PDL[1][16]
                * (1.0 + sw[1] * PDL[0][23] * (input.f107_81day_avg - 150.0));
            let hc16 = PDM[1][5] * PDL[1][3];
            let zc16 = PDM[1][4] * PDL[1][2];
            let hc216 = PDM[1][5] * PDL[1][4];
            out.d[1] *= ccor2(z, rl, hc16, zc16, hc216);
            // 化学反応の補正
            let hcc16 = PDM[1][7] * PDL[1][13];
            let zcc16 = PDM[1][6] * PDL[1][12];
            let rc16 = PDM[1][3] * PDL[1][14];
            out.d[1] *= ccor(z, rc16, hcc16, zcc16);
        }

        // ---- O2 ----
        let g32 = sw[21] * self.globe7(&PD[4], input);
        let db32 = PDM[3][0] * g32.exp() * PD[4][0];
        out.d[3] = self.densu(z, db32, tinf, tlb, 32.0, ALPHA[3], &mut tz, zlb, s, &zn1);
        if sw[15] != 0.0 {
            if z <= ALTL[3] {
                let zh32 = PDM[3][2];
                let b32 = self.densu(
                    zh32,
                    db32,
                    tinf,
                    tlb,
                    32.0 - xmm,
                    ALPHA[3] - 1.0,
                    &mut tz,
                    zlb,
                    s,
                    &zn1,
                );
                let dm32 = self.densu(z, b32, tinf, tlb, xmm, 0.0, &mut tz, zlb, s, &zn1);
                out.d[3] = dnet(out.d[3], dm32, zhm28, xmm, 32.0);
                let rl = (b28 * PDM[3][1] / b32).ln();
                let hc32 = PDM[3][5] * PDL[1][7];
                let zc32 = PDM[3][4] * PDL[1][6];
                out.d[3] *= ccor(z, rl, hc32, zc32);
            }
            // Zlb より上での拡散平衡からのずれの補正
            let hcc32 = PDM[3][7] * PDL[1][22];
            let hcc232 = PDM[3][7] * PDL[0][22];
            let zcc32 = PDM[3][6] * PDL[1][21];
            let rc32 = PDM[3][3]
                * PDL[1][23]
                * (1.0 + sw[1] * PDL[0][23] * (input.f107_81day_avg - 150.0));
            out.d[3] *= ccor2(z, rc32, hcc32, zcc32, hcc232);
        }

        // ---- Ar ----
        let g40 = sw[21] * self.globe7(&PD[5], input);
        let db40 = PDM[4][0] * g40.exp() * PD[5][0];
        out.d[4] = self.densu(z, db40, tinf, tlb, 40.0, ALPHA[4], &mut tz, zlb, s, &zn1);
        if sw[15] != 0.0 && z <= ALTL[4] {
            let zh40 = PDM[4][2];
            let b40 = self.densu(
                zh40,
                db40,
                tinf,
                tlb,
                40.0 - xmm,
                ALPHA[4] - 1.0,
                &mut tz,
                zlb,
                s,
                &zn1,
            );
            let dm40 = self.densu(z, b40, tinf, tlb, xmm, 0.0, &mut tz, zlb, s, &zn1);
            out.d[4] = dnet(out.d[4], dm40, zhm28, xmm, 40.0);
            let rl = (b28 * PDM[4][1] / b40).ln();
            let hc40 = PDM[4][5] * PDL[1][9];
            let zc40 = PDM[4][4] * PDL[1][8];
            out.d[4] *= ccor(z, rl, hc40, zc40);
        }

        // ---- H ----
        let g1 = sw[21] * self.globe7(&PD[6], input);
        let db01 = PDM[5][0] * g1.exp() * PD[6][0];
        out.d[6] = self.densu(z, db01, tinf, tlb, 1.0, ALPHA[6], &mut tz, zlb, s, &zn1);
        if sw[15] != 0.0 && z <= ALTL[6] {
            let zh01 = PDM[5][2];
            let b01 = self.densu(
                zh01,
                db01,
                tinf,
                tlb,
                1.0 - xmm,
                ALPHA[6] - 1.0,
                &mut tz,
                zlb,
                s,
                &zn1,
            );
            let dm01 = self.densu(z, b01, tinf, tlb, xmm, 0.0, &mut tz, zlb, s, &zn1);
            out.d[6] = dnet(out.d[6], dm01, zhm28, xmm, 1.0);
            let rl = (b28 * PDM[5][1] * (PDL[1][17] * PDL[1][17]).sqrt() / b01).ln();
            let hc01 = PDM[5][5] * PDL[1][11];
            let zc01 = PDM[5][4] * PDL[1][10];
            out.d[6] *= ccor(z, rl, hc01, zc01);
            let hcc01 = PDM[5][7] * PDL[1][19];
            let zcc01 = PDM[5][6] * PDL[1][18];
            let rc01 = PDM[5][3] * PDL[1][20];
            out.d[6] *= ccor(z, rc01, hcc01, zcc01);
        }

        // ---- N ----
        let g14 = sw[21] * self.globe7(&PD[7], input);
        let db14 = PDM[6][0] * g14.exp() * PD[7][0];
        out.d[7] = self.densu(z, db14, tinf, tlb, 14.0, ALPHA[7], &mut tz, zlb, s, &zn1);
        if sw[15] != 0.0 && z <= ALTL[7] {
            let zh14 = PDM[6][2];
            let b14 = self.densu(
                zh14,
                db14,
                tinf,
                tlb,
                14.0 - xmm,
                ALPHA[7] - 1.0,
                &mut tz,
                zlb,
                s,
                &zn1,
            );
            let dm14 = self.densu(z, b14, tinf, tlb, xmm, 0.0, &mut tz, zlb, s, &zn1);
            out.d[7] = dnet(out.d[7], dm14, zhm28, xmm, 14.0);
            let rl = (b28 * PDM[6][1] * (PDL[0][2] * PDL[0][2]).sqrt() / b14).ln();
            let hc14 = PDM[6][5] * PDL[0][1];
            let zc14 = PDM[6][4] * PDL[0][0];
            out.d[7] *= ccor(z, rl, hc14, zc14);
            let hcc14 = PDM[6][7] * PDL[0][4];
            let zcc14 = PDM[6][6] * PDL[0][3];
            let rc14 = PDM[6][3] * PDL[0][5];
            out.d[7] *= ccor(z, rc14, hcc14, zcc14);
        }

        // ---- 異常酸素 ----
        let g16h = sw[21] * self.globe7(&PD[8], input);
        let db16h = PDM[7][0] * g16h.exp() * PD[8][0];
        let tho = PDM[7][9] * PDL[0][6];
        let dd = self.densu(z, db16h, tho, tho, 16.0, ALPHA[8], &mut tz, zlb, s, &zn1);
        let zsht = PDM[7][5];
        let zmho = PDM[7][4];
        let zsho = self.scalh(zmho, 16.0, tho);
        out.d[8] = dd * (-zsht / zsho * ((-(z - zmho) / zsht).exp() - 1.0)).exp();

        // 全質量密度（異常酸素は含めない。原典 gtd7 の定義）
        out.d[5] = 1.66E-24
            * (4.0 * out.d[0]
                + 16.0 * out.d[1]
                + 28.0 * out.d[2]
                + 32.0 * out.d[3]
                + 40.0 * out.d[4]
                + out.d[6]
                + 14.0 * out.d[7]);

        // 温度（この呼び出しが最終的な t_alt を決める）
        let z = (alt * alt).sqrt();
        let _ = self.densu(z, 1.0, tinf, tlb, 0.0, 0.0, &mut tz, zlb, s, &zn1);
        out.t_alt = tz;

        out
    }
}
