use super::*;
use crate::Rational;
use crate::typed::Dimension as TypedDimension;
use crate::typed::{
    AbsorbedDose, AccelerationNoiseDensity, Angle, AngularMomentum, AngularRandomWalk, Area,
    Dimensionless, ElectricCharge, Force, Frequency, Length, MassDensity, Pressure,
    SpecificHeatCapacity, SquareRootFrequency, SquareRootTime, ThermalConductance, Velocity,
    Voltage,
};

fn close(left: f64, right: f64) {
    assert!((left - right).abs() < 1e-9, "{left} != {right}");
}

pub(super) fn dimension_of(input: &str) -> Dimension {
    parse_unit(input).expect(input).dimension()
}

/// 基底指数（L, M, T, I, K, N, J, Angle, Information）から次元を作る。名前付き次元を経由せずに
/// 指数そのものを固定したいテスト用。
fn exponents(values: [i8; 9]) -> Dimension {
    Dimension::from_exponents(values.map(Rational::from_int))
}

#[test]
fn parses_prefixed_units_without_allocation() {
    let kv = parse_unit("kV").unwrap();
    let volt = named_unit("V").unwrap();
    assert_eq!(kv.dimension(), volt.dimension());
    assert_eq!(kv.scale(), 1000.0 * volt.scale());
}

#[test]
fn exact_unit_match_wins() {
    assert_eq!(parse_unit("V"), Ok(named_unit("V").unwrap()));
    assert_eq!(parse_unit("m"), Ok(named_unit("m").unwrap()));
    assert_eq!(parse_unit("kg"), Ok(named_unit("kg").unwrap()));
}

#[test]
fn parses_named_dimensions() {
    let voltage_dimension = named_unit("V").unwrap().dimension();
    assert_eq!(parse_dimension("voltage"), Ok(voltage_dimension));
}

#[test]
fn unknown_unit_is_reported() {
    assert_eq!(parse_unit("xyz"), Err(UnitError::UnknownUnit));
}

#[test]
fn prefixed_affine_unit_is_rejected() {
    // "mdegC"(milli + degC)のような接頭辞付き affine 単位は無意味なので拒否。
    assert_eq!(parse_unit("mdegC"), Err(UnitError::AffineInArithmetic));
}

#[test]
fn unknown_dimension_is_reported() {
    assert_eq!(parse_dimension("bogus"), Err(UnitError::UnknownDimension));
}

#[test]
fn concept_examples_parse_with_expected_dimensions() {
    // よく使う単位の例。次元まで一致することを確かめる。
    // `N*m` は Torque::DIM とも Energy::DIM とも一致してしまうので、
    // ここでは基底指数を直接固定する。N か m の定義が動けばこれが落ちる。
    assert_eq!(dimension_of("N*m"), exponents([2, 1, -2, 0, 0, 0, 0, 0, 0]));
    assert_eq!(dimension_of("N*m*s"), AngularMomentum::DIM);
    assert_eq!(dimension_of("m^2"), Area::DIM);
    assert_eq!(dimension_of("W/K"), ThermalConductance::DIM);
    assert_eq!(dimension_of("deg"), Angle::DIM);
    close(
        parse_unit("deg").unwrap().scale(),
        0.017_453_292_519_943_295,
    );
}

#[test]
fn schema_unit_strings_parse() {
    // ssot/schema/{iwafune-6u,eps}.toml に現れる unit 文字列の全て。
    for unit in ["", "N", "N*m", "N*m*s", "V", "W", "Wh", "kg", "m", "s", "A"] {
        assert!(parse_unit(unit).is_ok(), "{unit} must parse");
    }
}

#[test]
fn empty_unit_is_dimensionless() {
    let unit = parse_unit("").unwrap();
    assert_eq!(unit.dimension(), Dimensionless::DIM);
    assert_eq!(unit.scale(), 1.0);
    assert_eq!(parse_unit("  "), Ok(DIMENSIONLESS_UNIT));
}

#[test]
fn added_named_units_have_expected_dimensions() {
    assert_eq!(dimension_of("N"), Force::DIM);
    assert_eq!(dimension_of("Hz"), Frequency::DIM);
    assert_eq!(dimension_of("Pa"), Pressure::DIM);
    assert_eq!(dimension_of("C"), ElectricCharge::DIM);
    assert_eq!(dimension_of("rad"), Angle::DIM);
    assert_eq!(dimension_of("%"), Dimensionless::DIM);
    close(parse_unit("%").unwrap().scale(), 0.01);
    close(parse_unit("W*h").unwrap().scale(), 3600.0);
}

#[test]
fn division_and_powers_compose_dimensions_and_scales() {
    assert_eq!(dimension_of("m/s"), Velocity::DIM);
    assert_eq!(dimension_of("kg/m^3"), MassDensity::DIM);
    assert_eq!(
        dimension_of("m^-3"),
        parse_unit("1/m^3").unwrap().dimension()
    );
    assert_eq!(dimension_of("s^2"), parse_unit("s*s").unwrap().dimension());
    // 接頭辞は項ごとに掛かる: km^2 = (10^3 m)^2 = 10^6 m^2。
    close(parse_unit("km^2").unwrap().scale(), 1.0e6);
    close(parse_unit("km/h").unwrap().scale(), 1000.0 / 3600.0);
    close(parse_unit("mN*m").unwrap().scale(), 1.0e-3);
    close(parse_unit("MHz").unwrap().scale(), 1.0e6);
    close(parse_unit("µm").unwrap().scale(), 1.0e-6);
    // 冪ゼロは無次元に畳み込まれる。
    assert_eq!(parse_unit("m^0"), Ok(DIMENSIONLESS_UNIT));
}

#[test]
fn expressions_fold_left_to_right() {
    // 括弧も優先順位も無いので W/K*s は (W/K)*s。
    let expected = parse_unit("W/K")
        .unwrap()
        .try_mul(&named_unit("s").unwrap());
    assert_eq!(parse_unit("W/K*s"), expected);
}

#[test]
fn affine_units_are_rejected_inside_expressions() {
    assert_eq!(parse_unit("degC/s"), Err(UnitError::AffineInArithmetic));
    assert_eq!(parse_unit("s*degC"), Err(UnitError::AffineInArithmetic));
    assert_eq!(parse_unit("degC^2"), Err(UnitError::AffineInArithmetic));
    assert_eq!(parse_unit("W/degF"), Err(UnitError::AffineInArithmetic));
    // 単独の affine 単位は従来どおり有効。
    assert_eq!(parse_unit("degC"), Ok(named_unit("degC").unwrap()));
}

#[test]
fn logarithmic_units_parse_alone() {
    for symbol in ["dB", "dBi", "dBm", "dBW"] {
        let unit = parse_unit(symbol).expect(symbol);
        assert!(unit.is_logarithmic(), "{symbol} must be logarithmic");
        assert_eq!(Ok(unit), named_unit(symbol).ok_or(UnitError::UnknownUnit));
    }
    // 前後の空白は単独の記号と同じ扱い。
    assert_eq!(parse_unit(" dBm "), parse_unit("dBm"));
}

#[test]
fn logarithmic_units_are_rejected_inside_expressions() {
    for input in [
        "dBm/Hz", "dB*W", "W/dB", "dBW^2", "dB^-1", "dBi*1", "dBm/dB",
    ] {
        assert_eq!(
            parse_unit(input),
            Err(UnitError::LogarithmicInArithmetic),
            "{input} must be rejected"
        );
    }
}

#[test]
fn prefixed_logarithmic_unit_is_rejected() {
    // "kdB" / "mdBm" は基準値を黙ってずらすだけで、意味のある単位にならない。
    for input in ["kdB", "mdBm", "MdBW", "udBi"] {
        assert_eq!(
            parse_unit(input),
            Err(UnitError::LogarithmicInArithmetic),
            "{input} must be rejected"
        );
    }
}

#[test]
fn malformed_expressions_are_rejected() {
    for input in [
        "N*",
        "m^",
        "m^^2",
        "/s",
        "*",
        "N//m",
        "m^x",
        "N m",
        "m^999",
        "m^()",
        "m^(1/0)",
        "m^(1/2",
        "m^(1/2/3)",
        "m^((1/2))",
        "m^(a/2)",
        "m^(1*2)",
    ] {
        assert_eq!(
            parse_unit(input),
            Err(UnitError::UnknownUnit),
            "{input} must be rejected"
        );
    }
}

#[test]
fn specific_heat_capacity_accepts_grouped_denominator() {
    assert_eq!(dimension_of("J/kg/K"), SpecificHeatCapacity::DIM);
    assert_eq!(dimension_of("J/(kg*K)"), SpecificHeatCapacity::DIM);
    // `J/kg*K` は `(J/kg)*K` なので K の指数が `-1` ではなく `+1` になる。
    assert_ne!(dimension_of("J/kg*K"), SpecificHeatCapacity::DIM);
    assert_eq!(
        dimension_of("J/kg*K"),
        exponents([2, 0, -2, 0, 1, 0, 0, 0, 0])
    );
}

#[test]
fn symbols_without_an_ascii_root_are_rejected() {
    // `√` は文法に無い。括弧の無い `s^-1/2` は `(s^-1)/2` と読み、`2` は単位でない。
    for input in ["µg/√Hz", "deg/√hr", "s^-1/2"] {
        assert_eq!(
            parse_unit(input),
            Err(UnitError::UnknownUnit),
            "{input} must be rejected"
        );
    }
}

#[test]
fn fractional_exponents_are_written_in_parentheses() {
    assert_eq!(dimension_of("Hz^(1/2)"), SquareRootFrequency::DIM);
    assert_eq!(dimension_of("Hz^(-1/2)"), SquareRootTime::DIM);
    assert_eq!(dimension_of("s^(1/2)"), SquareRootTime::DIM);
    assert_eq!(dimension_of("s^( -1 / 2 )"), SquareRootFrequency::DIM);
    assert_eq!(dimension_of("(m^2)^(1/2)"), Length::DIM);
    assert_eq!(dimension_of("m^(+3/2)"), dimension_of("m*m^(1/2)"));
    // 括弧付きの整数・約分できる分数は整数冪と同じ（scale も掛け算で正確）。
    assert_eq!(parse_unit("km^(2)"), parse_unit("km^2"));
    assert_eq!(parse_unit("km^(4/2)"), parse_unit("km^2"));
    assert_eq!(parse_unit("m^(-3)"), parse_unit("m^-3"));
    // 接頭辞の scale も冪に上がる: kHz^(1/2) = √1000 Hz^(1/2)。
    close(parse_unit("kHz^(1/2)").unwrap().scale(), 1000f64.sqrt());
    close(parse_unit("h^(1/2)").unwrap().scale(), 60.0);
    close(parse_unit("1/Hz^(1/2)").unwrap().scale(), 1.0);
}

#[test]
fn noise_densities_match_their_named_dimensions() {
    // 電圧雑音密度。名前付き次元は無いので基底指数で固定する。
    assert_eq!(
        dimension_of("V/Hz^(1/2)"),
        Voltage::DIM.try_div(SquareRootFrequency::DIM).unwrap()
    );
    assert_eq!(dimension_of("nV/Hz^(1/2)"), dimension_of("V/Hz^(1/2)"));
    assert_eq!(
        dimension_of("m/s^2/Hz^(1/2)"),
        AccelerationNoiseDensity::DIM
    );
    assert_eq!(dimension_of("ug0/Hz^(1/2)"), AccelerationNoiseDensity::DIM);
    close(parse_unit("ug0/Hz^(1/2)").unwrap().scale(), 9.806_65e-6);
    // 慣行の `µg` の g は重力加速度だが、この文法の g はグラム。次元が合わないことを固定する。
    assert_ne!(dimension_of("ug/Hz^(1/2)"), AccelerationNoiseDensity::DIM);
    assert_eq!(dimension_of("deg/h^(1/2)"), AngularRandomWalk::DIM);
    close(
        parse_unit("deg/h^(1/2)").unwrap().scale(),
        core::f64::consts::PI / 180.0 / 60.0,
    );
}

#[test]
fn decimal_exponents_point_to_the_fraction_syntax() {
    for input in ["Hz^0.5", "V/Hz^0.5", "s^-0.5", "m^2.5", "Hz^(0.5)", "m^.5"] {
        assert_eq!(
            parse_unit(input),
            Err(UnitError::DecimalExponent),
            "{input} must point to ^(1/2)"
        );
    }
}

#[test]
fn rational_exponents_keep_logarithmic_and_affine_units_out() {
    assert_eq!(
        parse_unit("dBm^(1/2)"),
        Err(UnitError::LogarithmicInArithmetic)
    );
    assert_eq!(parse_unit("degC^(1/2)"), Err(UnitError::AffineInArithmetic));
}

#[test]
fn large_prefixed_radian_is_rejected_as_ambiguous() {
    // 放射線分野の krad（1000 rad = 10 Gy）を角度として黙って通さない。
    for input in ["krad", "Mrad", "Grad", "krad/s", "1/krad", "krad^(1/2)"] {
        assert_eq!(
            parse_unit(input),
            Err(UnitError::AmbiguousRadian),
            "{input} must be rejected"
        );
    }
    // 倍率 < 1 は指向精度で普通に使うので角度のまま。
    for (input, scale) in [
        ("mrad", 1.0e-3),
        ("urad", 1.0e-6),
        ("µrad", 1.0e-6),
        ("nrad", 1.0e-9),
        ("crad", 1.0e-2),
    ] {
        let unit = parse_unit(input).expect(input);
        assert_eq!(unit.dimension(), Angle::DIM, "{input}");
        close(unit.scale(), scale);
    }
    assert_eq!(dimension_of("mrad/s"), dimension_of("rad/s"));
    // 素の rad と、rad を含まない k 接頭辞は影響を受けない。
    assert_eq!(dimension_of("rad"), Angle::DIM);
    assert!(parse_unit("km").is_ok());
    // エラー文は次の一手（Gy で書く）を示す。
    extern crate std;
    let message = std::format!("{}", UnitError::AmbiguousRadian);
    assert!(
        message.contains("Gy") && message.contains("10 Gy"),
        "{message}"
    );
}

#[test]
fn absorbed_dose_units_parse_with_prefixes() {
    for input in ["Gy", "Sv", "kGy", "mGy", "uGy", "mSv", "uSv", "J/kg"] {
        assert_eq!(dimension_of(input), AbsorbedDose::DIM, "{input}");
    }
    close(parse_unit("kGy").unwrap().scale(), 1.0e3);
    close(parse_unit("mSv").unwrap().scale(), 1.0e-3);
    assert_eq!(parse_unit("Gy"), parse_unit("Sv"));
    assert_eq!(parse_dimension("absorbed_dose"), Ok(AbsorbedDose::DIM));
    // 1 krad = 10 Gy（エラー文が案内する換算）。
    close(parse_unit("Gy").unwrap().scale() * 10.0, 10.0);
}

#[test]
fn absorbed_dose_shares_its_dimension_with_velocity_squared() {
    // Gy = Sv = J/kg = m^2/s^2。次元検査は線量と速度の 2 乗を区別しない。
    assert_eq!(dimension_of("Gy"), dimension_of("m^2/s^2"));
    assert_eq!(dimension_of("Sv"), dimension_of("J/kg"));
    assert!(dimension_of("Gy").is_compatible_with(dimension_of("(m/s)^2")));
}

#[test]
fn magnitude_parses_alone_only() {
    let mag = parse_unit("mag").unwrap();
    assert!(mag.is_logarithmic());
    assert_eq!(mag.dimension(), Dimensionless::DIM);
    for input in ["mag/s", "mag^2", "1/mag", "mag^(1/2)", "kmag", "mmag"] {
        assert_eq!(
            parse_unit(input),
            Err(UnitError::LogarithmicInArithmetic),
            "{input} must be rejected"
        );
    }
}
