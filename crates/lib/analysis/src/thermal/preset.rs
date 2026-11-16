//! 最悪ケースのプリセット（高温・低温・セーフモード）の熱環境の既定値。**プリセットの値はここだけ。**
//!
//! 環境条件にプリセットを書くと、書かれていない太陽光の強さ・アルベド・地球赤外にプリセットの値を使う
//! （書いた値はプリセットより優先。`ThermalEnvironment` のメソッドが引く）。値は概念設計で使う LEO の代表値。
//!
//! | プリセット | 太陽光 [W/m²] | アルベド | 地球赤外 [W/m²] | 組み合わせるもの |
//! |---|---|---|---|---|
//! | `hot` | 1414（近日点） | 0.35 | 258 | 発熱の多いモード・β の大きい軌道 |
//! | `cold` | 1322（遠日点） | 0.25 | 217 | 発熱の少ないモード・日陰の長い軌道 |
//! | `safe_mode` | 1322 | 0.25 | 217 | セーフモード（低温の環境で最小の発熱。ヒーターだけで保温できるかを見る） |

use sat_units::typed::{Fraction, Irradiance, Qty};
use serde::{Deserialize, Serialize};

/// 最悪ケースのプリセット。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ThermalPreset {
    /// 高温の最悪ケース
    Hot,
    /// 低温の最悪ケース
    Cold,
    /// セーフモード（低温の環境）
    SafeMode,
}

/// 高温の太陽光の強さ [W/m²]（近日点）。
const HOT_SOLAR_FLUX: f64 = 1414.0;
/// 低温の太陽光の強さ [W/m²]（遠日点）。
const COLD_SOLAR_FLUX: f64 = 1322.0;
/// 高温のアルベド。
const HOT_ALBEDO: f64 = 0.35;
/// 低温のアルベド。
const COLD_ALBEDO: f64 = 0.25;
/// 高温の地球赤外 [W/m²]。
const HOT_EARTH_IR: f64 = 258.0;
/// 低温の地球赤外 [W/m²]。
const COLD_EARTH_IR: f64 = 217.0;

impl ThermalPreset {
    fn is_hot(self) -> bool {
        self == Self::Hot
    }

    /// 太陽光の強さ。
    pub fn solar_flux(self) -> Qty<Irradiance> {
        Qty::from_raw(if self.is_hot() {
            HOT_SOLAR_FLUX
        } else {
            COLD_SOLAR_FLUX
        })
    }

    /// 地球のアルベド。
    pub fn albedo(self) -> Fraction {
        Fraction::new(if self.is_hot() {
            HOT_ALBEDO
        } else {
            COLD_ALBEDO
        })
    }

    /// 地球の赤外放射。
    pub fn earth_ir(self) -> Qty<Irradiance> {
        Qty::from_raw(if self.is_hot() {
            HOT_EARTH_IR
        } else {
            COLD_EARTH_IR
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 高温は低温より太陽光・アルベド・地球赤外のどれも大きく、セーフモードは低温と同じ環境。
    #[test]
    fn hot_is_hotter_and_safe_mode_is_cold() {
        let (h, c, s) = (
            ThermalPreset::Hot,
            ThermalPreset::Cold,
            ThermalPreset::SafeMode,
        );
        assert!(h.solar_flux().value() > c.solar_flux().value());
        assert!(h.albedo().value() > c.albedo().value());
        assert!(h.earth_ir().value() > c.earth_ir().value());
        assert_eq!(s.solar_flux().value(), c.solar_flux().value());
        assert_eq!(s.albedo().value(), c.albedo().value());
        assert_eq!(s.earth_ir().value(), c.earth_ir().value());
    }
}
