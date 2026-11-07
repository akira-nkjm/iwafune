//! 回線計算の部品（斜距離・自由空間損失・EIRP・G/T・C/N0・Eb/N0）。
//! **回線計算はここだけ。** `comm.link_budget` と `comm.data_throughput` が使う。
//!
//! dB の値は f64 で持ち、名前に `_db`（`_dbw`・`_dbk`・`_dbhz`）を付ける。すべて電力比の `10 log10`。
//! 線形の量（周波数・電力・雑音温度・データレート）は SI の `Qty` で持つ。

use crate::spec::AnalysisError;
use sat_astro::constants::{C_LIGHT, K_BOLTZMANN, R_EARTH};
use sat_astro::math::bisect;
use sat_units::typed::{Frequency, Power, Qty, Temperature};
use sat_units::{db_to_ratio, ratio_to_db};
use serde::{Deserialize, Serialize};
use std::f64::consts::{FRAC_PI_2, PI};

/// 要求する回線余裕の既定値 [dB]。**既定値はここだけ**（`comm.link_budget` と `comm.data_throughput` が使う）。
pub const DEFAULT_REQUIRED_MARGIN_DB: f64 = 3.0;

/// 送受信端と経路の RF パラメーター。データレートと所要 Eb/N0 は含まない（使う側の入力に置く）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RfLinkParams {
    /// 搬送波の周波数
    pub frequency: Qty<Frequency>,
    /// 送信電力（アンテナ給電点の手前）
    pub tx_power: Qty<Power>,
    /// 送信アンテナ利得 [dBi]
    pub tx_gain_db: f64,
    /// 送信側の給電損失 [dB]（正の値が損失）。省略時 0
    #[serde(default)]
    pub tx_loss_db: Option<f64>,
    /// 受信アンテナ利得 [dBi]
    pub rx_gain_db: f64,
    /// 受信側の給電損失 [dB]（LNA の手前。正の値が損失）。省略時 0
    #[serde(default)]
    pub rx_loss_db: Option<f64>,
    /// 受信系のシステム雑音温度
    pub system_noise_temperature: Qty<Temperature>,
    /// 大気・降雨の損失 [dB]。省略時 0
    #[serde(default)]
    pub atmospheric_loss_db: Option<f64>,
    /// 偏波損失 [dB]。省略時 0
    #[serde(default)]
    pub polarization_loss_db: Option<f64>,
    /// 指向損失 [dB]。省略時 0
    #[serde(default)]
    pub pointing_loss_db: Option<f64>,
    /// その他の損失（実装損失など）[dB]。省略時 0
    #[serde(default)]
    pub other_loss_db: Option<f64>,
}

/// 書かれていない損失は 0 dB（損失なし）とする。
fn loss_or_none(loss_db: Option<f64>) -> f64 {
    loss_db.unwrap_or(0.0)
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
    /// C/N0 [dBHz]
    pub c_over_n0_dbhz: f64,
}

/// 地上局から高度 `altitude_m` の衛星を仰角 `elevation_rad` で見たときの斜距離 [m]（球の地球）。
pub fn slant_range_m(altitude_m: f64, elevation_rad: f64) -> f64 {
    let r = R_EARTH + altitude_m;
    let (sin_e, cos_e) = elevation_rad.sin_cos();
    (r * r - (R_EARTH * cos_e).powi(2)).sqrt() - R_EARTH * sin_e
}

/// 自由空間損失 [dB] = 20 log10(4π d f / c)。
pub fn fspl_db(range_m: f64, frequency_hz: f64) -> f64 {
    20.0 * (4.0 * PI * range_m * frequency_hz / C_LIGHT).log10()
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
            self.tx_gain_db,
            self.tx_loss(),
            self.rx_gain_db,
            self.rx_loss(),
            self.extra_loss_db(),
        ];
        if positive.iter().any(|v| !(v.is_finite() && *v > 0.0))
            || finite.iter().any(|v| !v.is_finite())
        {
            return Err(AnalysisError::InvalidInput(format!(
                "{what}: 周波数・送信電力・雑音温度は正、利得と損失は有限の値にしてください"
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
        let eirp = eirp_dbw(self.tx_power.value(), self.tx_gain_db, self.tx_loss());
        let fspl = fspl_db(range_m, self.frequency.value());
        let extra = self.extra_loss_db();
        let g_t = g_over_t_dbk(
            self.rx_gain_db,
            self.rx_loss(),
            self.system_noise_temperature.value(),
        );
        LinkEvaluation {
            eirp_dbw: eirp,
            fspl_db: fspl,
            extra_loss_db: extra,
            g_over_t_dbk: g_t,
            c_over_n0_dbhz: c_over_n0_dbhz(eirp, fspl + extra, g_t),
        }
    }

    /// Eb/N0 の余裕 [dB]（`eb_n0 − required − margin` ではなく `eb_n0 − required`）。
    pub fn eb_n0_margin_db(&self, range_m: f64, data_rate_bps: f64, required_eb_n0_db: f64) -> f64 {
        eb_over_n0_db(self.evaluate(range_m).c_over_n0_dbhz, data_rate_bps) - required_eb_n0_db
    }

    /// 高度 `altitude_m` で、Eb/N0 の余裕が `margin_db` 以上になる最低の仰角 [rad]。
    /// 天頂でも足りなければ `None`、水平線でも足りれば 0。
    pub fn min_elevation_for_margin(
        &self,
        altitude_m: f64,
        data_rate_bps: f64,
        required_eb_n0_db: f64,
        margin_db: f64,
    ) -> Option<f64> {
        let excess = |e: f64| {
            self.eb_n0_margin_db(
                slant_range_m(altitude_m, e),
                data_rate_bps,
                required_eb_n0_db,
            ) - margin_db
        };
        if excess(FRAC_PI_2) < 0.0 {
            return None;
        }
        if excess(0.0) >= 0.0 {
            return Some(0.0);
        }
        // 余裕は仰角に対して単調に増えるので二分法で求める。
        bisect(excess, 0.0, FRAC_PI_2, 1e-12, 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fspl_437mhz_1000km() {
        // 20 log10(4π·1e6·437e6/c) = 145.25 dB
        let l = fspl_db(1.0e6, 437.0e6);
        assert!((l - 145.25).abs() < 0.02, "{l}");
    }

    #[test]
    fn slant_range_at_zenith_and_horizon() {
        let h = 500.0e3;
        assert!((slant_range_m(h, FRAC_PI_2) - h).abs() < 1e-6);
        let horizon = ((R_EARTH + h).powi(2) - R_EARTH.powi(2)).sqrt();
        assert!((slant_range_m(h, 0.0) - horizon).abs() < 1e-6);
    }

    #[test]
    fn boltzmann_constant_in_db() {
        assert!((ratio_to_db(K_BOLTZMANN) + 228.6).abs() < 0.01);
        assert!((db_to_ratio(ratio_to_db(123.0)) - 123.0).abs() < 1e-9);
    }
}
