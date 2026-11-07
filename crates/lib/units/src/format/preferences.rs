//! 次元ごとの表示単位の並び（表示の好みだけ。換算は `crate::parse_unit` が行う）。

/// 次元ごとの表示単位の並び。`(単位, しきい値)` は小さい順。`base` は 0・非有限のときの単位。
pub(super) struct Preference {
    pub(super) name: &'static str,
    pub(super) base: &'static str,
    pub(super) units: &'static [(&'static str, f64)],
}

const fn pref(
    name: &'static str,
    base: &'static str,
    units: &'static [(&'static str, f64)],
) -> Preference {
    Preference { name, base, units }
}

pub(super) const PREFERENCES: &[Preference] = &[
    pref("Dimensionless", "", &[("", 1.0)]),
    pref(crate::display_kind::FRACTION, "%", &[("%", 1.0)]),
    pref(
        "Length",
        "m",
        &[
            ("nm", 1.0),
            ("um", 1.0),
            ("mm", 1.0),
            ("m", 1.0),
            ("km", 1.0),
        ],
    ),
    pref(
        "Mass",
        "kg",
        &[("ug", 1.0), ("mg", 1.0), ("g", 1.0), ("kg", 1.0)],
    ),
    pref(
        "Time",
        "s",
        &[
            ("ns", 1.0),
            ("us", 1.0),
            ("ms", 1.0),
            ("s", 1.0),
            ("min", 2.0),
            ("h", 2.0),
            ("d", 2.0),
        ],
    ),
    pref(
        "ElectricCurrent",
        "A",
        &[
            ("nA", 1.0),
            ("uA", 1.0),
            ("mA", 1.0),
            ("A", 1.0),
            ("kA", 1.0),
        ],
    ),
    pref("Temperature", "degC", &[("degC", 1.0)]),
    pref("Angle", "deg", &[("deg", 1.0)]),
    pref(
        "Information",
        "B",
        &[("B", 1.0), ("kB", 1.0), ("MB", 1.0), ("GB", 1.0)],
    ),
    pref(
        "Frequency",
        "Hz",
        &[
            ("mHz", 1.0),
            ("Hz", 1.0),
            ("kHz", 1.0),
            ("MHz", 1.0),
            ("GHz", 1.0),
        ],
    ),
    pref(
        "Velocity",
        "m/s",
        &[("mm/s", 1.0), ("m/s", 1.0), ("km/s", 1.0)],
    ),
    pref(
        "Acceleration",
        "m/s^2",
        &[("um/s^2", 1.0), ("mm/s^2", 1.0), ("m/s^2", 1.0)],
    ),
    pref("AngularVelocity", "deg/s", &[("deg/s", 1.0)]),
    pref(
        "Force",
        "N",
        &[("uN", 1.0), ("mN", 1.0), ("N", 1.0), ("kN", 1.0)],
    ),
    pref(
        "Energy",
        "Wh",
        &[("mWh", 1.0), ("Wh", 1.0), ("kWh", 1.0), ("MWh", 1.0)],
    ),
    pref(
        "Torque",
        "N*m",
        &[("nN*m", 1.0), ("uN*m", 1.0), ("mN*m", 1.0), ("N*m", 1.0)],
    ),
    pref("MomentOfInertia", "kg*m^2", &[("kg*m^2", 1.0)]),
    pref(
        "AngularMomentum",
        "N*m*s",
        &[("uN*m*s", 1.0), ("mN*m*s", 1.0), ("N*m*s", 1.0)],
    ),
    pref(
        "Power",
        "W",
        &[
            ("nW", 1.0),
            ("uW", 1.0),
            ("mW", 1.0),
            ("W", 1.0),
            ("kW", 1.0),
            ("MW", 1.0),
        ],
    ),
    pref(
        "Voltage",
        "V",
        &[("uV", 1.0), ("mV", 1.0), ("V", 1.0), ("kV", 1.0)],
    ),
    pref(
        "MagneticFluxDensity",
        "T",
        &[("nT", 1.0), ("uT", 1.0), ("mT", 1.0), ("T", 1.0)],
    ),
    pref(
        "MagneticDipoleMoment",
        "A*m^2",
        &[("mA*m^2", 1.0), ("A*m^2", 1.0)],
    ),
    pref("Area", "m^2", &[("mm^2", 1.0), ("cm^2", 1.0), ("m^2", 1.0)]),
    pref("MassDensity", "kg/m^3", &[("kg/m^3", 1.0)]),
    pref(
        "MassFlowRate",
        "kg/s",
        &[("mg/s", 1.0), ("g/s", 1.0), ("kg/s", 1.0)],
    ),
    pref(
        "Pressure",
        "Pa",
        &[
            ("mPa", 1.0),
            ("Pa", 1.0),
            ("kPa", 1.0),
            ("MPa", 1.0),
            ("GPa", 1.0),
        ],
    ),
    pref(
        "GravitationalParameter",
        "m^3/s^2",
        &[("m^3/s^2", 1.0), ("km^3/s^2", 1.0)],
    ),
    pref("ElectricCharge", "Ah", &[("mAh", 1.0), ("Ah", 1.0)]),
    pref("Volume", "L", &[("mL", 1.0), ("L", 1.0), ("m^3", 1.0)]),
    pref("ThermalConductance", "W/K", &[("mW/K", 1.0), ("W/K", 1.0)]),
    pref("SpecificHeatCapacity", "J/(kg*K)", &[("J/(kg*K)", 1.0)]),
    pref("Irradiance", "W/m^2", &[("W/m^2", 1.0)]),
    pref(
        "DataRate",
        "bit/s",
        &[
            ("bit/s", 1.0),
            ("kbit/s", 1.0),
            ("Mbit/s", 1.0),
            ("Gbit/s", 1.0),
        ],
    ),
    pref("SolidAngle", "sr", &[("sr", 1.0)]),
    pref(
        "Resistance",
        "ohm",
        &[("mohm", 1.0), ("ohm", 1.0), ("kohm", 1.0), ("Mohm", 1.0)],
    ),
    pref(
        "Capacitance",
        "F",
        &[
            ("pF", 1.0),
            ("nF", 1.0),
            ("uF", 1.0),
            ("mF", 1.0),
            ("F", 1.0),
        ],
    ),
    pref(
        "Inductance",
        "H",
        &[("nH", 1.0), ("uH", 1.0), ("mH", 1.0), ("H", 1.0)],
    ),
    pref("Conductance", "S", &[("mS", 1.0), ("S", 1.0)]),
    pref("MagneticFlux", "Wb", &[("Wb", 1.0)]),
    pref(
        "AbsorbedDose",
        "Gy",
        &[("mGy", 1.0), ("Gy", 1.0), ("kGy", 1.0)],
    ),
];
