//! `orbit.trapped_dose` の入力・出力の型。

use crate::orbit::total_dose::DoseDepthPoint;
use sat_astro::radiation::{SolarCondition, Species};
use sat_astro::timeline::OrbitSampling;
use sat_units::typed::{AbsorbedDoseRate, Length, Qty, Time};
use serde::{Deserialize, Serialize};

/// 線量を求める幾何。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DoseGeometry {
    /// アルミの中実の球の中心（全方向から入射。既定）
    SphereCentre,
    /// 半無限のアルミの媒質の中（片面から入射）
    SemiInfiniteSlab,
    /// 有限のアルミの板の透過面（片面から入射）
    FiniteSlab,
}

/// 陽子の核相互作用の扱い。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum NuclearAttenuation {
    /// 見ない（SHIELDOSE-2 の `INUC` 1。既定）
    None,
    /// 見て、荷電の二次粒子のエネルギーはその場に付与（`INUC` 2）
    Local,
    /// 2 に、中性子のエネルギーの指数分布の寄与を足す（`INUC` 3）
    LocalWithNeutrons,
}

/// 値の出どころ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DoseSettingSource {
    /// 入力で与えられた
    Given,
    /// この解析の既定
    Default,
}

/// 解析の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TrappedDoseInput {
    /// 軌道のサンプルの取り方（1 日分。`spec::daily_sampling`）
    pub sampling: OrbitSampling,
    /// 遮蔽の厚さ（等価アルミ。増える順。無ければ [`DEFAULT_THICKNESSES_MM`]）
    #[serde(default)]
    pub thicknesses: Option<Vec<Qty<Length>>>,
    /// 幾何（無ければ [`DoseGeometry::SphereCentre`]）
    #[serde(default)]
    pub geometry: Option<DoseGeometry>,
    /// 陽子の核相互作用（無ければ [`NuclearAttenuation::None`]）
    #[serde(default)]
    pub nuclear: Option<NuclearAttenuation>,
}

/// 表の点 1 つ（遮蔽の厚さ → 線量率）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TrappedDosePoint {
    /// 等価アルミの遮蔽厚さ
    pub shielding: Qty<Length>,
    pub protons: Qty<AbsorbedDoseRate>,
    pub electrons: Qty<AbsorbedDoseRate>,
    /// 電子の制動放射
    pub bremsstrahlung: Qty<AbsorbedDoseRate>,
    /// 3 つの合計
    pub total: Qty<AbsorbedDoseRate>,
}

/// 太陽活動 1 つ（極小か極大）の結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TrappedDoseCase {
    pub solar: SolarCondition,
    /// 使ったマップ（陽子・電子）
    pub proton_model: String,
    pub electron_model: String,
    /// 厚さごとの線量率
    pub table: Vec<TrappedDosePoint>,
}

/// 粒子 1 種のスペクトルの使い方。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TrappedDoseSpectrumUsed {
    pub species: Species,
    pub solar: SolarCondition,
    /// 積分フラックスを引いたエネルギーの範囲 [MeV]
    pub energy_min_mev: f64,
    pub energy_max_mev: f64,
    /// 積分フラックスを引いたエネルギーの数
    pub integral_points: usize,
    /// 微分フラックスの点の数（粒子が無ければ 0）
    pub differential_points: usize,
}

/// 使った条件。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TrappedDoseUsed {
    pub span: Qty<Time>,
    pub step: Qty<Time>,
    pub samples: usize,
    /// L が求まらずマップの外としたサンプルの数（磁場ごとの最大）
    pub samples_without_l: usize,
    pub geometry: DoseGeometry,
    pub geometry_source: DoseSettingSource,
    /// 線量を測る物質
    pub detector: String,
    pub nuclear: NuclearAttenuation,
    pub nuclear_source: DoseSettingSource,
    pub thickness_source: DoseSettingSource,
    /// SHIELDOSE-2 の積分の格子点の数
    pub integration_points: usize,
    pub spectra: Vec<TrappedDoseSpectrumUsed>,
    /// 何の線量か（含むもの・含まないもの）
    pub scope: String,
}

/// 解析の出力。判定は付けない（部品への当てはめは `orbit.total_dose`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TrappedDoseOutput {
    /// 太陽活動の極小・極大の順
    pub cases: Vec<TrappedDoseCase>,
    pub used: TrappedDoseUsed,
}

impl TrappedDoseOutput {
    /// 厚さごとに、極小と極大の合計の大きい方を取った表（`orbit.total_dose` の環境の表の形）。
    /// 陽子は太陽活動の極小で、電子は極大で多くなる（LEO）ので、片方に決めずに**悪い方**を取る（保守側）。
    /// 合計が 0 以下の厚さから先は含めない（対数で補間する表は正の値だけ）。
    pub fn worst_case_table(&self) -> Vec<DoseDepthPoint> {
        let Some(first) = self.cases.first() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (i, p) in first.table.iter().enumerate() {
            let rate = self
                .cases
                .iter()
                .filter_map(|c| c.table.get(i))
                .map(|p| p.total.value())
                .fold(0.0, f64::max);
            if rate <= 0.0 {
                break;
            }
            out.push(DoseDepthPoint {
                shielding: p.shielding,
                rate: Qty::from_raw(rate),
            });
        }
        out
    }
}
