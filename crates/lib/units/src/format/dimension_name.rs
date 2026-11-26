//! 次元の表示名（日本語と英語の対）。画面に次元を出すときの名前は、次元名（識別子 `AngularMomentum`）ではなくここを使う。
//! 対は `label`（日本語）と `label_en`（英語）で、語彙や種類の表示名と同じ形（画面は `localized(label, label_en)` で選ぶ）。

use super::canonical_dimension_name;

/// 次元の表示名の対。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DimensionDisplayName {
    /// 日本語の表示名
    pub label: &'static str,
    /// 英語の表示名
    pub label_en: &'static str,
}

const fn names(label: &'static str, label_en: &'static str) -> DimensionDisplayName {
    DimensionDisplayName { label, label_en }
}

/// 正規の次元名（[`crate::named_dimensions`] の綴りと、表示の種類 `Fraction`・`Decibel*`）→ 表示名の対。
const NAMES: &[(&str, DimensionDisplayName)] = &[
    ("Dimensionless", names("無次元", "Dimensionless")),
    ("Length", names("長さ", "Length")),
    ("Mass", names("質量", "Mass")),
    ("Time", names("時間", "Time")),
    ("ElectricCurrent", names("電流", "Electric current")),
    ("Temperature", names("温度", "Temperature")),
    (
        "TemperatureDifference",
        names("温度差", "Temperature difference"),
    ),
    ("AmountOfSubstance", names("物質量", "Amount of substance")),
    ("LuminousIntensity", names("光度", "Luminous intensity")),
    ("Angle", names("角度", "Angle")),
    ("Information", names("情報量", "Information")),
    ("Frequency", names("周波数", "Frequency")),
    (
        "InverseSquareTime",
        names("時間の 2 乗の逆数", "Inverse square time"),
    ),
    ("Velocity", names("速度", "Velocity")),
    ("Acceleration", names("加速度", "Acceleration")),
    ("AngularVelocity", names("角速度", "Angular velocity")),
    ("Force", names("力", "Force")),
    ("Energy", names("エネルギー", "Energy")),
    ("Torque", names("トルク", "Torque")),
    (
        "MomentOfInertia",
        names("慣性モーメント", "Moment of inertia"),
    ),
    ("AngularMomentum", names("角運動量", "Angular momentum")),
    ("Power", names("電力・仕事率", "Power")),
    ("Voltage", names("電圧", "Voltage")),
    (
        "MagneticFluxDensity",
        names("磁束密度", "Magnetic flux density"),
    ),
    (
        "MagneticDipoleMoment",
        names("磁気双極子モーメント", "Magnetic dipole moment"),
    ),
    ("Area", names("面積", "Area")),
    ("MassDensity", names("密度", "Mass density")),
    ("MassFlowRate", names("質量流量", "Mass flow rate")),
    ("Pressure", names("圧力", "Pressure")),
    (
        "GravitationalParameter",
        names("重力定数（μ）", "Gravitational parameter"),
    ),
    (
        "SquareRootTime",
        names("時間の平方根", "Square root of time"),
    ),
    (
        "SquareRootFrequency",
        names("周波数の平方根", "Square root of frequency"),
    ),
    (
        "AccelerationNoiseDensity",
        names("加速度ノイズ密度", "Acceleration noise density"),
    ),
    (
        "AngularRandomWalk",
        names("角度ランダムウォーク", "Angular random walk"),
    ),
    ("ElectricCharge", names("電荷", "Electric charge")),
    ("Volume", names("体積", "Volume")),
    (
        "ThermalConductance",
        names("熱コンダクタンス", "Thermal conductance"),
    ),
    (
        "ThermalConductivity",
        names("熱伝導率", "Thermal conductivity"),
    ),
    (
        "HeatTransferCoefficient",
        names("熱伝達率", "Heat transfer coefficient"),
    ),
    (
        "SpecificHeatCapacity",
        names("比熱", "Specific heat capacity"),
    ),
    ("SpecificEnergy", names("比エネルギー", "Specific energy")),
    ("Irradiance", names("放射照度", "Irradiance")),
    ("DataRate", names("データレート", "Data rate")),
    ("SolidAngle", names("立体角", "Solid angle")),
    ("LuminousFlux", names("光束", "Luminous flux")),
    ("Illuminance", names("照度", "Illuminance")),
    ("Resistance", names("電気抵抗", "Resistance")),
    ("Capacitance", names("静電容量", "Capacitance")),
    ("Inductance", names("インダクタンス", "Inductance")),
    ("Conductance", names("コンダクタンス", "Conductance")),
    ("MagneticFlux", names("磁束", "Magnetic flux")),
    ("AbsorbedDose", names("吸収線量", "Absorbed dose")),
    (
        "AbsorbedDoseRate",
        names("吸収線量率", "Absorbed dose rate"),
    ),
    ("ParticleFlux", names("粒子フラックス", "Particle flux")),
    (
        "ParticleFluence",
        names("粒子フルエンス", "Particle fluence"),
    ),
    ("Impulse", names("力積", "Impulse")),
    // 表示の種類（次元は無次元）
    ("Fraction", names("比", "Fraction")),
    ("Decibel", names("デシベル", "Decibel")),
    ("DecibelWatt", names("デシベル（W 基準）", "Decibel (watt)")),
    (
        "DecibelPerKelvin",
        names("デシベル（K 基準）", "Decibel (per kelvin)"),
    ),
    (
        "DecibelHertz",
        names("デシベル（Hz 基準）", "Decibel (hertz)"),
    ),
];

/// 次元名（綴りの流儀は問わない。別名 `current` も可）の表示名の対。名前の無い次元名は None。
pub fn dimension_display_name(name: &str) -> Option<DimensionDisplayName> {
    let canonical = canonical_dimension_name(name)?;
    NAMES.iter().find(|(n, _)| *n == canonical).map(|(_, d)| *d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::named_dimensions;

    /// 新しい次元を足したら表示名の対も要る（足し忘れの検出）。
    #[test]
    fn every_dimension_has_a_non_empty_pair() {
        for (name, _) in named_dimensions() {
            let d = dimension_display_name(name)
                .unwrap_or_else(|| panic!("no display name for {name}"));
            assert!(!d.label.is_empty() && !d.label_en.is_empty(), "{name}");
        }
    }

    #[test]
    fn table_has_no_stale_or_duplicate_entries() {
        for (i, (name, _)) in NAMES.iter().enumerate() {
            assert!(canonical_dimension_name(name) == Some(*name), "{name}");
            assert!(NAMES[..i].iter().all(|(n, _)| n != name), "dup {name}");
        }
    }

    #[test]
    fn kinds_aliases_and_spelling_resolve() {
        let a = dimension_display_name("angular_momentum").unwrap();
        assert_eq!((a.label, a.label_en), ("角運動量", "Angular momentum"));
        assert_eq!(
            dimension_display_name("current"),
            dimension_display_name("ElectricCurrent")
        );
        assert_eq!(
            dimension_display_name("Fraction").unwrap().label_en,
            "Fraction"
        );
        assert!(dimension_display_name("L^2 M T^-3").is_none());
    }
}
