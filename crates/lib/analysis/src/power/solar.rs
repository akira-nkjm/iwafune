//! 太陽電池の発電モデル。**日照時（`.map(SolarPanel::eol)ower.solar_sunlit`）・軌道上（`power.orbit_generation`）・
//! 収支（`power.energy_balance`）の 3 つの解析はこのモデルだけを使う。**
//!
//! パネル 1 枚の発電（バス側）:
//! `P = Φ · A · η_cell · f_pack · η_conv · f_life · max(0, n̂·ŝ) · ν`
//!
//! - Φ: 太陽光の強さ（[`SolarArray::solar_flux`] の上書きが無ければ太陽までの距離から求める）
//! - A: パネル面積、η_cell: セル効率、f_pack: 実装率、η_conv: パネル → バスの変換効率
//! - f_life: BOL なら 1、EOL なら `eol_factor`
//! - n̂: パネルの法線（機体座標系）、ŝ: 太陽方向（機体座標系）、ν: 日照率（入射は `sat_astro::illumination`）

use crate::spec::{AnalysisError, ensure_fraction, ensure_non_negative};
use sat_astro::constants::SOLAR_CONSTANT;
use sat_astro::illumination::{solar_flux_w_m2, solar_incidence};
use sat_astro::math::Vec3;
use sat_astro::timeline::{TimelineSample, Window, shadow_windows};
use sat_units::typed::{Area, Irradiance, Power, Qty};
use serde::{Deserialize, Serialize};

/// 日照とみなす日照率の下限。これ以上を「日照中」、未満を「日陰」として区間や平均を数える（半影の中央）。
pub const SUNLIT_THRESHOLD: f64 = 0.5;

/// 太陽電池パネル 1 枚。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SolarPanel {
    /// パネルの ID（部品 ID や面の名前。出力の内訳に使う）
    pub id: String,
    /// セルを貼る面の面積
    pub area: Qty<Area>,
    /// セル効率（0〜1）
    pub cell_efficiency: f64,
    /// 実装率（面積のうちセルが占める割合、0〜1）。省略時 1
    #[serde(default)]
    pub packing_factor: Option<f64>,
    /// EOL 係数（寿命末期の出力 / 初期の出力、0〜1）。省略時 1（劣化なし）
    #[serde(default)]
    pub eol_factor: Option<f64>,
    /// 発電面の外向き法線（機体座標系。長さは問わず、正規化して使う）
    pub normal: Vec3,
}

/// 太陽電池の組（パネルの一覧と、組全体にかかる条件）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SolarArray {
    pub panels: Vec<SolarPanel>,
    /// 太陽光の強さの上書き。省略時は時系列の太陽距離から求め、方向だけ与える場合は 1 AU の太陽定数を使う
    #[serde(default)]
    pub solar_flux: Option<Qty<Irradiance>>,
    /// パネル → バスの変換効率（MPPT・配線など、0〜1）。省略時は 1
    #[serde(default)]
    pub conversion_efficiency: Option<f64>,
}

/// 寿命のどの時点で評価するか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SolarLife {
    /// 初期（Beginning of Life）
    Bol,
    /// 寿命末期（End of Life）。`eol_factor` を掛ける
    #[default]
    Eol,
}

impl SolarPanel {
    /// 実装率（省略時 1）。
    pub fn packing(&self) -> f64 {
        self.packing_factor.unwrap_or(1.0)
    }

    /// EOL 係数（省略時 1）。
    pub fn eol(&self) -> f64 {
        self.eol_factor.unwrap_or(1.0)
    }

    fn validate(&self) -> Result<(), AnalysisError> {
        let name = |what: &str| format!("太陽電池パネル {} の{what}", self.id);
        ensure_non_negative(&name("面積"), self.area.value())?;
        ensure_fraction(&name("セル効率"), self.cell_efficiency)?;
        ensure_fraction(&name("実装率"), self.packing())?;
        ensure_fraction(&name("EOL 係数"), self.eol())?;
        if self.normal.normalized().is_none() {
            return Err(AnalysisError::InvalidInput(name("法線がゼロベクトルです")));
        }
        Ok(())
    }

    /// 太陽光の強さ `flux`、入射係数 `incidence`（cos と日照率の積）での発電 [W]（変換効率は含まない）。
    fn power(&self, flux: Qty<Irradiance>, incidence: f64, life: SolarLife) -> Qty<Power> {
        let life_factor = match life {
            SolarLife::Bol => 1.0,
            SolarLife::Eol => self.eol(),
        };
        flux * self.area * (self.cell_efficiency * self.packing() * life_factor * incidence)
    }
}

impl SolarArray {
    /// パネルの値が物理的にありうるかを調べる（パネルが 0 枚でもよい。その場合の発電は 0）。
    pub fn validate(&self) -> Result<(), AnalysisError> {
        ensure_fraction("変換効率", self.conversion_efficiency())?;
        if let Some(flux) = self.solar_flux {
            ensure_non_negative("太陽光の強さ", flux.value())?;
        }
        for (i, p) in self.panels.iter().enumerate() {
            p.validate()?;
            if self.panels[..i].iter().any(|q| q.id == p.id) {
                return Err(AnalysisError::InvalidInput(format!(
                    "太陽電池パネルの ID {} が重複しています",
                    p.id
                )));
            }
        }
        Ok(())
    }

    /// パネル → バスの変換効率（省略時 1）。
    pub fn conversion_efficiency(&self) -> f64 {
        self.conversion_efficiency.unwrap_or(1.0)
    }

    /// 太陽光の強さ。上書きがあればそれ、無ければ太陽までの距離 [m] から、距離も無ければ 1 AU の太陽定数。
    pub fn flux(&self, sun_distance_m: Option<f64>) -> Qty<Irradiance> {
        self.solar_flux.unwrap_or_else(|| {
            Qty::from_raw(sun_distance_m.map_or(SOLAR_CONSTANT, solar_flux_w_m2))
        })
    }

    /// パネルごとの発電（バス側）[W]。`sun_dir_body` は機体座標系の太陽方向（正規化して使う）、`sunlit` は日照率。
    pub fn panel_powers(
        &self,
        sun_dir_body: Vec3,
        sunlit: f64,
        flux: Qty<Irradiance>,
        life: SolarLife,
    ) -> Vec<Qty<Power>> {
        let Some(sun) = sun_dir_body.normalized() else {
            return vec![Qty::ZERO; self.panels.len()];
        };
        let conv = self.conversion_efficiency();
        self.panels
            .iter()
            .map(|p| {
                let normal = p.normal.normalized().unwrap_or(Vec3::ZERO);
                p.power(flux, solar_incidence(normal, sun, sunlit), life) * conv
            })
            .collect()
    }

    /// 時系列の 1 点でのパネルごとの発電（バス側）[W]。太陽方向・日照率・太陽距離はサンプルから取る。
    pub fn sample_powers(&self, sample: &TimelineSample, life: SolarLife) -> Vec<Qty<Power>> {
        let flux = self.flux(Some(sample.sun_distance_m));
        self.panel_powers(sample.sun_dir_body(), sample.sunlit, flux, life)
    }
}

/// 電力の合計。
pub fn total(powers: &[Qty<Power>]) -> Qty<Power> {
    powers.iter().fold(Qty::ZERO, |sum, &p| sum + p)
}

/// 時系列に沿った発電（軌道上の発電と電力収支で共有する）。値は SI の生の f64 [W]（時系列の内側）。
#[derive(Debug, Clone)]
pub struct SolarProfile {
    /// 元期からの経過 [s]
    pub t: Vec<f64>,
    /// 日照率
    pub sunlit: Vec<f64>,
    /// `panel_bol[i][k]`: パネル `i` のサンプル `k` での発電（BOL、バス側）[W]
    pub panel_bol: Vec<Vec<f64>>,
    eol_factors: Vec<f64>,
}

impl SolarProfile {
    /// 時系列のサンプルから作る。
    pub fn build(array: &SolarArray, samples: &[TimelineSample]) -> Self {
        let mut panel_bol = vec![Vec::with_capacity(samples.len()); array.panels.len()];
        for s in samples {
            for (i, p) in array.sample_powers(s, SolarLife::Bol).iter().enumerate() {
                panel_bol[i].push(p.value());
            }
        }
        Self {
            t: samples.iter().map(|s| s.t_s).collect(),
            sunlit: samples.iter().map(|s| s.sunlit).collect(),
            panel_bol,
            eol_factors: array.panels.iter().map(SolarPanel::eol).collect(),
        }
    }

    /// パネル `i` の発電の時系列 [W]。
    pub fn panel(&self, i: usize, life: SolarLife) -> Vec<f64> {
        let k = match life {
            SolarLife::Bol => 1.0,
            SolarLife::Eol => self.eol_factors[i],
        };
        self.panel_bol[i].iter().map(|p| p * k).collect()
    }

    /// 組全体の発電の時系列 [W]。
    pub fn total(&self, life: SolarLife) -> Vec<f64> {
        let mut sum = vec![0.0; self.t.len()];
        for i in 0..self.panel_bol.len() {
            for (a, p) in sum.iter_mut().zip(self.panel(i, life)) {
                *a += p;
            }
        }
        sum
    }

    /// 日照中（日照率 ≥ [`SUNLIT_THRESHOLD`]）かどうか。
    pub fn sunlit_flags(&self) -> Vec<bool> {
        self.sunlit.iter().map(|&s| s >= SUNLIT_THRESHOLD).collect()
    }
}

/// 日陰の区間（日照率が [`SUNLIT_THRESHOLD`] を下回る標本の連続区間。`sat_astro::timeline::shadow_windows`）。
pub fn eclipses(samples: &[TimelineSample]) -> Vec<Window> {
    shadow_windows(samples, SUNLIT_THRESHOLD)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel(id: &str, normal: Vec3) -> SolarPanel {
        SolarPanel {
            id: id.to_owned(),
            area: Qty::from_raw(1.0),
            cell_efficiency: 0.3,
            packing_factor: Some(1.0),
            eol_factor: Some(0.8),
            normal,
        }
    }

    fn array(panels: Vec<SolarPanel>) -> SolarArray {
        SolarArray {
            panels,
            solar_flux: None,
            conversion_efficiency: None,
        }
    }

    #[test]
    fn one_square_metre_normal_incidence() {
        // 1 m² × 30 % × 1361 W/m² = 408.3 W（BOL）、EOL は ×0.8 = 326.64 W
        let a = array(vec![panel("PZ", Vec3::Z)]);
        a.validate().unwrap();
        let flux = a.flux(None);
        let bol = a.panel_powers(Vec3::Z, 1.0, flux, SolarLife::Bol);
        assert!((bol[0].value() - 408.3).abs() < 1e-9);
        let eol = a.panel_powers(Vec3::Z, 1.0, flux, SolarLife::Eol);
        assert!((eol[0].value() - 326.64).abs() < 1e-9);
    }

    #[test]
    fn cosine_shadow_and_back_side() {
        // 60° 傾いた面は cos 60° = 0.5 倍、裏面は 0、半影 0.5 で半分
        let a = array(vec![
            panel("PX", Vec3::X),
            panel("MZ", Vec3::new(0.0, 0.0, -2.0)),
        ]);
        let sun = Vec3::new(60f64.to_radians().cos(), 0.0, 60f64.to_radians().sin());
        let p = a.panel_powers(sun, 1.0, Qty::from_raw(1361.0), SolarLife::Bol);
        assert!((p[0].value() - 204.15).abs() < 1e-9);
        assert_eq!(p[1].value(), 0.0);
        let half = a.panel_powers(Vec3::X, 0.5, Qty::from_raw(1361.0), SolarLife::Bol);
        assert!((total(&half).value() - 204.15).abs() < 1e-9);
    }

    #[test]
    fn conversion_and_flux_override() {
        let mut a = array(vec![panel("PZ", Vec3::Z)]);
        a.conversion_efficiency = Some(0.9);
        a.solar_flux = Some(Qty::from_raw(1322.0));
        let p = a.panel_powers(Vec3::Z, 1.0, a.flux(Some(1.0e11)), SolarLife::Bol);
        assert!((p[0].value() - 1322.0 * 0.3 * 0.9).abs() < 1e-9);
    }

    #[test]
    fn validation_rejects_bad_panels() {
        let mut p = panel("PZ", Vec3::Z);
        p.cell_efficiency = 1.5;
        assert!(array(vec![p]).validate().is_err());
        assert!(array(vec![panel("A", Vec3::ZERO)]).validate().is_err());
        assert!(
            array(vec![panel("A", Vec3::Z), panel("A", Vec3::X)])
                .validate()
                .is_err()
        );
    }
}
