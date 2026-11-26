//! 遮蔽されたアルミの深さでの全吸収線量: SHIELDOSE-2（S. M. Seltzer, NIST, NISTIR 5477, 1994。初版は NBS TN 1116, 1980）の手順。
//! **捕捉粒子のスペクトルから遮蔽の深さごとの線量への換算はここだけ。**
//!
//! 入力は全方向の微分フラックス（[`GridSpectrum`]）、出力は 3 つの幾何（半無限の媒質・有限の板・球の中心）での、陽子・電子・電子の制動放射の
//! 線量 [Gy/s]。データベース（陽子 PROTBAS2・電子と制動放射 ELBRBAS2）は配布データ（`shieldose2.dat`）で、[`ShieldingData`] が読む。
//!
//! | サブモジュール | 責務 |
//! |---|---|
//! | `data` | データベースの読み取り（[`ShieldingData`]。形式と個数を厳密に確かめる） |
//! | `spectrum` | 対数格子・スペクトルの補間・シンプソン積分・積分フラックスから微分フラックスへ |
//! | `proton`・`electron` | 入射エネルギー 1 つの、深さごとの線量の核（陽子・電子と制動放射） |
//! | `dose` | 核とスペクトルの積分（[`ShieldingModel`]）、球の中心の線量 |
//!
//! 制限: 遮蔽はアルミの 3 つの単純な幾何だけ（実機の形・部品の配置は見ない）。陽子の核相互作用は近似（選択 2・3 は検証が少ない）。
//! 外挿・補間の扱いは SHIELDOSE-2 と同じ。

mod data;
mod dose;
mod electron;
mod proton;
pub mod spectrum;

pub use data::{DETECTOR_COUNT, ShieldingData};
pub use dose::{
    AL_DENSITY_G_CM3, ElectronDoses, GY_PER_MEV_PER_G, GeometryDoses, MIN_DEPTH_G_CM2,
    ShieldingModel, aluminium_depth_g_cm2,
};
pub use spectrum::{EnergyGrid, GridSpectrum, differential_from_integral};

/// 検出器（線量を測る物質）。SHIELDOSE-2 の `IDET`（1 始まり）の順。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Detector {
    Aluminum,
    Graphite,
    Silicon,
    Air,
    Bone,
    CalciumFluoride,
    GalliumArsenide,
    LithiumFluoride,
    SiliconDioxide,
    Tissue,
    Water,
}

impl Detector {
    /// 全部（データベースの順）。
    pub const ALL: [Detector; DETECTOR_COUNT] = [
        Detector::Aluminum,
        Detector::Graphite,
        Detector::Silicon,
        Detector::Air,
        Detector::Bone,
        Detector::CalciumFluoride,
        Detector::GalliumArsenide,
        Detector::LithiumFluoride,
        Detector::SiliconDioxide,
        Detector::Tissue,
        Detector::Water,
    ];

    /// データベースでの番号（0 始まり）。
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&d| d == self).unwrap_or(0)
    }

    /// 英語の名前（出力に出す）。
    pub fn name(self) -> &'static str {
        match self {
            Self::Aluminum => "aluminium",
            Self::Graphite => "graphite",
            Self::Silicon => "silicon",
            Self::Air => "air",
            Self::Bone => "bone",
            Self::CalciumFluoride => "CaF2",
            Self::GalliumArsenide => "GaAs",
            Self::LithiumFluoride => "LiF",
            Self::SiliconDioxide => "SiO2",
            Self::Tissue => "tissue",
            Self::Water => "water",
        }
    }
}

/// 陽子の核相互作用の扱い（SHIELDOSE-2 の `INUC`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NuclearOption {
    /// 1: 核相互作用による減衰を見ない（保守側の既定）
    None,
    /// 2: 核相互作用の減衰を見て、荷電の二次粒子のエネルギーはその場に付与
    Local,
    /// 3: 2 に加えて、中性子のエネルギーの指数分布の近似的な寄与
    LocalWithNeutrons,
}

#[cfg(test)]
mod tests;
