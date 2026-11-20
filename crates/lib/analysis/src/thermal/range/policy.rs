//! 解析マージンの方針（予測温度の不確かさ・範囲の端の警告の幅・ヒータのデューティの警告）と、使った値。
//! 書かれていない値の既定値はここだけ。温度範囲との比較と判定は親の `range`、ヒータの判定は `heater`。

use sat_units::typed::{Fraction, Qty, TemperatureDifference};
use serde::{Deserialize, Serialize};

/// 範囲の端からこの幅 [K] 以内なら警告にする（解析マージンの方針の `warning_band` の既定値）。
pub const RANGE_WARNING_BAND_K: f64 = 5.0;

/// 予測温度の不確かさの既定値 [K]（解析マージンの方針の `uncertainty` の既定値）。0 なら予測温度をそのまま比べる。
pub const DEFAULT_UNCERTAINTY_K: f64 = 0.0;

/// ヒータのデューティがこれを超えたら警告にする既定値（`heater_duty_warning` が書かれていないとき）。ヒータの余裕が 2 割を切ると、
/// 設計の見積りの誤差や劣化で飽和しうるので注意を促す。
pub const DEFAULT_HEATER_DUTY_WARNING: f64 = 0.8;

/// 解析マージンの方針。書かれていない値は解析の既定値（取り出しはメソッド、使った値は [`Self::used`]）。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalMarginPolicy {
    /// 予測温度の不確かさ（最高に足し、最低から引いてから範囲と比べる。既定 [`DEFAULT_UNCERTAINTY_K`]）
    #[serde(
        default,
        serialize_with = "sat_units::typed::TemperatureDifferenceKind::serialize_option"
    )]
    pub uncertainty: Option<Qty<TemperatureDifference>>,
    /// 範囲の端からこの幅以内を警告にする（既定 [`RANGE_WARNING_BAND_K`]）
    #[serde(
        default,
        serialize_with = "sat_units::typed::TemperatureDifferenceKind::serialize_option"
    )]
    pub warning_band: Option<Qty<TemperatureDifference>>,
    /// ヒータのデューティがこれを超えたら警告（0〜1。既定 [`DEFAULT_HEATER_DUTY_WARNING`]）
    #[serde(default)]
    pub heater_duty_warning: Option<Fraction>,
}

impl ThermalMarginPolicy {
    /// 予測温度の不確かさ [K]（省略時 [`DEFAULT_UNCERTAINTY_K`]）。
    pub fn uncertainty(&self) -> Qty<TemperatureDifference> {
        self.uncertainty
            .unwrap_or(Qty::from_raw(DEFAULT_UNCERTAINTY_K))
    }

    /// 警告の幅 [K]（省略時 [`RANGE_WARNING_BAND_K`]）。
    pub fn warning_band(&self) -> Qty<TemperatureDifference> {
        self.warning_band
            .unwrap_or(Qty::from_raw(RANGE_WARNING_BAND_K))
    }

    /// ヒータのデューティの警告の値（省略時 [`DEFAULT_HEATER_DUTY_WARNING`]）。
    pub fn heater_duty_warning(&self) -> Fraction {
        self.heater_duty_warning
            .unwrap_or(Fraction::new(DEFAULT_HEATER_DUTY_WARNING))
    }

    /// 使った値（既定値を埋めたもの）。出力に返す。
    pub fn used(&self) -> ThermalMarginUsed {
        ThermalMarginUsed {
            uncertainty: self.uncertainty(),
            warning_band: self.warning_band(),
            heater_duty_warning: self.heater_duty_warning(),
        }
    }
}

fn default_heater_duty_warning() -> Fraction {
    Fraction::new(DEFAULT_HEATER_DUTY_WARNING)
}

/// 解析に使った解析マージン（既定値を埋めたもの）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalMarginUsed {
    /// 予測温度の不確かさ（温度差 [K]）
    #[serde(serialize_with = "sat_units::typed::TemperatureDifferenceKind::serialize")]
    pub uncertainty: Qty<TemperatureDifference>,
    /// 範囲の端からこの幅以内を警告にした（温度差 [K]。Web の温度表もこれを使う）
    #[serde(serialize_with = "sat_units::typed::TemperatureDifferenceKind::serialize")]
    pub warning_band: Qty<TemperatureDifference>,
    /// ヒータのデューティがこれを超えたら警告にした（0〜1。書かれていない古い出力は既定値）
    #[serde(default = "default_heater_duty_warning")]
    pub heater_duty_warning: Fraction,
}

impl ThermalMarginUsed {
    /// 入力を検査する（不確かさと警告の幅は 0 以上の有限の値、ヒータのデューティの警告は 0〜1）。
    pub(crate) fn validate(&self) -> Result<(), crate::spec::AnalysisError> {
        let ok = |v: f64| v.is_finite() && v >= 0.0;
        let duty = self.heater_duty_warning.value();
        if ok(self.uncertainty.value())
            && ok(self.warning_band.value())
            && (0.0..=1.0).contains(&duty)
        {
            Ok(())
        } else {
            Err(crate::spec::AnalysisError::InvalidInput(
                "Analysis margin uncertainty and warning width must be 0 or more, and the heater duty warning within 0 to 1".to_owned(),
            ))
        }
    }
}
