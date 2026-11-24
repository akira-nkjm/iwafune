//! 名前付きの次元の別名（`Length`・`Power` など）。次元の指数の組はここだけに書く。

use super::private::{Dim, Rat};
use typenum::{N1, N2, N3, P1, P2, P3, P4, Z0};

type Zero = Rat<Z0, P1>;
type One = Rat<P1, P1>;
type Two = Rat<P2, P1>;
type Three = Rat<P3, P1>;
type Four = Rat<P4, P1>;
type NegativeOne = Rat<N1, P1>;
type NegativeTwo = Rat<N2, P1>;
type NegativeThree = Rat<N3, P1>;
type Half = Rat<P1, P2>;
type NegativeHalf = Rat<N1, P2>;
type NegativeThreeHalves = Rat<N3, P2>;

// Each public name goes through one hidden alias so rustdoc keeps the readable name as the public
// declaration instead of expanding the typenum polyfill. Rust has no stable opaque type aliases;
// this extra indirection is the stable way to keep that implementation detail out of generated docs.
#[doc(hidden)]
pub type __Dimensionless = Dim<Zero, Zero, Zero, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Length = Dim<One, Zero, Zero, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Mass = Dim<Zero, One, Zero, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Time = Dim<Zero, Zero, One, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __ElectricCurrent = Dim<Zero, Zero, Zero, One, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Temperature = Dim<Zero, Zero, Zero, Zero, One, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __AmountOfSubstance = Dim<Zero, Zero, Zero, Zero, Zero, One, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __LuminousIntensity = Dim<Zero, Zero, Zero, Zero, Zero, Zero, One, Zero, Zero>;
#[doc(hidden)]
pub type __Angle = Dim<Zero, Zero, Zero, Zero, Zero, Zero, Zero, One, Zero>;
#[doc(hidden)]
pub type __Information = Dim<Zero, Zero, Zero, Zero, Zero, Zero, Zero, Zero, One>;
#[doc(hidden)]
pub type __Frequency = Dim<Zero, Zero, NegativeOne, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __InverseSquareTime = Dim<Zero, Zero, NegativeTwo, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Velocity = Dim<One, Zero, NegativeOne, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Acceleration = Dim<One, Zero, NegativeTwo, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __AngularVelocity = Dim<Zero, Zero, NegativeOne, Zero, Zero, Zero, Zero, One, Zero>;
#[doc(hidden)]
pub type __Force = Dim<One, One, NegativeTwo, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Energy = Dim<Two, One, NegativeTwo, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Torque = Dim<Two, One, NegativeTwo, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __MomentOfInertia = Dim<Two, One, Zero, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __AngularMomentum = Dim<Two, One, NegativeOne, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Power = Dim<Two, One, NegativeThree, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Voltage = Dim<Two, One, NegativeThree, NegativeOne, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __MagneticFluxDensity =
    Dim<Zero, One, NegativeTwo, NegativeOne, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __MagneticDipoleMoment = Dim<Two, Zero, Zero, One, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Area = Dim<Two, Zero, Zero, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __MassDensity = Dim<NegativeThree, One, Zero, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __MassFlowRate = Dim<Zero, One, NegativeOne, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Pressure = Dim<NegativeOne, One, NegativeTwo, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __GravitationalParameter =
    Dim<Three, Zero, NegativeTwo, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __SquareRootTime = Dim<Zero, Zero, Half, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __SquareRootFrequency = Dim<Zero, Zero, NegativeHalf, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __AccelerationNoiseDensity =
    Dim<One, Zero, NegativeThreeHalves, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __AngularRandomWalk = Dim<Zero, Zero, NegativeHalf, Zero, Zero, Zero, Zero, One, Zero>;
#[doc(hidden)]
pub type __ElectricCharge = Dim<Zero, Zero, One, One, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Volume = Dim<Three, Zero, Zero, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __ThermalConductance =
    Dim<Two, One, NegativeThree, Zero, NegativeOne, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __ThermalConductivity =
    Dim<One, One, NegativeThree, Zero, NegativeOne, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __HeatTransferCoefficient =
    Dim<Zero, One, NegativeThree, Zero, NegativeOne, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __SpecificHeatCapacity =
    Dim<Two, Zero, NegativeTwo, Zero, NegativeOne, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __SpecificEnergy = Dim<Two, Zero, NegativeTwo, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Irradiance = Dim<Zero, One, NegativeThree, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __DataRate = Dim<Zero, Zero, NegativeOne, Zero, Zero, Zero, Zero, Zero, One>;
#[doc(hidden)]
pub type __SolidAngle = Dim<Zero, Zero, Zero, Zero, Zero, Zero, Zero, Two, Zero>;
#[doc(hidden)]
pub type __LuminousFlux = Dim<Zero, Zero, Zero, Zero, Zero, Zero, One, Two, Zero>;
#[doc(hidden)]
pub type __Illuminance = Dim<NegativeTwo, Zero, Zero, Zero, Zero, Zero, One, Two, Zero>;
#[doc(hidden)]
pub type __Resistance = Dim<Two, One, NegativeThree, NegativeTwo, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Capacitance = Dim<NegativeTwo, NegativeOne, Four, Two, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Inductance = Dim<Two, One, NegativeTwo, NegativeTwo, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Conductance = Dim<NegativeTwo, NegativeOne, Three, Two, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __MagneticFlux = Dim<Two, One, NegativeTwo, NegativeOne, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __AbsorbedDose = Dim<Two, Zero, NegativeTwo, Zero, Zero, Zero, Zero, Zero, Zero>;
#[doc(hidden)]
pub type __Impulse = Dim<One, One, NegativeOne, Zero, Zero, Zero, Zero, Zero, Zero>;

/// Dimensionless quantity (for example `1`).
pub type Dimensionless = __Dimensionless;
/// Length dimension (`L`, for example `m`).
pub type Length = __Length;
/// Mass dimension (`M`, for example `kg`).
pub type Mass = __Mass;
/// Time dimension (`T`, for example `s`).
pub type Time = __Time;
/// Electric-current dimension (`I`, for example `A`).
pub type ElectricCurrent = __ElectricCurrent;
/// Thermodynamic-temperature dimension (for example `K`).
pub type Temperature = __Temperature;
/// Temperature-difference dimension (for example `K`).
///
/// **[`Temperature`] と次元が同一**（差は絶対温度と同じ SI の K）。同じ型のエイリアスなので取り違えはコンパイル時には弾けない。
/// 違いは表示（絶対温度は `degC`、差は 273.15 を引かず `K`）だけで、`Qty<TemperatureDifference>` の欄は
/// `TemperatureDifferenceKind::serialize` で種類を記録し、整形には `TemperatureDifferenceKind` を渡す。
pub type TemperatureDifference = __Temperature;
/// Amount-of-substance dimension (for example `mol`).
pub type AmountOfSubstance = __AmountOfSubstance;
/// Luminous-intensity dimension (for example `cd`).
pub type LuminousIntensity = __LuminousIntensity;
/// Angle dimension (for example `rad`). Angle remains distinct from dimensionless ratios.
pub type Angle = __Angle;
/// Information dimension (for example `bit`). Information remains distinct from pure ratios.
pub type Information = __Information;
/// Frequency dimension (`T^-1`, for example `Hz`).
pub type Frequency = __Frequency;
/// Inverse-square-time dimension (`T^-2`, for example `s^-2`).
pub type InverseSquareTime = __InverseSquareTime;
/// Velocity dimension (`L T^-1`, for example `m/s`).
pub type Velocity = __Velocity;
/// Acceleration dimension (`L T^-2`, for example `m/s^2`).
pub type Acceleration = __Acceleration;
/// Angular-velocity dimension (`Angle T^-1`, for example `rad/s`).
pub type AngularVelocity = __AngularVelocity;
/// Force dimension (`M L T^-2`, for example `N`).
pub type Force = __Force;
/// Energy dimension (`M L^2 T^-2`, for example `J`).
///
/// **[`Torque`] と次元が同一**（SI では rad は無次元なので `J` と `N*m` は同じ次元）。
/// 次元検査はこの 2 つを区別しない。使い分けは単位の綴り（`J` と `N*m`）で担保する。
pub type Energy = __Energy;
/// Torque dimension (`M L^2 T^-2`, for example `N*m`).
///
/// **[`Energy`] と次元が同一**。`Torque` と `Energy` は同じ型のエイリアスなので、
/// コンパイル時にも実行時にも取り違えは弾けない。読み手への意図表明として使い分ける。
pub type Torque = __Torque;
/// Moment-of-inertia dimension (`M L^2`, for example `kg*m^2`).
pub type MomentOfInertia = __MomentOfInertia;
/// Angular-momentum dimension (`M L^2 T^-1`, for example `N*m*s`).
pub type AngularMomentum = __AngularMomentum;
/// Power dimension (`M L^2 T^-3`, for example `W`).
pub type Power = __Power;
/// Electric-potential dimension (`M L^2 T^-3 I^-1`, for example `V`).
pub type Voltage = __Voltage;
/// Magnetic-flux-density dimension (`M T^-2 I^-1`).
pub type MagneticFluxDensity = __MagneticFluxDensity;
/// Magnetic-dipole-moment dimension (`I L^2`, for example `A*m^2`).
pub type MagneticDipoleMoment = __MagneticDipoleMoment;
/// Area dimension (`L^2`, for example `m^2`).
pub type Area = __Area;
/// Mass-density dimension (`M L^-3`, for example `kg/m^3`).
pub type MassDensity = __MassDensity;
/// Mass-flow-rate dimension (`M T^-1`, for example `kg/s`).
pub type MassFlowRate = __MassFlowRate;
/// Pressure dimension (`M L^-1 T^-2`, for example `Pa`).
pub type Pressure = __Pressure;
/// Standard gravitational parameter dimension (`L^3 T^-2`, for example `m^3/s^2`).
pub type GravitationalParameter = __GravitationalParameter;
/// Square root of time (`T^1/2`, for example `s^(1/2)`).
pub type SquareRootTime = __SquareRootTime;
/// Square root of frequency (`T^-1/2`, for example `Hz^(1/2)`).
pub type SquareRootFrequency = __SquareRootFrequency;
/// Acceleration noise density (`L T^-3/2`, for example `m/s^2/Hz^(1/2)`)。
///
/// 慣行の `µg/√Hz` の `g` は重力加速度だが、[`crate::parse_unit`] の `g` はグラム。
/// 綴るときは `ug0/Hz^(1/2)` と書く（`ug/Hz^(1/2)` は質量になり次元が合わない）。
pub type AccelerationNoiseDensity = __AccelerationNoiseDensity;
/// Angular random walk (`Angle T^-1/2`, for example `deg/h^(1/2)`)。
///
/// 慣行の `deg/√hr` は `√` を文法に持たないので `deg/h^(1/2)` と綴る。
pub type AngularRandomWalk = __AngularRandomWalk;
/// Electric-charge dimension (`T I`, for example `C`).
pub type ElectricCharge = __ElectricCharge;
/// Volume dimension (`L^3`, for example `m^3`).
pub type Volume = __Volume;
/// Thermal-conductance dimension (`M L^2 T^-3 K^-1`, for example `W/K`).
pub type ThermalConductance = __ThermalConductance;
/// Thermal-conductivity dimension (`L M T^-3 K^-1`, for example `W/(m*K)`; the conduction of a material).
pub type ThermalConductivity = __ThermalConductivity;
/// Heat-transfer-coefficient dimension (`M T^-3 K^-1`, for example `W/(m^2*K)`; the conductance per contact area).
pub type HeatTransferCoefficient = __HeatTransferCoefficient;
/// Specific-heat-capacity dimension (`L^2 T^-2 K^-1`, for example `J/(kg*K)`).
pub type SpecificHeatCapacity = __SpecificHeatCapacity;
/// Specific-energy dimension (`L^2 T^-2`, for example `J/kg` or `km^2/s^2`; the characteristic energy C3 of an interplanetary departure).
/// [`AbsorbedDose`] (Gy = J/kg) has the same exponents; the name lookup from a dimension gives `SpecificEnergy` (it comes first in the vocabulary).
pub type SpecificEnergy = __SpecificEnergy;
/// Irradiance dimension (`M T^-3`, for example `W/m^2`).
pub type Irradiance = __Irradiance;
/// Data-rate dimension (`Information T^-1`, for example `bit/s`).
pub type DataRate = __DataRate;
/// Solid-angle dimension (`Angle^2`, for example `sr`).
pub type SolidAngle = __SolidAngle;
/// Luminous-flux dimension (`J Angle^2`, for example `lm`).
pub type LuminousFlux = __LuminousFlux;
/// Illuminance dimension (`J Angle^2 L^-2`, for example `lx`).
pub type Illuminance = __Illuminance;
/// Electrical-resistance dimension (`M L^2 T^-3 I^-2`, for example `ohm`).
pub type Resistance = __Resistance;
/// Capacitance dimension (`M^-1 L^-2 T^4 I^2`, for example `F`).
pub type Capacitance = __Capacitance;
/// Inductance dimension (`M L^2 T^-2 I^-2`, for example `H`).
pub type Inductance = __Inductance;
/// Electrical-conductance dimension (`M^-1 L^-2 T^3 I^2`, for example `S`).
pub type Conductance = __Conductance;
/// Magnetic-flux dimension (`M L^2 T^-2 I^-1`, for example `Wb`).
pub type MagneticFlux = __MagneticFlux;
/// Absorbed-dose dimension (`L^2 T^-2`, for example `Gy`).
///
/// `Gy`（吸収線量）と `Sv`（線量当量）はどちらも `J/kg` で、速度の 2 乗・比エネルギーとも
/// 次元が同一。次元検査はこれらを区別しない。線量当量用の別名は置かず、使い分けは単位の
/// 綴りで担保する。
pub type AbsorbedDose = __AbsorbedDose;
/// Impulse (linear momentum) dimension (`M L T^-1`, for example `N*s`)。推進の総インパルス・最小インパルスビットに使う。
pub type Impulse = __Impulse;
