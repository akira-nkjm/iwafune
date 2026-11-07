use crate::Rational;

const BASE_DIMENSIONS: usize = 9;

/// 7 SI 基底次元 + 補助次元 Angle / Information。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseDimension {
    Length,
    Mass,
    Time,
    ElectricCurrent,
    Temperature,
    AmountOfSubstance,
    LuminousIntensity,
    Angle,
    Information,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DimError {
    ExponentOverflow,
}

/// 基底次元の有理指数ベクトルで表した次元。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimension {
    exponents: [Rational; BASE_DIMENSIONS],
}

impl Dimension {
    pub const DIMENSIONLESS: Dimension = Dimension {
        exponents: [Rational::ZERO; BASE_DIMENSIONS],
    };

    pub const fn base(dim: BaseDimension) -> Dimension {
        let mut exponents = [Rational::ZERO; BASE_DIMENSIONS];
        exponents[dim as usize] = Rational::ONE;
        Dimension { exponents }
    }

    pub(crate) const fn from_exponents(exponents: [Rational; BASE_DIMENSIONS]) -> Dimension {
        Dimension { exponents }
    }

    pub const fn exponents(self) -> [Rational; BASE_DIMENSIONS] {
        self.exponents
    }

    pub fn is_compatible_with(self, other: Dimension) -> bool {
        self.exponents == other.exponents
    }

    pub fn try_mul(self, rhs: Dimension) -> Result<Dimension, DimError> {
        self.zip_exponents(rhs, Rational::checked_add)
    }

    pub fn try_div(self, rhs: Dimension) -> Result<Dimension, DimError> {
        self.zip_exponents(rhs, Rational::checked_sub)
    }

    pub fn powi(self, n: i8) -> Result<Dimension, DimError> {
        self.powr(Rational::from_int(n))
    }

    pub fn powr(self, r: Rational) -> Result<Dimension, DimError> {
        let mut exponents = [Rational::ZERO; BASE_DIMENSIONS];
        let mut index = 0;
        while index < BASE_DIMENSIONS {
            exponents[index] = self.exponents[index]
                .checked_mul(r)
                .ok_or(DimError::ExponentOverflow)?;
            index += 1;
        }
        Ok(Dimension { exponents })
    }

    fn zip_exponents(
        self,
        rhs: Dimension,
        op: fn(Rational, Rational) -> Option<Rational>,
    ) -> Result<Dimension, DimError> {
        let mut exponents = [Rational::ZERO; BASE_DIMENSIONS];
        let mut index = 0;
        while index < BASE_DIMENSIONS {
            exponents[index] = op(self.exponents[index], rhs.exponents[index])
                .ok_or(DimError::ExponentOverflow)?;
            index += 1;
        }
        Ok(Dimension { exponents })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensionless_is_compatible_with_itself() {
        assert!(Dimension::DIMENSIONLESS.is_compatible_with(Dimension::DIMENSIONLESS));
    }

    #[test]
    fn base_dimensions_differ() {
        let length = Dimension::base(BaseDimension::Length);
        let time = Dimension::base(BaseDimension::Time);
        assert!(!length.is_compatible_with(time));
    }

    #[test]
    fn velocity_times_time_is_length() {
        let length = Dimension::base(BaseDimension::Length);
        let time = Dimension::base(BaseDimension::Time);
        let velocity = length.try_div(time).unwrap();
        assert!(velocity.try_mul(time).unwrap().is_compatible_with(length));
    }

    #[test]
    fn length_squared_is_area() {
        let length = Dimension::base(BaseDimension::Length);
        let area = length.try_mul(length).unwrap();
        assert_eq!(area, length.powi(2).unwrap());
    }

    #[test]
    fn square_root_of_area_is_length() {
        let length = Dimension::base(BaseDimension::Length);
        let area = length.powi(2).unwrap();
        assert_eq!(area.powr(Rational::new(1, 2)), Ok(length));
    }

    #[test]
    fn incompatible_dimensions_are_not_compatible() {
        let length = Dimension::base(BaseDimension::Length);
        let mass = Dimension::base(BaseDimension::Mass);
        assert!(!length.is_compatible_with(mass));
    }

    #[test]
    fn exponent_overflow_is_reported() {
        let huge = Dimension::base(BaseDimension::Length).powi(127).unwrap();
        let result = huge.try_mul(Dimension::base(BaseDimension::Length));
        assert_eq!(result, Err(DimError::ExponentOverflow));
    }
}
