use super::*;
use crate::typed::{Dimension as _, Energy, Power, Torque};
use alloc::vec::Vec;

fn text(si: f64, dim: &str) -> String {
    format_quantity(si, dim).expect(dim).text
}

/// 表示の好みの並びは、すべて crate の解析器で読めて、次元名の次元と一致する。
#[test]
fn every_preferred_unit_parses_with_matching_dimension() {
    for p in PREFERENCES {
        let dim = parse_dimension(p.name).unwrap_or_else(|_| panic!("{} は次元名ではない", p.name));
        assert_eq!(canonical_dimension_name(p.name), Some(p.name), "{}", p.name);
        for symbol in p.units.iter().map(|(u, _)| *u).chain([p.base]) {
            let unit = parse_unit(symbol).unwrap_or_else(|e| panic!("{symbol}: {e:?}"));
            assert!(
                unit.dimension().is_compatible_with(dim),
                "{symbol} は {} でない",
                p.name
            );
        }
        // 小さい順に並んでいる
        let scales: Vec<f64> = p
            .units
            .iter()
            .map(|(u, _)| parse_unit(u).expect(u).scale())
            .collect();
        assert!(scales.windows(2).all(|w| w[0] < w[1]), "{} の並び", p.name);
    }
}

#[test]
fn power_picks_si_prefix() {
    assert_eq!(text(0.0123, "Power"), "12.3 mW");
    assert_eq!(text(18.2, "Power"), "18.2 W");
    assert_eq!(text(19.0, "Power"), "19.0 W");
    assert_eq!(text(1500.0, "Power"), "1.50 kW");
    assert_eq!(text(4.2e-7, "Power"), "420 nW");
    assert_eq!(text(0.0, "Power"), "0 W");
    // 丸めで桁が上がるときは上の単位にする
    assert_eq!(text(0.999_96, "Power"), "1.00 W");
    let f = format_quantity(0.0123, "power").expect("power");
    assert_eq!(f.unit, "mW");
    assert!((f.value - 12.3).abs() < 1e-12);
}

#[test]
fn energy_prefers_watt_hours_and_torque_is_distinct() {
    assert_eq!(text(3600.0 * 20.0, "Energy"), "20.0 Wh");
    assert_eq!(text(36.0, "Energy"), "10.0 mWh");
    assert_eq!(text(0.012, "Torque"), "12.0 mN·m");
    assert_eq!(format_dimension(72_000.0, Energy::DIM).text, "20.0 Wh");
    assert_eq!(dimension_name(Torque::DIM), Some("Energy"));
    assert_eq!(dimension_name(Power::DIM), Some("Power"));
}

#[test]
fn angular_momentum_angle_time() {
    assert_eq!(text(0.018, "AngularMomentum"), "18.0 mN·m·s");
    assert_eq!(text(1.2, "angular_momentum"), "1.20 N·m·s");
    assert_eq!(text(97.6_f64.to_radians(), "Angle"), "97.6°");
    assert_eq!(text(5.0, "Time"), "5.00 s");
    assert_eq!(text(90.0, "Time"), "90.0 s");
    assert_eq!(text(5700.0, "Time"), "95.0 min");
    assert_eq!(text(86_400.0, "Time"), "24.0 h");
    assert_eq!(text(3.0 * 86_400.0, "Time"), "3.00 d");
    assert_eq!(text(0.004, "Time"), "4.00 ms");
}

#[test]
fn data_information_temperature() {
    assert_eq!(text(9600.0, "DataRate"), "9.60 kbit/s");
    assert_eq!(text(2.5e6, "DataRate"), "2.50 Mbit/s");
    assert_eq!(text(8.0 * 1.5e9, "Information"), "1.50 GB");
    assert_eq!(text(8.0 * 512.0, "Information"), "512 B");
    assert_eq!(text(293.15, "Temperature"), "20.0 °C");
    assert_eq!(text(233.15, "Temperature"), "-40.0 °C");
    assert_eq!(format_in_unit(10.0, "K").expect("K").text, "10.0 K");
}

#[test]
fn dimensionless_and_fallback() {
    assert_eq!(text(0.62, "Dimensionless"), "0.620");
    assert_eq!(
        format_quantity(1.0, "NoSuchDimension"),
        Err(UnitError::UnknownDimension)
    );
    // 好みの無い次元は SI の基本単位の式で出す（解析し直せる）
    let f = format_quantity(2.0, "InverseSquareTime").expect("InverseSquareTime");
    assert_eq!(f.unit, "1/s^2");
    assert!(parse_unit(&f.unit).is_ok());
    let power_expr = si_unit_expression(Power::DIM);
    assert_eq!(power_expr, "m^2*kg/s^3");
    assert!(
        parse_unit(&power_expr)
            .expect("式")
            .dimension()
            .is_compatible_with(Power::DIM)
    );
}

#[test]
fn logarithmic_target_requires_positive() {
    assert_eq!(
        format_in_unit(0.0, "dBm"),
        Err(UnitError::NonPositiveForLogarithmic)
    );
    assert_eq!(format_in_unit(1e-3, "dBm").expect("dBm").text, "0 dBm");
    assert_eq!(format_in_unit(100.0, "dB").expect("dB").text, "20.0 dB");
}

#[test]
fn number_formatting() {
    assert_eq!(format_number(123_456.7, 3), "123457");
    assert_eq!(format_number(-0.001_234, 3), "-0.00123");
    assert_eq!(format_number(-0.000_123_4, 3), "-1.23e-4");
    assert_eq!(format_number(1.0e-7, 3), "1.00e-7");
    assert_eq!(format_number(f64::NAN, 3), "NaN");
    assert_eq!(format_number(-0.0, 3), "0");
    assert_eq!(text(f64::INFINITY, "Power"), "inf W");
}

#[test]
fn display_units_lists_candidates() {
    let units: Vec<&str> = display_units("Power").expect("Power").collect();
    assert_eq!(units, ["nW", "uW", "mW", "W", "kW", "MW"]);
    assert!(display_units("Nope").is_none());
}

#[test]
fn dimension_label_names_or_expresses() {
    assert_eq!(dimension_label(Power::DIM), "Power");
    let odd = Power::DIM.try_mul(Power::DIM).expect("次元");
    assert_eq!(dimension_label(odd), si_unit_expression(odd));
    assert!(parse_unit(&dimension_label(odd)).is_ok());
}

#[test]
fn fractions_are_shown_in_percent() {
    let f = format_quantity(0.253, crate::typed::Fraction::KIND).expect("Fraction");
    assert_eq!(f.unit, "%");
    assert!((f.value - 25.3).abs() < 1e-9);
    assert_eq!(canonical_dimension_name("fraction"), Some("Fraction"));
    assert_eq!(
        crate::parse_dimension("Fraction"),
        Ok(crate::Dimension::DIMENSIONLESS)
    );
    assert_eq!(format_quantity(0.0, "Fraction").expect("0").unit, "%");
}

#[test]
fn decibel_kinds_format_existing_logarithmic_values() {
    for (kind, unit) in [
        ("Decibel", "dB"),
        ("DecibelWatt", "dBW"),
        ("DecibelPerKelvin", "dB/K"),
        ("DecibelHertz", "dB·Hz"),
    ] {
        for value in [-12.5, 0.0, 33.0] {
            let q = format_quantity(value, kind).unwrap();
            assert_eq!(q.value, value);
            assert_eq!(q.unit, unit);
        }
    }
}
