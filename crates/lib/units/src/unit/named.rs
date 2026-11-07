//! 記号 1 個の単位の表（[`named_unit`]・[`NAMED_UNIT_SYMBOLS`]）と、表で使う次元。

use crate::{BaseDimension, Dimension, Unit};

/// [`named_unit`] が知っている記号を、宣言順に列挙する。
///
/// **表を外に持たせないための入口**（[`crate::named_dimensions`] と同じ理由）。
/// `named_unit` の `match` と**同じ順序・同じ内容**を保つこと。ずれていないことは
/// `every_listed_symbol_resolves` が固定する。
pub const NAMED_UNIT_SYMBOLS: &[&str] = &[
    // 無次元
    "one", "1", "%", "ppm", //
    // 基本単位
    "m", "s", "kg", "g", "A", "K", "mol", "cd", //
    // 角度（補助次元。無次元比とは区別する）
    "rad", "deg", "arcmin", "arcsec", "sr", //
    // 情報量（補助次元。無次元比とは区別する）
    "bit", "B", //
    // 組立単位
    "N", "Hz", "Pa", "C", "V", "W", "J", "ohm", "F", "H", "S", "Wb", "T", "lm", "lx", "bps", "Gy",
    "Sv", //
    // 非 SI だが実務で使う倍数単位
    "min", "h", "d", "day", "yr", "rpm", "Wh", "Ah", "mAh", "AU", "au", "L", "mL", "bar", "atm",
    "psi", "g0", "eV", "G", "nT", "uT", "kbps", //
    // affine 単位（単独でのみ有効）
    "degC", "degF", //
    // 対数単位（単独でのみ有効）
    "dB", "dBi", "dBm", "dBW", "mag",
];

/// 記号 1 個に対応する単位を引く。SI 接頭辞と `*` `/` `^` の複合式は
/// [`crate::parse_unit`] が扱う。
pub fn named_unit(symbol: &str) -> Option<Unit> {
    match symbol {
        // 無次元
        "one" | "1" => Some(Unit::new(Dimension::DIMENSIONLESS, 1.0, 0.0)),
        "%" => Some(Unit::new(Dimension::DIMENSIONLESS, 0.01, 0.0)),
        // parts per million。比として 10^-6（BIPM SI Brochure 9th ed., §5.4.7）。
        "ppm" => Some(Unit::new(Dimension::DIMENSIONLESS, 1.0e-6, 0.0)),
        // 基本単位
        "m" => Some(Unit::new(length(), 1.0, 0.0)),
        "s" => Some(Unit::new(time(), 1.0, 0.0)),
        "kg" => Some(Unit::new(mass(), 1.0, 0.0)),
        "g" => Some(Unit::new(mass(), 1.0e-3, 0.0)),
        "A" => Some(Unit::new(current(), 1.0, 0.0)),
        "K" => Some(Unit::new(temperature(), 1.0, 0.0)),
        "mol" => Some(Unit::new(amount_of_substance(), 1.0, 0.0)),
        "cd" => Some(Unit::new(luminous_intensity(), 1.0, 0.0)),
        // 角度（補助次元。無次元比とは区別する）
        "rad" => Some(Unit::new(angle(), 1.0, 0.0)),
        "deg" => Some(Unit::new(angle(), core::f64::consts::PI / 180.0, 0.0)),
        "arcmin" => Some(Unit::new(angle(), core::f64::consts::PI / 10_800.0, 0.0)),
        "arcsec" => Some(Unit::new(angle(), core::f64::consts::PI / 648_000.0, 0.0)),
        // SI では sr = rad^2。rad を補助次元にする既存方針に合わせ Angle^2 とする。
        "sr" => Some(Unit::new(solid_angle(), 1.0, 0.0)),
        // 情報量は角度と同じ補助次元。byte は IEC 80000-13 の 8 bit。
        "bit" => Some(Unit::new(information(), 1.0, 0.0)),
        "B" => Some(Unit::new(information(), 8.0, 0.0)),
        // 組立単位
        "N" => Some(Unit::new(force(), 1.0, 0.0)),
        "Hz" => Some(Unit::new(frequency(), 1.0, 0.0)),
        "Pa" => Some(Unit::new(pressure(), 1.0, 0.0)),
        "C" => Some(Unit::new(electric_charge(), 1.0, 0.0)),
        "V" => Some(Unit::new(voltage(), 1.0, 0.0)),
        "W" => Some(Unit::new(power(), 1.0, 0.0)),
        "J" => Some(Unit::new(energy(), 1.0, 0.0)),
        // SI 組立単位（BIPM SI Brochure 9th ed., Table 4）。
        "ohm" => Some(Unit::new(resistance(), 1.0, 0.0)),
        "F" => Some(Unit::new(capacitance(), 1.0, 0.0)),
        "H" => Some(Unit::new(inductance(), 1.0, 0.0)),
        "S" => Some(Unit::new(conductance(), 1.0, 0.0)),
        "Wb" => Some(Unit::new(magnetic_flux(), 1.0, 0.0)),
        "T" => Some(Unit::new(magnetic_flux_density(), 1.0, 0.0)),
        "lm" => Some(Unit::new(luminous_flux(), 1.0, 0.0)),
        "lx" => Some(Unit::new(illuminance(), 1.0, 0.0)),
        "bps" => Some(Unit::new(data_rate(), 1.0, 0.0)),
        // 吸収線量 Gy と線量当量 Sv はどちらも J/kg（BIPM SI Brochure 9th ed., Table 4）。
        // 次元は速度の 2 乗・比エネルギーと同一で、次元検査では区別できない。
        "Gy" | "Sv" => Some(Unit::new(absorbed_dose(), 1.0, 0.0)),
        // 非 SI だが実務で使う倍数単位
        // time / angle は BIPM SI Brochure 9th ed. の換算。yr は IAU の Julian year。
        "min" => Some(Unit::new(time(), 60.0, 0.0)),
        "h" => Some(Unit::new(time(), 3600.0, 0.0)),
        "d" | "day" => Some(Unit::new(time(), 86_400.0, 0.0)),
        "yr" => Some(Unit::new(time(), 365.25 * 86_400.0, 0.0)),
        "rpm" => Some(Unit::new(
            <crate::dimensions::AngularVelocity as crate::dimensions::Dimension>::DIM,
            2.0 * core::f64::consts::PI / 60.0,
            0.0,
        )),
        "Wh" => Some(Unit::new(energy(), 3600.0, 0.0)),
        // NIST SP 811 Appendix B: 1 A*h = 3600 C。
        "Ah" => Some(Unit::new(electric_charge(), 3600.0, 0.0)),
        "mAh" => Some(Unit::new(electric_charge(), 3.6, 0.0)),
        // IAU 2012 Resolution B2。規定記号 au に加え、要求される既存綴り AU も受ける。
        "AU" | "au" => Some(Unit::new(length(), 149_597_870_700.0, 0.0)),
        // litre = 1 dm^3（BIPM SI Brochure 9th ed.）。mL は完全一致を目録にも載せる。
        "L" => Some(Unit::new(
            <crate::dimensions::Volume as crate::dimensions::Dimension>::DIM,
            1.0e-3,
            0.0,
        )),
        "mL" => Some(Unit::new(
            <crate::dimensions::Volume as crate::dimensions::Dimension>::DIM,
            1.0e-6,
            0.0,
        )),
        // NIST SP 811 Appendix B。定義値から SI への換算を固定する。
        "bar" => Some(Unit::new(pressure(), 100_000.0, 0.0)),
        "atm" => Some(Unit::new(pressure(), 101_325.0, 0.0)),
        "psi" => Some(Unit::new(pressure(), 6_894.757_293_168, 0.0)),
        // 2022 CODATA。標準重力加速度は正確に 9.80665 m/s^2。
        "g0" => Some(Unit::new(
            <crate::dimensions::Acceleration as crate::dimensions::Dimension>::DIM,
            9.806_65,
            0.0,
        )),
        // 2019 SI で電気素量が固定されたため eV の換算も正確。
        "eV" => Some(Unit::new(energy(), 1.602_176_634e-19, 0.0)),
        // NIST SP 811 Appendix B: gauss = 10^-4 T。G は接頭辞より完全一致を優先する。
        "G" => Some(Unit::new(magnetic_flux_density(), 1.0e-4, 0.0)),
        "nT" => Some(Unit::new(magnetic_flux_density(), 1.0e-9, 0.0)),
        "uT" => Some(Unit::new(magnetic_flux_density(), 1.0e-6, 0.0)),
        "kbps" => Some(Unit::new(data_rate(), 1.0e3, 0.0)),
        // affine 単位（単独でのみ有効。算術・接頭辞・複合式では拒否する）
        "degC" => Some(Unit::new(temperature(), 1.0, 273.15)),
        "degF" => Some(Unit::new(temperature(), 5.0 / 9.0, 459.67 * 5.0 / 9.0)),
        // 対数単位（単独でのみ有効。算術・接頭辞・複合式では拒否する）。
        // いずれも電力比（10 log10）。dBi は等方性アンテナ比の利得で、換算は dB と同じだが
        // 読み手の意図を残すため記号を分ける。
        "dB" | "dBi" => Some(Unit::logarithmic(Dimension::DIMENSIONLESS, 1.0, 10.0)),
        "dBm" => Some(Unit::logarithmic(power(), 1.0e-3, 10.0)),
        "dBW" => Some(Unit::logarithmic(power(), 1.0, 10.0)),
        // 天体の等級。比 = 10^(-0.4 m)（Pogson）なので factor = -2.5。無次元の比で、
        // 零点（どの明るさを 0 等とするか）は単位ではなく読み手の文脈が持つ。
        "mag" => Some(Unit::logarithmic(Dimension::DIMENSIONLESS, 1.0, -2.5)),
        _ => None,
    }
}

pub(crate) const fn length() -> Dimension {
    Dimension::base(BaseDimension::Length)
}

pub(crate) const fn mass() -> Dimension {
    Dimension::base(BaseDimension::Mass)
}

pub(crate) const fn time() -> Dimension {
    Dimension::base(BaseDimension::Time)
}

pub(crate) const fn current() -> Dimension {
    Dimension::base(BaseDimension::ElectricCurrent)
}

pub(crate) const fn temperature() -> Dimension {
    Dimension::base(BaseDimension::Temperature)
}

pub(crate) const fn amount_of_substance() -> Dimension {
    Dimension::base(BaseDimension::AmountOfSubstance)
}

pub(crate) const fn luminous_intensity() -> Dimension {
    Dimension::base(BaseDimension::LuminousIntensity)
}

pub(crate) const fn angle() -> Dimension {
    Dimension::base(BaseDimension::Angle)
}

pub(crate) const fn information() -> Dimension {
    Dimension::base(BaseDimension::Information)
}

// 組立次元は型レベルの [`crate::typed`] から取る。指数表を二重に持つと片方だけ
// 古くなるため、実行時と型レベルで唯一の定義を共有する。
pub(crate) const fn voltage() -> Dimension {
    <crate::dimensions::Voltage as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn power() -> Dimension {
    <crate::dimensions::Power as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn energy() -> Dimension {
    <crate::dimensions::Energy as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn force() -> Dimension {
    <crate::dimensions::Force as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn frequency() -> Dimension {
    <crate::dimensions::Frequency as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn pressure() -> Dimension {
    <crate::dimensions::Pressure as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn electric_charge() -> Dimension {
    <crate::dimensions::ElectricCharge as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn resistance() -> Dimension {
    <crate::dimensions::Resistance as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn capacitance() -> Dimension {
    <crate::dimensions::Capacitance as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn inductance() -> Dimension {
    <crate::dimensions::Inductance as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn conductance() -> Dimension {
    <crate::dimensions::Conductance as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn magnetic_flux() -> Dimension {
    <crate::dimensions::MagneticFlux as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn magnetic_flux_density() -> Dimension {
    <crate::dimensions::MagneticFluxDensity as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn solid_angle() -> Dimension {
    <crate::dimensions::SolidAngle as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn luminous_flux() -> Dimension {
    <crate::dimensions::LuminousFlux as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn illuminance() -> Dimension {
    <crate::dimensions::Illuminance as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn data_rate() -> Dimension {
    <crate::dimensions::DataRate as crate::dimensions::Dimension>::DIM
}

pub(crate) const fn absorbed_dose() -> Dimension {
    <crate::dimensions::AbsorbedDose as crate::dimensions::Dimension>::DIM
}
