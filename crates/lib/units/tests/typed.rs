use sat_units::typed::{
    Acceleration, AccelerationNoiseDensity, AmountOfSubstance, Angle, AngularMomentum,
    AngularRandomWalk, AngularVelocity, Area, DataRate, Dimensionless, ElectricCurrent, Energy,
    Force, Frequency, GravitationalParameter, Information, InverseSquareTime, Length,
    LuminousIntensity, MagneticDipoleMoment, MagneticFluxDensity, Mass, MassDensity, MassFlowRate,
    MomentOfInertia, Power, Pressure, Qty, Scalar, SquareRootFrequency, SquareRootTime,
    Temperature, Time, Torque, Velocity, Voltage,
};
use sat_units::{BaseDimension, Dimension, Rational};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fixed(i32);

impl Scalar for Fixed {}

impl core::ops::Div for Fixed {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 / rhs.0)
    }
}

fn assert_close(left: f64, right: f64) {
    assert!((left - right).abs() < f64::EPSILON, "{left} != {right}");
}

#[test]
fn same_dimension_values_add_and_subtract() {
    let left: Qty<Length> = Qty::from_raw(7.5);
    let right: Qty<Length> = Qty::from_raw(2.5);

    assert_close((left + right).value(), 10.0);
    assert_close((left - right).value(), 5.0);
}

#[test]
fn multiplication_and_division_compute_dimensions() {
    let length: Qty<Length> = Qty::from_raw(12.0);
    let time: Qty<Time> = Qty::from_raw(3.0);
    let velocity: Qty<Velocity> = length / time;
    let acceleration: Qty<Acceleration> = velocity / time;
    let mass: Qty<Mass> = Qty::from_raw(2.0);
    let force: Qty<Force> = mass * acceleration;
    let recovered_length: Qty<Length> = velocity * time;

    assert_close(velocity.value(), 4.0);
    assert_close(acceleration.value(), 4.0 / 3.0);
    assert_close(force.value(), 8.0 / 3.0);
    assert_close(recovered_length.value(), 12.0);
}

#[test]
fn information_divided_by_time_is_data_rate() {
    let information: Qty<Information> = Qty::from_raw(8_000.0);
    let time: Qty<Time> = Qty::from_raw(2.0);
    let rate: Qty<DataRate> = information / time;

    assert_close(rate.value(), 4_000.0);
}

#[test]
fn scalar_negation_and_comparison_preserve_dimensions() {
    let length: Qty<Length> = Qty::from_raw(4.0);
    let doubled: Qty<Length> = length * 2.0;
    let doubled_from_left: Qty<Length> = 2.0 * length;
    let negative: Qty<Length> = -length;
    let halved: Qty<Length> = length / 2.0;

    assert_eq!(doubled, doubled_from_left);
    assert!(doubled > length);
    assert_close(negative.value(), -4.0);
    assert_close(halved.value(), 2.0);
    assert_eq!(
        core::mem::size_of::<Qty<Length>>(),
        core::mem::size_of::<f64>()
    );
}

#[test]
fn non_default_scalar_type_is_supported() {
    let length: Qty<Length, i32> = Qty::from_raw(6);
    let time: Qty<Time, i32> = Qty::from_raw(2);
    let velocity: Qty<Velocity, i32> = length / time;
    let scaled: Qty<Velocity, i32> = velocity * 3;

    assert_eq!(scaled.value(), 9);
}

#[test]
fn downstream_scalar_type_is_supported() {
    let length: Qty<Length, Fixed> = Qty::from_raw(Fixed(12));
    let time: Qty<Time, Fixed> = Qty::from_raw(Fixed(3));
    let velocity: Qty<Velocity, Fixed> = length / time;

    assert_eq!(velocity.value(), Fixed(4));
}

#[test]
fn rational_exponents_participate_in_dimension_algebra() {
    let acceleration: Qty<Acceleration> = Qty::from_raw(9.806_65);
    let root_hertz: Qty<SquareRootFrequency> = Qty::from_raw(2.0);
    let noise: Qty<AccelerationNoiseDensity> = acceleration / root_hertz;
    let angle: Qty<Angle> = Qty::from_raw(0.3);
    let root_hour: Qty<SquareRootTime> = Qty::from_raw(2.0);
    let random_walk: Qty<AngularRandomWalk> = angle / root_hour;
    let normalized: Qty<Dimensionless> = root_hour * root_hertz;

    assert_close(noise.value(), 9.806_65 / 2.0);
    assert_close(random_walk.value(), 0.15);
    assert_close(normalized.value(), 4.0);
    assert_eq!(
        Qty::<SquareRootTime>::DIM.exponents()[2],
        Rational::new(1, 2)
    );
    assert_eq!(
        Qty::<SquareRootFrequency>::DIM.exponents()[2],
        Rational::new(-1, 2)
    );
    assert_eq!(
        Qty::<AccelerationNoiseDensity>::DIM.exponents()[2],
        Rational::new(-3, 2)
    );
    assert_eq!(
        Qty::<AngularRandomWalk>::DIM.exponents(),
        [
            Rational::ZERO,
            Rational::ZERO,
            Rational::new(-1, 2),
            Rational::ZERO,
            Rational::ZERO,
            Rational::ZERO,
            Rational::ZERO,
            Rational::ONE,
            Rational::ZERO,
        ]
    );
}

#[test]
fn named_type_dimensions_match_runtime_dimensions() {
    let length = Dimension::base(BaseDimension::Length);
    let mass = Dimension::base(BaseDimension::Mass);
    let time = Dimension::base(BaseDimension::Time);
    let current = Dimension::base(BaseDimension::ElectricCurrent);
    let temperature = Dimension::base(BaseDimension::Temperature);
    let amount = Dimension::base(BaseDimension::AmountOfSubstance);
    let luminosity = Dimension::base(BaseDimension::LuminousIntensity);
    let angle = Dimension::base(BaseDimension::Angle);
    let frequency = Dimension::DIMENSIONLESS.try_div(time).unwrap();
    let inverse_square_time = frequency.try_div(time).unwrap();
    let velocity = length.try_div(time).unwrap();
    let acceleration = velocity.try_div(time).unwrap();
    let angular_velocity = angle.try_div(time).unwrap();
    let force = mass.try_mul(acceleration).unwrap();
    let energy = force.try_mul(length).unwrap();
    let torque = force.try_mul(length).unwrap();
    let area = length.powi(2).unwrap();
    let moment_of_inertia = mass.try_mul(area).unwrap();
    let angular_momentum = torque.try_mul(time).unwrap();
    let power = energy.try_div(time).unwrap();
    let voltage = power.try_div(current).unwrap();
    let magnetic_flux_density = mass
        .try_div(time.powi(2).unwrap())
        .unwrap()
        .try_div(current)
        .unwrap();
    let magnetic_dipole_moment = current.try_mul(area).unwrap();
    let mass_density = mass.try_div(length.powi(3).unwrap()).unwrap();
    let mass_flow_rate = mass.try_div(time).unwrap();
    let pressure = force.try_div(area).unwrap();
    let gravitational_parameter = length
        .powi(3)
        .unwrap()
        .try_div(time.powi(2).unwrap())
        .unwrap();

    assert_eq!(Qty::<Dimensionless>::DIM, Dimension::DIMENSIONLESS);
    assert_eq!(Qty::<Length>::DIM, length);
    assert_eq!(Qty::<Mass>::DIM, mass);
    assert_eq!(Qty::<Time>::DIM, time);
    assert_eq!(Qty::<ElectricCurrent>::DIM, current);
    assert_eq!(Qty::<Temperature>::DIM, temperature);
    assert_eq!(Qty::<AmountOfSubstance>::DIM, amount);
    assert_eq!(Qty::<LuminousIntensity>::DIM, luminosity);
    assert_eq!(Qty::<Angle>::DIM, angle);
    assert_eq!(Qty::<Frequency>::DIM, frequency);
    assert_eq!(Qty::<InverseSquareTime>::DIM, inverse_square_time);
    assert_eq!(Qty::<Velocity>::DIM, velocity);
    assert_eq!(Qty::<Acceleration>::DIM, acceleration);
    assert_eq!(Qty::<AngularVelocity>::DIM, angular_velocity);
    assert_eq!(Qty::<Force>::DIM, force);
    assert_eq!(Qty::<Energy>::DIM, energy);
    assert_eq!(Qty::<Torque>::DIM, torque);
    assert_eq!(Qty::<MomentOfInertia>::DIM, moment_of_inertia);
    assert_eq!(Qty::<AngularMomentum>::DIM, angular_momentum);
    assert_eq!(Qty::<Power>::DIM, power);
    assert_eq!(Qty::<Voltage>::DIM, voltage);
    assert_eq!(Qty::<MagneticFluxDensity>::DIM, magnetic_flux_density);
    assert_eq!(Qty::<MagneticDipoleMoment>::DIM, magnetic_dipole_moment);
    assert_eq!(Qty::<Area>::DIM, area);
    assert_eq!(Qty::<MassDensity>::DIM, mass_density);
    assert_eq!(Qty::<MassFlowRate>::DIM, mass_flow_rate);
    assert_eq!(Qty::<Pressure>::DIM, pressure);
    assert_eq!(Qty::<GravitationalParameter>::DIM, gravitational_parameter);
}

#[test]
fn common_typed_constants_and_finite_check_stay_in_typed_domain() {
    assert_eq!(Qty::<Length>::ZERO, Qty::from_raw(0.0));
    assert_eq!(Qty::<Dimensionless>::ONE, Qty::from_raw(1.0));
    assert!(Qty::<Power>::from_raw(3.0).is_finite());
    assert!(!Qty::<Power>::from_raw(f64::INFINITY).is_finite());
}
