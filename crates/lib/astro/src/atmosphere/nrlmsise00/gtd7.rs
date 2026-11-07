//! 原典 C の `gtd7` と状態の初期化（`tselec`）。72.5 km より上は熱圏（`gts7`）、
//! 下は混合大気（`densm`）で解く。式と評価順序は原典のまま。

use super::coeffs::{PAVGM, PDM, PMA};
use super::profile;
use super::{GeomagneticActivity, Input, RawOutput, State};

impl State {
    /// 標準のスイッチ設定で状態を作る（原典 `tselec` 相当）。
    ///
    /// 原典の `switches[]` は 24 個あり、個々の変動項を切るためのものだが、実運用で要るのは
    /// **標準モデル（全て 1）** と、地磁気を日平均 Ap で見るか 3 時間 Ap 履歴で見るかの
    /// 選択（スイッチ 9）だけである。スイッチ 0 は出力単位（0 = CGS）で、内部は原典と同じ
    /// CGS で計算して境界で SI に直すため 0 に固定する。
    pub(super) fn new(geomagnetic: GeomagneticActivity) -> Self {
        let sw9 = match geomagnetic {
            GeomagneticActivity::DailyAp(_) => 1.0,
            GeomagneticActivity::ThreeHourlyAp(_) => -1.0,
        };
        let mut sw = [1.0_f64; 24];
        let mut swc = [1.0_f64; 24];
        sw[0] = 0.0;
        swc[0] = 0.0;
        sw[9] = sw9;
        swc[9] = sw9;
        Self {
            sw,
            swc,
            ..Self::default()
        }
    }

    /// 原典 `gtd7` — 全高度の中性大気。72.5 km より上は [`Self::gts7`]、
    /// それより下は混合大気として [`Self::densm`] で解く。
    pub(super) fn gtd7(&mut self, input: &Input) -> RawOutput {
        let zn3 = [32.5, 20.0, 15.0, 10.0, 0.0];
        let zn2 = [72.5, 55.0, 45.0, 32.5];
        let zmix = 62.5;

        // 重力の緯度変化
        let (gsurf, re) = profile::glatf(input.latitude_deg);
        self.gsurf = gsurf;
        self.re = re;

        let xmm = PDM[2][4];

        // 熱圏/中間圏は zn2[0] 以上でしか解かないので、下では境界高度で評価する
        let altt = if input.altitude_km > zn2[0] {
            input.altitude_km
        } else {
            zn2[0]
        };
        let upper_input = Input {
            altitude_km: altt,
            ..*input
        };
        let soutput = self.gts7(&upper_input);
        let dm28m = self.dm28; // スイッチ 0 = 0（CGS）なので倍率は掛からない

        let mut out = RawOutput {
            t_inf: soutput.t_inf,
            t_alt: soutput.t_alt,
            ..RawOutput::default()
        };
        if input.altitude_km >= zn2[0] {
            out.d = soutput.d;
            return out;
        }

        // 下部中間圏／上部成層圏（zn3[0]〜zn2[0]）: 逆温度を球面調和の線形関数として与える
        self.meso_tgn2[0] = self.meso_tgn1[1];
        self.meso_tn2[0] = self.meso_tn1[4];
        self.meso_tn2[1] = PMA[0][0] * PAVGM[0] / (1.0 - self.sw[20] * self.glob7s(&PMA[0], input));
        self.meso_tn2[2] = PMA[1][0] * PAVGM[1] / (1.0 - self.sw[20] * self.glob7s(&PMA[1], input));
        self.meso_tn2[3] =
            PMA[2][0] * PAVGM[2] / (1.0 - self.sw[20] * self.sw[22] * self.glob7s(&PMA[2], input));
        self.meso_tgn2[1] = PAVGM[8]
            * PMA[9][0]
            * (1.0 + self.sw[20] * self.sw[22] * self.glob7s(&PMA[9], input))
            * self.meso_tn2[3]
            * self.meso_tn2[3]
            / (PMA[2][0] * PAVGM[2]).powf(2.0);
        self.meso_tn3[0] = self.meso_tn2[3];

        if input.altitude_km <= zn3[0] {
            // 下部成層圏と対流圏（zn3[0] 以下）
            //
            // 原典の 2019-07-09 の修正で `<` から `<=` になった箇所。高度ちょうど 32.5 km で
            // このブロックを飛ばすと meso_tn3[1..] が未初期化のまま densm に入り NaN が出る。
            // 適応ステップ積分器は NaN を踏むと復帰できないので、この修正は必須。
            self.meso_tgn3[0] = self.meso_tgn2[1];
            self.meso_tn3[1] =
                PMA[3][0] * PAVGM[3] / (1.0 - self.sw[22] * self.glob7s(&PMA[3], input));
            self.meso_tn3[2] =
                PMA[4][0] * PAVGM[4] / (1.0 - self.sw[22] * self.glob7s(&PMA[4], input));
            self.meso_tn3[3] =
                PMA[5][0] * PAVGM[5] / (1.0 - self.sw[22] * self.glob7s(&PMA[5], input));
            self.meso_tn3[4] =
                PMA[6][0] * PAVGM[6] / (1.0 - self.sw[22] * self.glob7s(&PMA[6], input));
            self.meso_tgn3[1] = PMA[7][0]
                * PAVGM[7]
                * (1.0 + self.sw[22] * self.glob7s(&PMA[7], input))
                * self.meso_tn3[4]
                * self.meso_tn3[4]
                / (PMA[6][0] * PAVGM[6]).powf(2.0);
        }

        // zn2[0] より下は完全混合へ線形に遷移させる
        let dmc = if input.altitude_km > zmix {
            1.0 - (zn2[0] - input.altitude_km) / (zn2[0] - zmix)
        } else {
            0.0
        };
        let dz28 = soutput.d[2];
        let alt = input.altitude_km;
        let mut tz = 0.0;

        // N2
        let dmr = soutput.d[2] / dm28m - 1.0;
        out.d[2] = self.densm(alt, dm28m, xmm, &mut tz, &zn3, &zn2);
        out.d[2] *= 1.0 + dmr * dmc;

        // He
        let dmr = soutput.d[0] / (dz28 * PDM[0][1]) - 1.0;
        out.d[0] = out.d[2] * PDM[0][1] * (1.0 + dmr * dmc);

        // O と異常酸素はこの高度では 0（原典どおり）
        out.d[1] = 0.0;
        out.d[8] = 0.0;

        // O2
        let dmr = soutput.d[3] / (dz28 * PDM[3][1]) - 1.0;
        out.d[3] = out.d[2] * PDM[3][1] * (1.0 + dmr * dmc);

        // Ar
        let dmr = soutput.d[4] / (dz28 * PDM[4][1]) - 1.0;
        out.d[4] = out.d[2] * PDM[4][1] * (1.0 + dmr * dmc);

        // H と N もこの高度では 0
        out.d[6] = 0.0;
        out.d[7] = 0.0;

        // 全質量密度
        out.d[5] = 1.66E-24
            * (4.0 * out.d[0]
                + 16.0 * out.d[1]
                + 28.0 * out.d[2]
                + 32.0 * out.d[3]
                + 40.0 * out.d[4]
                + out.d[6]
                + 14.0 * out.d[7]);

        // その高度の温度（densm は xm = 0 のとき温度を返す）
        let _ = self.densm(alt, 1.0, 0.0, &mut tz, &zn3, &zn2);
        out.t_alt = tz;

        out
    }
}
