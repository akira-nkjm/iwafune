//! 大気密度（指数大気・NRLMSISE-00）。**大気モデルはここだけ。**
//!
//! - 指数大気 [`density_exponential_kg_m3`] — 指数大気（Vallado 表 8-4）。高度だけの静的モデル。
//! - [`nrlmsise00`] モジュール — NRLMSISE-00（太陽活動・地磁気活動・季節・緯度経度・地方時に応答する）。
//! - 入口 [`earth_density_kg_m3`] — [`AtmosphereModel`] で選んで、ECI の位置と時刻から密度を求める入口。
//!   時刻で変わる宇宙天気を渡すときは [`density_nrlmsise00_kg_m3`]。
//! - [`space_weather`] モジュール — 宇宙天気（F10.7・Ap）の日ごとの表（CelesTrak SW-All）と日付からの引き方、太陽活動のプリセット。
//!
//! 指数大気は、各層の基準高度 h₀・基準密度 ρ₀・スケールハイト H から `ρ = ρ₀ exp(−(h − h₀)/H)`。
//! 太陽活動の変化は扱わない（平均的な活動度の静的モデル）。1000 km より上は最上層を外挿する。

pub mod nrlmsise00;
pub mod space_weather;

use crate::earth::geodetic_from_eci;
use crate::ephem::CentralBody;
use crate::math::Vec3;
use crate::time::Epoch;
use serde::{Deserialize, Serialize};

/// 大気モデルの選択。
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "model", rename_all = "snake_case")]
pub enum AtmosphereModel {
    /// 指数大気（Vallado 表 8-4）
    #[default]
    Exponential,
    /// NRLMSISE-00。`f107` は前日の F10.7、`f107a` は 81 日平均 [sfu]、`ap` は日平均 Ap。
    Nrlmsise00 { f107: f64, f107a: f64, ap: f64 },
}

/// 地球の大気の、位置 `r_eci_m` [m]（地心）・時刻 `epoch` での密度 [kg/m³]。**地球が中心のときだけ**
/// （中心天体を選ぶ入口は [`density_about_kg_m3`]）。
///
/// 高度・緯度・経度は [`crate::earth::geodetic_from_eci`]（WGS84）で求める。NRLMSISE-00 の地方太陽時は
/// 原典の推奨どおり `UT [h] + 経度 [deg]/15`（平均太陽時）にする（[`nrlmsise00::Nrlmsise00Input::from_geodetic`]）。
pub fn earth_density_kg_m3(model: &AtmosphereModel, r_eci_m: Vec3, epoch: Epoch) -> f64 {
    match *model {
        AtmosphereModel::Exponential => {
            let (_, _, alt) = geodetic_from_eci(r_eci_m, epoch);
            density_exponential_kg_m3(alt)
        }
        AtmosphereModel::Nrlmsise00 { f107, f107a, ap } => {
            let weather = nrlmsise00::SpaceWeather {
                f107_daily: f107,
                f107_81day_avg: f107a,
                geomagnetic: nrlmsise00::GeomagneticActivity::DailyAp(ap),
            };
            density_nrlmsise00_kg_m3(&weather, r_eci_m, epoch)
        }
    }
}

/// 宇宙天気 `weather` を与えた NRLMSISE-00 の密度 [kg/m³]（位置 `r_eci_m` [m]・時刻 `epoch`）。
/// 日付で引いた宇宙天気（[`space_weather::DatedSpaceWeather::nrlmsise00`]）を時刻ごとに渡すときに使う。
pub fn density_nrlmsise00_kg_m3(
    weather: &nrlmsise00::SpaceWeather,
    r_eci_m: Vec3,
    epoch: Epoch,
) -> f64 {
    let (lat, lon, alt) = geodetic_from_eci(r_eci_m, epoch);
    let (doy, sec) = epoch.day_of_year_and_seconds();
    let input = nrlmsise00::Nrlmsise00Input::from_geodetic(
        doy,
        sec,
        alt,
        lat.to_degrees(),
        lon.to_degrees(),
        *weather,
    );
    nrlmsise00::evaluate(&input).total_mass_density_kg_m3
}

/// 中心天体 `central` のまわりの大気密度 [kg/m³]。大気のモデルは地球だけなので、ほかの中心天体では 0 を返す。
/// その 0 が「大気が無い」のか「大気はあるがモデルが無い」のかは [`CentralBody::atmosphere_model`] で分かる
/// （解析は後者のとき 0 を結果として出さず、利用者に伝える）。`r_rel_m` は中心天体からの位置 [m]。
pub fn density_about_kg_m3(
    central: CentralBody,
    model: &AtmosphereModel,
    r_rel_m: Vec3,
    epoch: Epoch,
) -> f64 {
    match central {
        CentralBody::Earth => earth_density_kg_m3(model, r_rel_m, epoch),
        _ => 0.0,
    }
}

/// (基準高度 [km], 基準密度 [kg/m³], スケールハイト [km])。Vallado, 4th ed., Table 8-4。
const LAYERS: [(f64, f64, f64); 28] = [
    (0.0, 1.225, 7.249),
    (25.0, 3.899e-2, 6.349),
    (30.0, 1.774e-2, 6.682),
    (40.0, 3.972e-3, 7.554),
    (50.0, 1.057e-3, 8.382),
    (60.0, 3.206e-4, 7.714),
    (70.0, 8.770e-5, 6.549),
    (80.0, 1.905e-5, 5.799),
    (90.0, 3.396e-6, 5.382),
    (100.0, 5.297e-7, 5.877),
    (110.0, 9.661e-8, 7.263),
    (120.0, 2.438e-8, 9.473),
    (130.0, 8.484e-9, 12.636),
    (140.0, 3.845e-9, 16.149),
    (150.0, 2.070e-9, 22.523),
    (180.0, 5.464e-10, 29.740),
    (200.0, 2.789e-10, 37.105),
    (250.0, 7.248e-11, 45.546),
    (300.0, 2.418e-11, 53.628),
    (350.0, 9.518e-12, 53.298),
    (400.0, 3.725e-12, 58.515),
    (450.0, 1.585e-12, 60.828),
    (500.0, 6.967e-13, 63.822),
    (600.0, 1.454e-13, 71.835),
    (700.0, 3.614e-14, 88.667),
    (800.0, 1.170e-14, 124.64),
    (900.0, 5.245e-15, 181.05),
    (1000.0, 3.019e-15, 268.00),
];

/// 高度 `altitude_m` [m] での密度 [kg/m³]。負の高度は 0 m として扱う。NaN には NaN を返す。
pub fn density_exponential_kg_m3(altitude_m: f64) -> f64 {
    if altitude_m.is_nan() {
        return f64::NAN;
    }
    let h_km = altitude_m.max(0.0) / 1e3;
    let (h0, rho0, scale) = LAYERS
        .iter()
        .rev()
        .find(|(base, _, _)| *base <= h_km)
        .copied()
        .unwrap_or(LAYERS[0]);
    rho0 * (-(h_km - h0) / scale).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_values_at_layer_bases() {
        assert!((density_exponential_kg_m3(0.0) - 1.225).abs() < 1e-12);
        let rho400 = density_exponential_kg_m3(400e3);
        assert!((rho400 / 3.725e-12 - 1.0).abs() < 1e-12);
    }

    /// Vallado 例: 550 km では ρ = 6.967e-13·exp(−50/63.822) ≈ 3.19e-13 kg/m³。
    #[test]
    fn density_at_550_km() {
        let rho = density_exponential_kg_m3(550e3);
        let expected = 6.967e-13 * (-50.0_f64 / 63.822).exp();
        assert!((rho / expected - 1.0).abs() < 1e-12);
        assert!(rho > 3.0e-13 && rho < 3.4e-13, "{rho}");
    }

    #[test]
    fn model_selection_from_eci_position() {
        use crate::earth::{ecef_to_eci, geodetic_to_ecef};
        let e = Epoch::parse("2027-04-01T12:00:00Z").expect("epoch");
        let r = ecef_to_eci(geodetic_to_ecef(0.3, 1.0, 420e3), e);
        let exp = earth_density_kg_m3(&AtmosphereModel::Exponential, r, e);
        assert!((exp / density_exponential_kg_m3(420e3) - 1.0).abs() < 1e-6);
        let quiet = AtmosphereModel::Nrlmsise00 {
            f107: 70.0,
            f107a: 70.0,
            ap: 4.0,
        };
        let active = AtmosphereModel::Nrlmsise00 {
            f107: 250.0,
            f107a: 250.0,
            ap: 15.0,
        };
        let (rq, ra) = (
            earth_density_kg_m3(&quiet, r, e),
            earth_density_kg_m3(&active, r, e),
        );
        assert!(rq > 1e-13 && rq < 1e-11, "{rq}");
        // 太陽活動が高いと 400 km の密度は数倍になる
        assert!(ra / rq > 3.0, "{rq} → {ra}");
        let json = r#"{"model":"nrlmsise00","f107":150.0,"f107a":150.0,"ap":4.0}"#;
        let parsed: AtmosphereModel = serde_json::from_str(json).expect("JSON");
        assert_eq!(
            parsed,
            AtmosphereModel::Nrlmsise00 {
                f107: 150.0,
                f107a: 150.0,
                ap: 4.0
            }
        );
    }

    #[test]
    fn decreases_with_altitude_and_is_nearly_continuous() {
        let mut prev = f64::INFINITY;
        for i in 0..=1500 {
            let h = f64::from(i) * 1e3;
            let rho = density_exponential_kg_m3(h);
            assert!(rho < prev, "{h} m");
            prev = rho;
        }
        // 層の境目での跳びは数 % 以内
        for (base, _, _) in LAYERS.iter().skip(1) {
            let below = density_exponential_kg_m3(base * 1e3 - 1.0);
            let above = density_exponential_kg_m3(base * 1e3);
            assert!((below / above - 1.0).abs() < 0.2, "{base} km");
        }
        assert_eq!(density_exponential_kg_m3(-10.0), 1.225);
    }
}
