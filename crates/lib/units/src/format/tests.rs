use super::*;
use crate::typed::{Dimension as _, Energy, Power, Temperature, Torque};
use alloc::vec::Vec;

fn text(si: f64, dim: &str) -> String {
    format_quantity(si, dim).expect(dim).text
}

/// 表示の好みの並びは、すべて crate の解析器で読めて、次元名の次元と一致する。
#[test]
fn every_preferred_unit_parses_with_matching_dimension() {
    for p in PREFERENCES {
        let dim = parse_dimension(p.name)
            .unwrap_or_else(|_| panic!("{} is not a dimension name", p.name));
        assert_eq!(canonical_dimension_name(p.name), Some(p.name), "{}", p.name);
        for symbol in p.units.iter().map(|(u, _)| *u).chain([p.base]) {
            let unit = parse_unit(symbol).unwrap_or_else(|e| panic!("{symbol}: {e:?}"));
            assert!(
                unit.dimension().is_compatible_with(dim),
                "{symbol} is not {}",
                p.name
            );
        }
        // 小さい順に並んでいる
        let scales: Vec<f64> = p
            .units
            .iter()
            .map(|(u, _)| parse_unit(u).expect(u).scale())
            .collect();
        assert!(
            scales.windows(2).all(|w| w[0] < w[1]),
            "{} ordering",
            p.name
        );
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
    // 温度差は 273.15 を引かず K で出す: 5 K は 5 K（絶対温度の扱いなら 278.15 K = 5.00 °C や -268 °C になる）
    assert_eq!(text(5.0, "TemperatureDifference"), "5.00 K");
    assert_eq!(text(-2.5, "TemperatureDifference"), "-2.50 K");
    assert_eq!(text(0.0, "TemperatureDifference"), "0 K");
    assert_eq!(text(5.0, "temperature_difference"), "5.00 K");
    // 次元だけからは Temperature（別名の選び方は `dimension_label_preferring`）
    assert_eq!(
        dimension_label_preferring(Temperature::DIM, Some("TemperatureDifference")),
        "TemperatureDifference"
    );
    assert_eq!(
        dimension_label_preferring(Temperature::DIM, None),
        "Temperature"
    );
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
            .expect("expression")
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
    let odd = Power::DIM.try_mul(Power::DIM).expect("dimension");
    assert_eq!(dimension_label(odd), si_unit_expression(odd));
    assert!(parse_unit(&dimension_label(odd)).is_ok());
}

/// 同じ次元の別名は、呼び出し側が知っている名前を正規の綴りで選ぶ。合わない名前は次元から決める。
#[test]
fn dimension_label_prefers_a_matching_alias() {
    assert_eq!(dimension_label_preferring(Torque::DIM, None), "Energy");
    assert_eq!(
        dimension_label_preferring(Torque::DIM, Some("torque")),
        "Torque"
    );
    assert_eq!(
        dimension_label_preferring(Energy::DIM, Some("Energy")),
        "Energy"
    );
    assert_eq!(
        dimension_label_preferring(Power::DIM, Some("Torque")),
        "Power"
    );
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

/// 時間: 日までは日、2 年（ユリウス年 365.25 d で 730.5 d）以上は年。
#[test]
fn time_switches_to_years_at_two_years() {
    const YEAR: f64 = 365.25 * 86_400.0;
    assert_eq!(text(15.0 * YEAR, "Time"), "15.0 yr");
    assert_eq!(text(2.0 * YEAR, "Time"), "2.00 yr");
    assert_eq!(text(1.99 * YEAR, "Time"), "727 d");
    assert_eq!(text(YEAR, "Time"), "365 d");
    assert_eq!(text(370.0 * 86_400.0, "Time"), "370 d");
    assert_eq!(
        format_quantity(25.0 * YEAR, "Time").expect("Time").unit,
        "yr"
    );
}

/// 情報量・周波数: TB・THz まで。1 つ下の単位との境界で切り替わる。
#[test]
fn information_and_frequency_reach_tera() {
    assert_eq!(text(8.0 * 1.0e12, "Information"), "1.00 TB");
    assert_eq!(text(8.0 * 999.0e9, "Information"), "999 GB");
    assert_eq!(text(8.0 * 2.5e13, "Information"), "25.0 TB");
    assert_eq!(text(1.0e12, "Frequency"), "1.00 THz");
    assert_eq!(text(999.0e9, "Frequency"), "999 GHz");
    assert_eq!(text(1.93e14, "Frequency"), "193 THz");
}

/// 長さ: 0.1 AU（約 1.5e7 km）以上だけ AU。静止軌道・月・L2 は km のまま。
#[test]
fn length_uses_au_only_for_interplanetary_distances() {
    const AU: f64 = 149_597_870_700.0;
    assert_eq!(text(AU, "Length"), "1.00 AU");
    assert_eq!(text(5.2 * AU, "Length"), "5.20 AU");
    assert_eq!(text(0.1 * AU, "Length"), "0.100 AU");
    for (si, label) in [
        (0.099 * AU, "just below 0.1 AU"),
        (1.5e9, "L2"),
        (3.844e8, "Moon"),
        (4.2164e7, "GEO"),
        (7.0e6, "LEO"),
    ] {
        let unit = format_quantity(si, "Length").expect(label).unit;
        assert_eq!(unit, "km", "{label}");
    }
}

/// 角度: 0.01° 以上は度、それより小さければ秒角（分角は使わない）。
#[test]
fn angle_uses_arcseconds_below_a_hundredth_of_a_degree() {
    let deg = |d: f64| d.to_radians();
    assert_eq!(text(deg(97.6), "Angle"), "97.6°");
    assert_eq!(text(deg(0.05), "Angle"), "0.0500°");
    assert_eq!(text(deg(0.01), "Angle"), "0.0100°");
    assert_eq!(text(deg(-0.01), "Angle"), "-0.0100°");
    assert_eq!(text(deg(0.005), "Angle"), "18.0 arcsec");
    assert_eq!(text(deg(-0.005), "Angle"), "-18.0 arcsec");
    assert_eq!(text(deg(1.0 / 3600.0), "Angle"), "1.00 arcsec");
    assert_eq!(text(0.0, "Angle"), "0°");
}

/// 角速度: 0.01 deg/s 以上は deg/s、それより小さければ arcsec/s。
#[test]
fn angular_velocity_uses_arcseconds_per_second_below_a_hundredth_deg_per_s() {
    let rate = |d: f64| d.to_radians();
    assert_eq!(text(rate(1.5), "AngularVelocity"), "1.50 °/s");
    assert_eq!(text(rate(0.01), "AngularVelocity"), "0.0100 °/s");
    assert_eq!(text(rate(0.0099), "AngularVelocity"), "35.6 arcsec/s");
    assert_eq!(text(rate(1.0 / 3600.0), "AngularVelocity"), "1.00 arcsec/s");
    assert_eq!(text(0.0, "AngularVelocity"), "0 °/s");
}

/// 比エネルギー（惑星間の出発の C3）: m²/s² から km²/s² へ。C3 = 12.09 km²/s² は 1.209e7 m²/s²。
#[test]
fn specific_energy_is_shown_in_km2_per_s2() {
    assert_eq!(text(12.09e6, "SpecificEnergy"), "12.1 km²/s²");
    assert_eq!(text(500.0, "SpecificEnergy"), "500 m²/s²");
    assert_eq!(text(0.0, "SpecificEnergy"), "0 km²/s²");
}

/// 吸収線量率: SI（Gy/s）から年あたりへ。10 Gy/yr = 10 / (365.25 × 86400) Gy/s。
#[test]
fn absorbed_dose_rate_is_shown_per_year() {
    let per_second = 10.0 / (365.25 * 86_400.0);
    assert_eq!(text(per_second, "AbsorbedDoseRate"), "10.0 Gy/yr");
}

/// 入力の例は、その次元の表示単位で書き、解析すると同じ次元になる。単位の無い次元・未知の次元には無い。
#[test]
fn input_example_is_a_parsable_quantity_of_the_dimension() {
    assert_eq!(input_example("Mass").as_deref(), Some("1 kg"));
    for name in ["Mass", "Power", "Length", "Angle", "Frequency", "Time"] {
        let example = input_example(name).expect(name);
        let (number, unit) = example.split_once(' ').expect(&example);
        assert_eq!(number, "1");
        let dim = parse_dimension(name).expect(name);
        assert!(
            parse_unit(unit)
                .expect(unit)
                .dimension()
                .is_compatible_with(dim)
        );
    }
    assert_eq!(input_example("NoSuchDimension"), None);
}
