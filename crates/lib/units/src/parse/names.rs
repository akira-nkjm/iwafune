//! 次元名の語彙（型レベル次元の別名から作る）と、次元名の解析・比較。

use crate::dimensions::Dimension as TypedDimension;
use crate::{Dimension, UnitError};

/// 実行時の次元語彙を型レベル次元から生成する。名前は型名の `stringify!`、値は
/// 型が持つ [`crate::dimensions::Dimension::DIM`] なので、綴りも指数も二重管理にならない。
macro_rules! typed_dimensions {
    ($($name:ident),+ $(,)?) => {
        const DIMENSIONS: &[(&str, Dimension)] = &[
            $((
                stringify!($name),
                <crate::dimensions::$name as TypedDimension>::DIM,
            )),+
        ];
    };
}

typed_dimensions![
    Dimensionless,
    Length,
    Mass,
    Time,
    ElectricCurrent,
    Temperature,
    TemperatureDifference,
    AmountOfSubstance,
    LuminousIntensity,
    Angle,
    Information,
    Frequency,
    InverseSquareTime,
    Velocity,
    Acceleration,
    AngularVelocity,
    Force,
    Energy,
    Torque,
    MomentOfInertia,
    AngularMomentum,
    Power,
    Voltage,
    MagneticFluxDensity,
    MagneticDipoleMoment,
    Area,
    MassDensity,
    MassFlowRate,
    Pressure,
    GravitationalParameter,
    SquareRootTime,
    SquareRootFrequency,
    AccelerationNoiseDensity,
    AngularRandomWalk,
    ElectricCharge,
    Volume,
    ThermalConductance,
    ThermalConductivity,
    HeatTransferCoefficient,
    SpecificHeatCapacity,
    SpecificEnergy,
    Irradiance,
    DataRate,
    SolidAngle,
    LuminousFlux,
    Illuminance,
    Resistance,
    Capacitance,
    Inductance,
    Conductance,
    MagneticFlux,
    AbsorbedDose,
    AbsorbedDoseRate,
    ParticleFlux,
    ParticleFluence,
    Impulse,
];

/// 型レベルのエイリアス名と綴りが対応しない別名。既存データが使っているので受け続ける。
const DIMENSION_ALIASES: &[(&str, Dimension)] = &[
    ("one", Dimension::DIMENSIONLESS),
    // 比（表示の種類。次元は無次元）
    (crate::display_kind::FRACTION, Dimension::DIMENSIONLESS),
    (
        "current",
        <crate::dimensions::ElectricCurrent as TypedDimension>::DIM,
    ),
];

/// 名前で引ける次元を、宣言順に列挙する。
///
/// **表を外に持たせないための入口。** 次元の一覧が要る側（設計を読む画面など）が
/// 自前の表を持つと、`typed_dimensions!` に足したものが片方だけ古くなる。
/// 別名（[`DIMENSION_ALIASES`]）は含めない —— 同じ次元を 2 度数えることになるため。
#[must_use]
pub fn named_dimensions() -> &'static [(&'static str, Dimension)] {
    DIMENSIONS
}

/// 次元名を解析する。語彙は [`crate::typed`] のエイリアス集合（＋歴史的な別名）。
pub fn parse_dimension(name: &str) -> Result<Dimension, UnitError> {
    let name = name.trim();
    for (candidate, dimension) in DIMENSIONS.iter().chain(DIMENSION_ALIASES) {
        if dimension_name_eq(name, candidate) {
            return Ok(*dimension);
        }
    }
    Err(UnitError::UnknownDimension)
}

/// 次元名を綴りの流儀（区切り文字と大小）を無視して比較する。
pub(crate) fn dimension_name_eq(left: &str, right: &str) -> bool {
    let significant = |byte: &u8| !matches!(byte, b'_' | b'-' | b' ');
    let mut left = left.bytes().filter(significant);
    let mut right = right.bytes().filter(significant);
    loop {
        match (left.next(), right.next()) {
            (None, None) => return true,
            (Some(l), Some(r)) if l.eq_ignore_ascii_case(&r) => {}
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests;
