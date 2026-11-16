//! 回線計算の部品（自由空間損失・EIRP・G/T・C/N0・Eb/N0）。斜距離は `sat_astro::ground::slant_range_m`（中心天体の半径）。
//! **回線計算はここだけ。** `comm.link_budget` と `comm.data_throughput` が使う。
//!
//! 入出力の dB の値は型（`sat_units::typed::Decibel`・`DecibelWatt` など）で持ち、名前に `_db`（`_dbw`・`_dbk`・`_dbhz`）を付ける。
//! すべて電力比の `10 log10`（`sat_units::ratio_to_db`）。式の部品（`fspl_db` など）は境界で取り出した f64 で計算する。
//! 線形の量（周波数・電力・雑音温度・データレート）は SI の `Qty` で持つ。

use crate::spec::AnalysisError;
use sat_astro::constants::{C_LIGHT, K_BOLTZMANN};
use sat_astro::ground::slant_range_m;
use sat_astro::math::bisect;
use sat_units::typed::{Decibel, DecibelPerKelvin, Frequency, Power, Qty, Temperature};
use sat_units::{db_to_ratio, ratio_to_db};
use serde::{Deserialize, Serialize};
use std::f64::consts::{FRAC_PI_2, PI};

/// 要求する回線余裕の既定値。**既定値はここだけ**（`comm.link_budget` と `comm.data_throughput` が使う）。
pub const DEFAULT_REQUIRED_MARGIN: Decibel = Decibel::new(3.0);

/// 要求する回線余裕（入力 `given`、省略時 [`DEFAULT_REQUIRED_MARGIN`]）。
pub fn required_margin(given: Option<Decibel>) -> Decibel {
    given.unwrap_or(DEFAULT_REQUIRED_MARGIN)
}

/// 回線が閉じる仰角の二分法の許容 [rad]。
const ELEVATION_TOLERANCE_RAD: f64 = 1e-12;

/// 回線が閉じる仰角の二分法の反復の上限。
const ELEVATION_MAX_ITERATIONS: usize = 100;

/// 送受信端と経路の RF パラメーター。データレートと所要 Eb/N0 は含まない（使う側の入力に置く）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RfLinkParams {
    /// 搬送波の周波数
    pub frequency: Qty<Frequency>,
    /// 送信電力（アンテナ給電点の手前）
    pub tx_power: Qty<Power>,
    /// 送信アンテナ利得 [dBi]
    pub tx_gain_db: Decibel,
    /// 送信側の給電損失 [dB]（正の値が損失）。省略時 0
    #[serde(default)]
    pub tx_loss_db: Option<Decibel>,
    /// 受信アンテナ利得 [dBi]
    pub rx_gain_db: Decibel,
    /// 受信側の給電損失 [dB]（LNA の手前。正の値が損失）。省略時 0
    #[serde(default)]
    pub rx_loss_db: Option<Decibel>,
    /// 受信系のシステム雑音温度
    pub system_noise_temperature: Qty<Temperature>,
    /// 受信系の G/T [dB/K]（与えれば `rx_gain_db`・`rx_loss_db`・`system_noise_temperature` から求めた G/T の代わりに使う。
    /// 両方あるときはこちらを優先し、結果の `g_over_t_given` で分かる）
    #[serde(default)]
    pub rx_g_over_t_dbk: Option<DecibelPerKelvin>,
    /// 大気・降雨の損失 [dB]。省略時 0
    #[serde(default)]
    pub atmospheric_loss_db: Option<Decibel>,
    /// 偏波損失 [dB]。省略時 0
    #[serde(default)]
    pub polarization_loss_db: Option<Decibel>,
    /// 指向損失 [dB]。省略時 0
    #[serde(default)]
    pub pointing_loss_db: Option<Decibel>,
    /// その他の損失（実装損失など）[dB]。省略時 0
    #[serde(default)]
    pub other_loss_db: Option<Decibel>,
}

/// 書かれていない損失は 0 dB（損失なし）とする。
fn loss_or_none(loss_db: Option<Decibel>) -> f64 {
    loss_db.map_or(0.0, Decibel::value)
}

/// ある斜距離での回線の計算結果（すべて dB）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinkEvaluation {
    /// EIRP [dBW]
    pub eirp_dbw: f64,
    /// 自由空間損失 [dB]
    pub fspl_db: f64,
    /// 自由空間損失以外の経路の損失（大気・偏波・指向・その他）[dB]
    pub extra_loss_db: f64,
    /// 受信系の G/T [dB/K]
    pub g_over_t_dbk: f64,
    /// G/T が与えられた値か（`false` なら利得・損失・雑音温度から求めた値）
    pub g_over_t_given: bool,
    /// C/N0 [dBHz]
    pub c_over_n0_dbhz: f64,
}

/// 自由空間損失 [dB] = 20 log10(4π d f / c)。電力の比 (4π d f / c)² の dB なので、振幅の比の dB を 2 倍する。
pub fn fspl_db(range_m: f64, frequency_hz: f64) -> f64 {
    2.0 * ratio_to_db(4.0 * PI * range_m * frequency_hz / C_LIGHT)
}

/// EIRP [dBW] = 10 log10(P) + G − L。
pub fn eirp_dbw(tx_power_w: f64, tx_gain_db: f64, tx_loss_db: f64) -> f64 {
    ratio_to_db(tx_power_w) + tx_gain_db - tx_loss_db
}

/// G/T [dB/K] = G − L − 10 log10(T)。
pub fn g_over_t_dbk(rx_gain_db: f64, rx_loss_db: f64, system_noise_temperature_k: f64) -> f64 {
    rx_gain_db - rx_loss_db - ratio_to_db(system_noise_temperature_k)
}

/// C/N0 [dBHz] = EIRP − 経路損失 + G/T − 10 log10(k)。
pub fn c_over_n0_dbhz(eirp_dbw: f64, path_loss_db: f64, g_over_t_dbk: f64) -> f64 {
    eirp_dbw - path_loss_db + g_over_t_dbk - ratio_to_db(K_BOLTZMANN)
}

/// Eb/N0 [dB] = C/N0 − 10 log10(R)。
pub fn eb_over_n0_db(c_over_n0_dbhz: f64, data_rate_bps: f64) -> f64 {
    c_over_n0_dbhz - ratio_to_db(data_rate_bps)
}

/// 所要 Eb/N0 と余裕を満たす最大のデータレート [bit/s]。
pub fn max_data_rate_bps(c_over_n0_dbhz: f64, required_eb_n0_db: f64, margin_db: f64) -> f64 {
    db_to_ratio(c_over_n0_dbhz - required_eb_n0_db - margin_db)
}

impl RfLinkParams {
    /// 値の範囲を検査する（周波数・送信電力・雑音温度は正、損失は有限）。
    pub fn validate(&self, what: &str) -> Result<(), AnalysisError> {
        let positive = [
            self.frequency.value(),
            self.tx_power.value(),
            self.system_noise_temperature.value(),
        ];
        let finite = [
            self.tx_gain_db.value(),
            self.tx_loss(),
            self.rx_gain_db.value(),
            self.rx_loss(),
            self.extra_loss_db(),
            self.rx_g_over_t_dbk.map_or(0.0, DecibelPerKelvin::value),
        ];
        if positive.iter().any(|v| !(v.is_finite() && *v > 0.0))
            || finite.iter().any(|v| !v.is_finite())
        {
            return Err(AnalysisError::InvalidInput(format!(
                "{what}: frequency, transmit power and noise temperature must be positive; gain and loss must be finite"
            )));
        }
        Ok(())
    }

    /// 送信側の給電損失 [dB]（省略時 0）。
    pub fn tx_loss(&self) -> f64 {
        loss_or_none(self.tx_loss_db)
    }

    /// 受信側の給電損失 [dB]（省略時 0）。
    pub fn rx_loss(&self) -> f64 {
        loss_or_none(self.rx_loss_db)
    }

    /// 自由空間損失以外の経路の損失の合計 [dB]（書かれていない損失は 0）。
    pub fn extra_loss_db(&self) -> f64 {
        [
            self.atmospheric_loss_db,
            self.polarization_loss_db,
            self.pointing_loss_db,
            self.other_loss_db,
        ]
        .into_iter()
        .map(loss_or_none)
        .sum()
    }

    /// 斜距離 `range_m` での回線を計算する。
    pub fn evaluate(&self, range_m: f64) -> LinkEvaluation {
        let eirp = eirp_dbw(
            self.tx_power.value(),
            self.tx_gain_db.value(),
            self.tx_loss(),
        );
        let fspl = fspl_db(range_m, self.frequency.value());
        let extra = self.extra_loss_db();
        let g_t = self.rx_g_over_t_dbk.map_or_else(
            || {
                g_over_t_dbk(
                    self.rx_gain_db.value(),
                    self.rx_loss(),
                    self.system_noise_temperature.value(),
                )
            },
            DecibelPerKelvin::value,
        );
        LinkEvaluation {
            eirp_dbw: eirp,
            fspl_db: fspl,
            extra_loss_db: extra,
            g_over_t_dbk: g_t,
            g_over_t_given: self.rx_g_over_t_dbk.is_some(),
            c_over_n0_dbhz: c_over_n0_dbhz(eirp, fspl + extra, g_t),
        }
    }

    /// Eb/N0 の余裕 [dB]（`eb_n0 − required − margin` ではなく `eb_n0 − required`）。
    pub fn eb_n0_margin_db(
        &self,
        range_m: f64,
        data_rate_bps: f64,
        required_eb_n0: Decibel,
    ) -> f64 {
        eb_over_n0_db(self.evaluate(range_m).c_over_n0_dbhz, data_rate_bps) - required_eb_n0.value()
    }

    /// 半径 `body_radius_m` の中心天体を回る高度 `altitude_m` の衛星で、Eb/N0 の余裕が `margin` 以上になる
    /// 最低の仰角 [rad]（斜距離は `sat_astro::ground::slant_range_m`）。天頂でも足りなければ `None`、水平線でも足りれば 0。
    pub fn min_elevation_for_margin(
        &self,
        body_radius_m: f64,
        altitude_m: f64,
        data_rate_bps: f64,
        required_eb_n0: Decibel,
        margin: Decibel,
    ) -> Option<f64> {
        let excess = |e: f64| {
            self.eb_n0_margin_db(
                slant_range_m(body_radius_m, altitude_m, e),
                data_rate_bps,
                required_eb_n0,
            ) - margin.value()
        };
        if excess(FRAC_PI_2) < 0.0 {
            return None;
        }
        if excess(0.0) >= 0.0 {
            return Some(0.0);
        }
        // 余裕は仰角に対して単調に増えるので二分法で求める。
        bisect(
            excess,
            0.0,
            FRAC_PI_2,
            ELEVATION_TOLERANCE_RAD,
            ELEVATION_MAX_ITERATIONS,
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use sat_astro::constants::{R_EARTH, R_MOON};

    #[test]
    fn fspl_437mhz_1000km() {
        // 20 log10(4π·1e6·437e6/c) = 145.25 dB
        let l = fspl_db(1.0e6, 437.0e6);
        assert!((l - 145.25).abs() < 0.02, "{l}");
        // 距離が 10 倍なら 20 dB 増える（振幅の比の 20 log10）
        assert!((fspl_db(1.0e7, 437.0e6) - l - 20.0).abs() < 1e-9);
    }

    #[test]
    fn min_elevation_uses_the_central_body_radius() {
        // 高度 550 km・所要 20 dB ＋ 余裕 3 dB。地平線の斜距離は地球（R = 6378 km）で約 2705 km、
        // Eb/N0 ≈ 0 − 153.9 − 15.0 + 228.6 − 39.8 = 19.9 dB で足りない。月（R = 1737 km）では約 1488 km、
        // FSPL ≈ 148.7 dB で Eb/N0 ≈ 25.1 dB となり、地平線から閉じる
        let rf = RfLinkParams {
            frequency: Qty::from_raw(437.0e6),
            tx_power: Qty::from_raw(1.0),
            tx_gain_db: Decibel::new(0.0),
            tx_loss_db: None,
            rx_gain_db: Decibel::new(12.0),
            rx_loss_db: None,
            system_noise_temperature: Qty::from_raw(500.0),
            rx_g_over_t_dbk: None,
            atmospheric_loss_db: None,
            polarization_loss_db: None,
            pointing_loss_db: None,
            other_loss_db: None,
        };
        let db = Decibel::new;
        let el =
            |radius: f64| rf.min_elevation_for_margin(radius, 550.0e3, 9600.0, db(20.0), db(3.0));
        let (earth, moon) = (el(R_EARTH).unwrap(), el(R_MOON).unwrap());
        assert!(earth > 0.0 && moon == 0.0, "{moon} vs {earth}");
        // 閉じる仰角での余裕は要求ちょうど（斜距離は ground::slant_range_m）
        let at = rf.eb_n0_margin_db(slant_range_m(R_EARTH, 550.0e3, earth), 9600.0, db(20.0));
        assert!((at - 3.0).abs() < 1e-6, "{at}");
    }

    /// 与えた G/T は利得・雑音温度から求めた G/T の代わりに使う（両方あるときは与えた方）。
    /// 手計算: 12 − 0 − 10 log10(500) = 12 − 26.99 = −14.99 dB/K。与えた −10 dB/K なら C/N0 は 4.99 dB 高い。
    #[test]
    fn given_g_over_t_replaces_the_derived_one() {
        let mut rf = RfLinkParams {
            frequency: Qty::from_raw(437.0e6),
            tx_power: Qty::from_raw(1.0),
            tx_gain_db: Decibel::new(0.0),
            tx_loss_db: None,
            rx_gain_db: Decibel::new(12.0),
            rx_loss_db: None,
            system_noise_temperature: Qty::from_raw(500.0),
            rx_g_over_t_dbk: None,
            atmospheric_loss_db: None,
            polarization_loss_db: None,
            pointing_loss_db: None,
            other_loss_db: None,
        };
        let derived = rf.evaluate(1.0e6);
        assert!((derived.g_over_t_dbk + 14.99).abs() < 0.01);
        assert!(!derived.g_over_t_given);
        rf.rx_g_over_t_dbk = Some(DecibelPerKelvin::new(-10.0));
        let given = rf.evaluate(1.0e6);
        assert_eq!(given.g_over_t_dbk, -10.0);
        assert!(given.g_over_t_given);
        let gain = given.c_over_n0_dbhz - derived.c_over_n0_dbhz;
        assert!((gain - 4.99).abs() < 0.01, "{gain}");
    }

    #[test]
    fn boltzmann_constant_in_db() {
        assert!((ratio_to_db(K_BOLTZMANN) + 228.6).abs() < 0.01);
        assert!((db_to_ratio(ratio_to_db(123.0)) - 123.0).abs() < 1e-9);
    }
}
