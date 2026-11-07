//! 単位文字列の解析と SI 換算の窓口（**app 側の単位の唯一の入口**）。中身はすべて sat-units（単位の唯一の正本）に任せる。
//! projection が設定値を SI にするときは [`Quantity::si_as`]、次元の表示名は [`dimension_label`] / [`unit_dimension`] を使う。

use super::{Quantity, ValueError};
use sat_units::{Dimension, Unit, parse_dimension, parse_unit};

/// 解析済みの単位と、その次元の名前。
#[derive(Debug, Clone, Copy)]
pub struct UnitInfo {
    pub unit: Unit,
    pub dimension: Dimension,
}

/// 単位文字列を解析する（空文字列は無次元）。
pub fn unit_info(unit: &str) -> Result<UnitInfo, ValueError> {
    let parsed = parse_unit(unit).map_err(|e| ValueError::Unit {
        unit: unit.to_string(),
        reason: e.to_string(),
    })?;
    Ok(UnitInfo {
        unit: parsed,
        dimension: parsed.dimension(),
    })
}

/// 次元名（`Mass`・`angular_momentum` など）を解析する。
pub fn parse_dimension_name(name: &str) -> Result<Dimension, ValueError> {
    parse_dimension(name).map_err(|_| ValueError::UnknownDimension(name.to_string()))
}

/// 次元の表示名（名前の付いた次元ならその名前、無ければ SI の基本単位の式 `kg/m`）。sat-units のものをそのまま使う。
pub use sat_units::format::dimension_label;

/// 単位文字列の次元の表示名。
pub fn unit_dimension(unit: &str) -> Result<String, ValueError> {
    Ok(dimension_label(unit_info(unit)?.dimension))
}

/// 単位が期待する次元（次元名）を持つか検査し、解析済みの単位を返す。
pub fn expect_dimension(unit: &str, expected: &str) -> Result<Unit, ValueError> {
    let info = unit_info(unit)?;
    let want = parse_dimension_name(expected)?;
    if info.dimension.is_compatible_with(want) {
        Ok(info.unit)
    } else {
        Err(ValueError::Dimension {
            unit: unit.to_string(),
            expected: expected.to_string(),
            found: dimension_label(info.dimension),
        })
    }
}

/// 単位が別の単位と同じ次元か検査する（許可単位どうしの一致など）。
pub fn same_dimension(unit: &str, reference: &str) -> Result<bool, ValueError> {
    Ok(unit_info(unit)?
        .dimension
        .is_compatible_with(unit_info(reference)?.dimension))
}

/// 数値と単位から SI 値を求める（affine・対数単位も sat-units の規則で換算する）。
pub fn to_si(value: f64, unit: &str) -> Result<f64, ValueError> {
    Ok(unit_info(unit)?.unit.to_si(value))
}

impl Quantity {
    /// SI 値（次元は検査しない）。
    pub fn si(&self) -> Result<f64, ValueError> {
        to_si(self.value, &self.unit)
    }

    /// 次元を検査して SI 値を返す（projection が設定値を読むときの入口）。
    pub fn si_as(&self, dimension: &str) -> Result<f64, ValueError> {
        Ok(expect_dimension(&self.unit, dimension)?.to_si(self.value))
    }

    /// 次元の表示名。
    pub fn dimension(&self) -> Result<String, ValueError> {
        unit_dimension(&self.unit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realistic_units_parse_with_the_right_dimension() {
        for (unit, dim) in [
            ("g", "Mass"),
            ("mN*m*s", "AngularMomentum"),
            ("mN*m", "Torque"),
            ("kbit/s", "DataRate"),
            ("Mbit/s", "DataRate"),
            ("GB", "Information"),
            ("Gbit", "Information"),
            ("A*m^2", "MagneticDipoleMoment"),
            ("J/(kg*K)", "SpecificHeatCapacity"),
            ("cm^2", "Area"),
            ("degC", "Temperature"),
            ("dB", "Dimensionless"),
            ("dBi", "Dimensionless"),
            ("deg", "Angle"),
            ("MHz", "Frequency"),
            ("Wh", "Energy"),
            ("g0", "Acceleration"),
            ("GPa", "Pressure"),
            ("kg/m^3", "MassDensity"),
            ("W/K", "ThermalConductance"),
            ("W/m^2", "Irradiance"),
            ("d", "Time"),
        ] {
            assert!(expect_dimension(unit, dim).is_ok(), "{unit} as {dim}");
        }
    }

    #[test]
    fn dimension_mismatch_is_reported() {
        let err = expect_dimension("W", "Mass");
        assert!(matches!(err, Err(ValueError::Dimension { .. })));
    }

    #[test]
    fn si_conversion_uses_sat_units() {
        let q = Quantity::new(620.0, "g");
        assert!((q.si_as("Mass").unwrap_or(0.0) - 0.62).abs() < 1e-12);
        let t = Quantity::new(20.0, "degC");
        assert!((t.si().unwrap_or(0.0) - 293.15).abs() < 1e-9);
    }

    #[test]
    fn unnamed_dimension_uses_the_si_expression() {
        let name = unit_dimension("g/m").unwrap_or_default();
        assert_eq!(name, "kg/m");
    }
}
