//! 中心天体ごとの熱環境の既定値（アルベドと天体の赤外放射）。**天体ごとの環境の既定値はここだけ。**
//!
//! | 天体 | アルベド | 赤外放射 | 出典 |
//! |---|---|---|---|
//! | 地球 | 0.30（`sat_astro::constants::EARTH_ALBEDO`） | 一律 237 W/m²（`EARTH_IR`。大気上端の平均） | Gilmore, *Spacecraft Thermal Control Handbook* Vol. I, 2nd ed., §2 |
//! | 月 | 0.12（[`MOON_ALBEDO`]） | 直下点の太陽天頂角 θ で変わる: 昼側 `(1 − A)·S·cos θ`、夜側の下限 `ε σ T_夜⁴`（[`MOON_NIGHT_TEMPERATURE_K`]・[`MOON_EMISSIVITY`]） | NASA GSFC Moon Fact Sheet（幾何アルベド 0.12）、Williams et al., Icarus 283 (2017)（Diviner: 赤道の昼 ≈ 390 K・夜 ≈ 95〜100 K）、Vasavada et al., JGR 117 (2012)（放射率 0.95） |
//!
//! 月の昼側は熱慣性が小さく、吸収した太陽光をその場で放射するとみなす（放射平衡）。直下点の温度は
//! `T = ((1 − A) S cos θ / (ε σ))^¼` で、S = 1414 W/m² なら ≈ 390 K、夜側は ≈ 100 K で一定。
//! ここに無い天体（太陽・水星・金星・火星・木星型惑星）は既定値を持たず、解析がアルベドと赤外放射の入力を求める。
//! 地球周回で月から受けるアルベド・赤外は地球の数万分の 1 以下なので扱わない。

use sat_astro::constants::{EARTH_ALBEDO, EARTH_IR, SIGMA_SB};
use sat_astro::ephem::CentralBody;

/// 月のアルベド。
pub const MOON_ALBEDO: f64 = 0.12;
/// 月の夜側の表面温度 [K]。
pub const MOON_NIGHT_TEMPERATURE_K: f64 = 100.0;
/// 月の表面の赤外の放射率。
pub const MOON_EMISSIVITY: f64 = 0.95;

/// 中心天体の赤外放射のモデル。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlanetIr {
    /// どこでも同じ値 [W/m²]（地球、または値を書いたとき）
    Uniform(f64),
    /// 直下点の太陽天頂角で変わる（月）。昼側は吸収した太陽光の再放射、夜側の下限は `night_w_m2` [W/m²]
    LocalTime { night_w_m2: f64 },
}

impl PlanetIr {
    /// 直下点から出る赤外放射 [W/m²]。`albedo` は天体のアルベド、`solar_flux_w_m2` は太陽光の強さ、
    /// `cos_sun` は直下点の太陽天頂角の cos（夜側は 0 以下）。
    pub fn emitted_w_m2(&self, albedo: f64, solar_flux_w_m2: f64, cos_sun: f64) -> f64 {
        match *self {
            Self::Uniform(v) => v,
            Self::LocalTime { night_w_m2 } => {
                ((1.0 - albedo) * solar_flux_w_m2 * cos_sun.max(0.0)).max(night_w_m2)
            }
        }
    }
}

/// 中心天体の熱環境の既定値。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanetEnvironment {
    pub albedo: f64,
    pub ir: PlanetIr,
}

/// 中心天体の熱環境の既定値（既定値を持たない天体は `None`）。
pub fn planet_environment(central: CentralBody) -> Option<PlanetEnvironment> {
    match central {
        CentralBody::Earth => Some(PlanetEnvironment {
            albedo: EARTH_ALBEDO,
            ir: PlanetIr::Uniform(EARTH_IR),
        }),
        CentralBody::Moon => Some(PlanetEnvironment {
            albedo: MOON_ALBEDO,
            ir: PlanetIr::LocalTime {
                night_w_m2: MOON_EMISSIVITY * SIGMA_SB * MOON_NIGHT_TEMPERATURE_K.powi(4),
            },
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 月の直下点の温度: 高温（1414 W/m²）の正午で ≈ 390 K、夜側は 100 K（σ T⁴ ε ≈ 5.4 W/m²）。
    #[test]
    fn lunar_surface_temperature_by_local_time() {
        let env = planet_environment(CentralBody::Moon).unwrap();
        let temp = |cos_sun: f64| {
            let e = env.ir.emitted_w_m2(env.albedo, 1414.0, cos_sun);
            (e / (MOON_EMISSIVITY * SIGMA_SB)).powf(0.25)
        };
        assert!((temp(1.0) - 390.0).abs() < 2.0, "{}", temp(1.0));
        assert!((temp(-0.5) - 100.0).abs() < 1e-9);
        assert!((temp(0.0) - 100.0).abs() < 1e-9);
        // 太陽天頂角 60° では cos^¼ 倍
        assert!((temp(0.5) - temp(1.0) * 0.5_f64.powf(0.25)).abs() < 1e-9);
        assert!(planet_environment(CentralBody::Mars).is_none());
        assert_eq!(
            planet_environment(CentralBody::Earth)
                .unwrap()
                .ir
                .emitted_w_m2(0.3, 1361.0, 1.0),
            EARTH_IR
        );
    }
}
