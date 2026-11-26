//! 4 つの束縛粒子の地図と、それぞれが組にする地磁気モデル。

/// 粒子の種類。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Species {
    /// 陽子（AP-8）
    Proton,
    /// 電子（AE-8）
    Electron,
}

/// 太陽活動の条件（地図の元期）。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SolarCondition {
    /// 太陽活動極小（元期 1964）
    Minimum,
    /// 太陽活動極大（元期 1970）
    Maximum,
}

/// 地図が前提にする地磁気モデル。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BeltField {
    /// Jensen & Cain (1962) の 1960 年の係数（AP-8 極小・AE-8 極小・AE-8 極大）
    JensenCain1960,
    /// GSFC 12/66 の係数を 1970 年へ外挿した場（AP-8 極大）
    Gsfc1266To1970,
}

impl BeltField {
    /// 2 つの場。
    pub const ALL: [BeltField; 2] = [BeltField::JensenCain1960, BeltField::Gsfc1266To1970];
}

/// 束縛粒子の地図。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TrappedModel {
    /// AP-8 MIN（陽子、太陽活動極小）
    Ap8Min,
    /// AP-8 MAX（陽子、太陽活動極大）
    Ap8Max,
    /// AE-8 MIN（電子、太陽活動極小）
    Ae8Min,
    /// AE-8 MAX（電子、太陽活動極大）
    Ae8Max,
}

impl TrappedModel {
    /// 4 つの地図。
    pub const ALL: [TrappedModel; 4] = [
        TrappedModel::Ap8Min,
        TrappedModel::Ap8Max,
        TrappedModel::Ae8Min,
        TrappedModel::Ae8Max,
    ];

    /// 粒子の種類。
    pub fn species(self) -> Species {
        match self {
            Self::Ap8Min | Self::Ap8Max => Species::Proton,
            Self::Ae8Min | Self::Ae8Max => Species::Electron,
        }
    }

    /// 太陽活動の条件。
    pub fn solar(self) -> SolarCondition {
        match self {
            Self::Ap8Min | Self::Ae8Min => SolarCondition::Minimum,
            Self::Ap8Max | Self::Ae8Max => SolarCondition::Maximum,
        }
    }

    /// 粒子と太陽活動から地図を選ぶ。
    pub fn of(species: Species, solar: SolarCondition) -> Self {
        match (species, solar) {
            (Species::Proton, SolarCondition::Minimum) => Self::Ap8Min,
            (Species::Proton, SolarCondition::Maximum) => Self::Ap8Max,
            (Species::Electron, SolarCondition::Minimum) => Self::Ae8Min,
            (Species::Electron, SolarCondition::Maximum) => Self::Ae8Max,
        }
    }

    /// 組にする地磁気モデル。SPENVIS・IRBEM の AP-8/AE-8 と同じ対応で、AP-8 MAX だけが GSFC 12/66（1970 年へ外挿）。
    pub fn field(self) -> BeltField {
        match self {
            Self::Ap8Max => BeltField::Gsfc1266To1970,
            _ => BeltField::JensenCain1960,
        }
    }

    /// 地図の名前（表示と配布ファイルの名前の元。`ap8min` など）。
    pub fn id(self) -> &'static str {
        match self {
            Self::Ap8Min => "ap8min",
            Self::Ap8Max => "ap8max",
            Self::Ae8Min => "ae8min",
            Self::Ae8Max => "ae8max",
        }
    }
}
