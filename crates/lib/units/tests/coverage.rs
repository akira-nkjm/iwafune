use sat_units::typed::{
    Acceleration, Angle, AngularVelocity, DataRate, Dimension as _, ElectricCharge, Energy,
    Information, MagneticFluxDensity, Pressure, SpecificHeatCapacity, ThermalConductance, Time,
    Volume,
};
use sat_units::{NAMED_UNIT_SYMBOLS, UnitError, named_unit, parse_dimension, parse_unit};

fn assert_scale(symbol: &str, expected: f64) {
    let actual = parse_unit(symbol).unwrap_or_else(|error| panic!("{symbol}: {error:?}"));
    let tolerance = expected.abs() * 1.0e-12 + f64::MIN_POSITIVE;
    assert!(
        (actual.scale() - expected).abs() <= tolerance,
        "{symbol}: {} != {expected}",
        actual.scale()
    );
}

#[test]
fn spacecraft_units_have_defined_si_scales() {
    for (symbol, expected) in [
        ("Ah", 3_600.0),
        ("mAh", 3.6),
        ("ohm", 1.0),
        ("F", 1.0),
        ("T", 1.0),
        ("nT", 1.0e-9),
        ("uT", 1.0e-6),
        ("Wb", 1.0),
        ("rpm", core::f64::consts::PI / 30.0),
        ("min", 60.0),
        ("d", 86_400.0),
        ("day", 86_400.0),
        ("yr", 31_557_600.0),
        ("AU", 149_597_870_700.0),
        ("arcmin", core::f64::consts::PI / 10_800.0),
        ("arcsec", core::f64::consts::PI / 648_000.0),
        ("sr", 1.0),
        ("lm", 1.0),
        ("lx", 1.0),
        ("bar", 100_000.0),
        ("atm", 101_325.0),
        ("psi", 6_894.757_293_168),
        ("L", 1.0e-3),
        ("mL", 1.0e-6),
        ("ppm", 1.0e-6),
        ("g0", 9.806_65),
        ("bit", 1.0),
        ("B", 8.0),
        ("bps", 1.0),
        ("kbps", 1_000.0),
        // 同じ方針で補う、衛星の電気・磁気設計で頻出する単位。
        ("H", 1.0),
        ("S", 1.0),
        ("eV", 1.602_176_634e-19),
        ("G", 1.0e-4),
    ] {
        assert_scale(symbol, expected);
        assert!(
            NAMED_UNIT_SYMBOLS.contains(&symbol),
            "{symbol} must be exposed by the catalog"
        );
        assert!(
            named_unit(symbol).is_some(),
            "{symbol} must be a named unit"
        );
    }
}

#[test]
fn spacecraft_units_have_expected_dimensions() {
    assert_eq!(parse_unit("Ah").unwrap().dimension(), ElectricCharge::DIM);
    assert_eq!(
        parse_unit("T").unwrap().dimension(),
        MagneticFluxDensity::DIM
    );
    assert_eq!(parse_unit("rpm").unwrap().dimension(), AngularVelocity::DIM);
    assert_eq!(parse_unit("min").unwrap().dimension(), Time::DIM);
    assert_eq!(
        parse_unit("sr").unwrap().dimension(),
        Angle::DIM.powi(2).unwrap()
    );
    assert_eq!(parse_unit("bar").unwrap().dimension(), Pressure::DIM);
    assert_eq!(parse_unit("L").unwrap().dimension(), Volume::DIM);
    assert_eq!(parse_unit("g0").unwrap().dimension(), Acceleration::DIM);
    assert_eq!(parse_unit("eV").unwrap().dimension(), Energy::DIM);
    for (symbol, expression) in [
        ("ohm", "V/A"),
        ("F", "C/V"),
        ("H", "Wb/A"),
        ("S", "A/V"),
        ("Wb", "V*s"),
        ("lm", "cd*sr"),
        ("lx", "lm/m^2"),
    ] {
        assert_eq!(
            parse_unit(symbol).unwrap().dimension(),
            parse_unit(expression).unwrap().dimension(),
            "{symbol} must match {expression}"
        );
    }
    assert_eq!(parse_unit("bit").unwrap().dimension(), Information::DIM);
    assert_eq!(parse_unit("bps").unwrap().dimension(), DataRate::DIM);
    assert_eq!(parse_dimension("Information"), Ok(Information::DIM));
    assert_eq!(parse_dimension("data_rate"), Ok(DataRate::DIM));
}

#[test]
fn information_units_accept_decimal_but_not_binary_prefixes() {
    assert_scale("kbit", 1.0e3);
    assert_scale("Mbit", 1.0e6);
    assert_scale("Gbit", 1.0e9);
    assert_scale("kB", 8.0e3);
    assert_scale("Mbps", 1.0e6);
    assert_scale("Gbps", 1.0e9);

    for unsupported in ["Kibit", "KiB", "Kibps", "Mibit", "Gibit"] {
        assert_eq!(parse_unit(unsupported), Err(UnitError::UnknownUnit));
    }
}

#[test]
fn exact_named_units_win_before_prefix_splitting() {
    // `min` は milli + `in` ではなく minute、`mL` は明示した millilitre。
    assert_scale("min", 60.0);
    assert_scale("mL", 1.0e-6);
    assert_scale("mAh", 3.6);
    assert_scale("nT", 1.0e-9);
    assert_scale("ppm", 1.0e-6);

    // `G` は gauss の完全一致。後続する `s` がある `Gs` は SI 接頭辞 G + second。
    assert_scale("G", 1.0e-4);
    assert_scale("Gs", 1.0e9);
    assert_scale("G*s", 1.0e-4);
}

#[test]
fn parentheses_group_denominators_and_can_nest() {
    assert_eq!(
        parse_unit("W/(m*K)").unwrap().dimension(),
        ThermalConductance::DIM
            .try_div(sat_units::typed::Length::DIM)
            .unwrap()
    );
    assert_eq!(
        parse_unit("J/(kg*K)").unwrap().dimension(),
        SpecificHeatCapacity::DIM
    );
    assert_eq!(parse_unit("((J)/(kg*(K)))"), parse_unit("J/kg/K"));
    assert_eq!(parse_unit("(m/s)^2"), parse_unit("m^2/s^2"));

    // 括弧の外では従来どおり左結合。
    assert_eq!(parse_unit("W/K*s"), parse_unit("(W/K)*s"));
    assert_ne!(parse_unit("W/K*s"), parse_unit("W/(K*s)"));
}

#[test]
fn malformed_parentheses_are_rejected() {
    for malformed in ["(", ")", "()", "W/(m*K", "W/m*K)", "W/(*K)", "W/(m/)K"] {
        assert_eq!(
            parse_unit(malformed),
            Err(UnitError::UnknownUnit),
            "{malformed} must be rejected"
        );
    }

    let maximum = format!("{}m{}", "(".repeat(32), ")".repeat(32));
    assert!(parse_unit(&maximum).is_ok());
    let too_deep = format!("{}m{}", "(".repeat(33), ")".repeat(33));
    assert_eq!(parse_unit(&too_deep), Err(UnitError::UnknownUnit));
}
