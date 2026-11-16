//! 熱環境（太陽光・中心天体のアルベドと赤外放射・宇宙の背景温度）と、中心天体に合わせた値の決め方。
//!
//! 値は「書いた値 → プリセット（`thermal::preset`）→ 中心天体の既定値（`thermal::planet`）」の順に決める。
//! プリセットの太陽光（近日点・遠日点の 1 AU の値）は太陽距離の慣例値が 1 AU の天体（地球・月）だけ、
//! プリセットのアルベド・赤外（LEO の代表値）は地球だけに使う。決まらない値は `InvalidInput`（入力を書く）。

use crate::spec::AnalysisError;
use crate::thermal::planet::{PlanetIr, planet_environment};
use crate::thermal::preset::ThermalPreset;
use sat_astro::ephem::CentralBody;
use sat_units::typed::{Fraction, Irradiance, Qty, Temperature};
use serde::{Deserialize, Serialize};

/// 熱環境（高温・低温のケースで値を変える）。書かれていない値はプリセット、それも無ければ中心天体の既定値。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalEnvironment {
    /// 最悪ケースのプリセット（書かれていない太陽光・アルベド・赤外に使う。`thermal::preset`）
    #[serde(default)]
    pub preset: Option<ThermalPreset>,
    /// 太陽光の強さ（既定は太陽との距離から。高温ケースは 1414 W/m² などを与える）
    #[serde(default)]
    pub solar_flux: Option<Qty<Irradiance>>,
    /// 中心天体のアルベド（既定は天体ごと。`thermal::planet`）
    #[serde(default)]
    pub albedo: Option<Fraction>,
    /// 中心天体の赤外放射（一律の値。既定は天体ごとで、月は直下点の地方時で変わる。`thermal::planet`）
    #[serde(default)]
    pub planet_ir: Option<Qty<Irradiance>>,
    /// 宇宙の背景温度（既定 [`DEFAULT_SPACE_TEMPERATURE_K`]）
    #[serde(default)]
    pub space_temperature: Option<Qty<Temperature>>,
}

/// 宇宙の背景温度の既定値 [K]。
pub const DEFAULT_SPACE_TEMPERATURE_K: f64 = 3.0;

/// 中心天体に合わせて決めた熱環境（入熱の計算に使う）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ResolvedEnvironment {
    /// 太陽光の強さ [W/m²]（`None` なら時刻ごとの太陽との距離から）
    pub solar_flux_w_m2: Option<f64>,
    pub albedo: f64,
    pub ir: PlanetIr,
    /// 中心天体の半径 [m]（形態係数に使う）
    pub body_radius_m: f64,
}

fn missing(central: CentralBody, what: &str, key: &str) -> AnalysisError {
    AnalysisError::InvalidInput(format!(
        "No default {what} for central body {central:?}. Set {key} in the environment conditions"
    ))
}

impl ThermalEnvironment {
    /// 宇宙の背景温度（省略時 [`DEFAULT_SPACE_TEMPERATURE_K`]）。
    pub fn space_temperature(&self) -> Qty<Temperature> {
        self.space_temperature
            .unwrap_or(Qty::from_raw(DEFAULT_SPACE_TEMPERATURE_K))
    }

    /// 中心天体 `central` に合わせて値を決める（モジュールの説明の順）。
    pub(crate) fn resolve(
        &self,
        central: CentralBody,
    ) -> Result<ResolvedEnvironment, AnalysisError> {
        let earth_preset = self.preset.filter(|_| central == CentralBody::Earth);
        let defaults = planet_environment(central);
        let solar_flux_w_m2 = match (self.solar_flux, self.preset) {
            (Some(f), _) => Some(f.value()),
            (None, Some(p)) if central.nominal_sun_distance_m().is_some() => {
                Some(p.solar_flux().value())
            }
            (None, Some(_)) => {
                return Err(AnalysisError::InvalidInput(format!(
                    "The preset solar flux is the 1 AU value, so set solar_flux for central body {central:?}"
                )));
            }
            (None, None) => None,
        };
        let albedo = self
            .albedo
            .or_else(|| earth_preset.map(ThermalPreset::albedo))
            .map(Fraction::value)
            .or_else(|| defaults.map(|d| d.albedo))
            .ok_or_else(|| missing(central, "albedo", "albedo"))?;
        let ir = self
            .planet_ir
            .or_else(|| earth_preset.map(ThermalPreset::earth_ir))
            .map(|q| PlanetIr::Uniform(q.value()))
            .or_else(|| defaults.map(|d| d.ir))
            .ok_or_else(|| missing(central, "planet IR", "planet_ir"))?;
        Ok(ResolvedEnvironment {
            solar_flux_w_m2,
            albedo,
            ir,
            body_radius_m: central.radius_m(),
        })
    }

    /// 使った値（中心天体 `central` に合わせて既定値を埋めたもの）。出力に返す。
    pub fn used(&self, central: CentralBody) -> Result<ThermalEnvironmentUsed, AnalysisError> {
        let r = self.resolve(central)?;
        let (planet_ir, night_ir) = match r.ir {
            PlanetIr::Uniform(v) => (Some(Qty::from_raw(v)), None),
            PlanetIr::LocalTime { night_w_m2 } => (None, Some(Qty::from_raw(night_w_m2))),
        };
        Ok(ThermalEnvironmentUsed {
            preset: self.preset,
            solar_flux: r.solar_flux_w_m2.map(Qty::from_raw),
            albedo: Fraction::new(r.albedo),
            planet_ir,
            space_temperature: self.space_temperature(),
            night_ir,
        })
    }
}

/// 解析に使った熱環境（既定値を埋めたもの）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalEnvironmentUsed {
    /// 使ったプリセット
    pub preset: Option<ThermalPreset>,
    /// 太陽光の強さ（`None` なら時刻ごとの太陽との距離から求めた）
    pub solar_flux: Option<Qty<Irradiance>>,
    /// 中心天体のアルベド
    pub albedo: Fraction,
    /// 中心天体の一律の赤外放射（地方時で変わるモデルなら `None`。そのときは `night_ir`）
    pub planet_ir: Option<Qty<Irradiance>>,
    pub space_temperature: Qty<Temperature>,
    /// 地方時で変わる赤外放射（月）の夜側の値。昼側は `(1 − アルベド)·太陽光·cos θ`（θ は直下点の太陽天頂角）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub night_ir: Option<Qty<Irradiance>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// プリセットは書かれていない値だけを埋める（地球）: 高温にアルベド 0.3 を書くと、太陽光 1414・赤外 258 は
    /// プリセット、アルベドは書いた 0.3。
    #[test]
    fn preset_fills_unwritten_values() {
        let env = ThermalEnvironment {
            preset: Some(ThermalPreset::Hot),
            albedo: Some(Fraction::new(0.3)),
            ..ThermalEnvironment::default()
        };
        let used = env.used(CentralBody::Earth).unwrap();
        assert_eq!(used.solar_flux.map(Qty::value), Some(1414.0));
        assert_eq!(used.albedo.value(), 0.3);
        assert_eq!(used.planet_ir.map(Qty::value), Some(258.0));
        assert!(used.night_ir.is_none());
    }

    /// 月ではプリセットは太陽光だけを決め、アルベド・赤外は月の既定値（赤外は地方時で変わる）。
    #[test]
    fn lunar_preset_uses_lunar_albedo_and_ir() {
        let env = ThermalEnvironment {
            preset: Some(ThermalPreset::Hot),
            ..ThermalEnvironment::default()
        };
        let used = env.used(CentralBody::Moon).unwrap();
        assert_eq!(used.solar_flux.map(Qty::value), Some(1414.0));
        assert_eq!(used.albedo.value(), 0.12);
        assert!(used.planet_ir.is_none());
        assert!((used.night_ir.unwrap().value() - 5.387).abs() < 1e-2);
    }

    /// 既定値の無い天体（火星）はアルベドと赤外を書かないと止まり、プリセットの 1 AU の太陽光も使えない。
    #[test]
    fn bodies_without_defaults_need_values() {
        let mut env = ThermalEnvironment::default();
        assert!(env.used(CentralBody::Mars).is_err());
        env.albedo = Some(Fraction::new(0.25));
        env.planet_ir = Some(Qty::from_raw(110.0));
        assert!(env.used(CentralBody::Mars).is_ok());
        env.preset = Some(ThermalPreset::Cold);
        assert!(env.used(CentralBody::Mars).is_err());
        env.solar_flux = Some(Qty::from_raw(586.0));
        let used = env.used(CentralBody::Mars).unwrap();
        assert_eq!(used.albedo.value(), 0.25);
        assert_eq!(used.planet_ir.map(Qty::value), Some(110.0));
    }
}
