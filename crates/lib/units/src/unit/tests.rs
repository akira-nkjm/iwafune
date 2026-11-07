use super::named::length;
use super::*;

/// 列挙表と `named_unit` の `match` がずれていないこと。
///
/// 表を 2 つ持つ以上、片方だけ足すことは起こる。**足し忘れは黙って
/// 「その単位は無い」に化ける**ので、ここで落とす。
#[test]
fn every_listed_symbol_resolves() {
    for symbol in NAMED_UNIT_SYMBOLS {
        assert!(
            named_unit(symbol).is_some(),
            "NAMED_UNIT_SYMBOLS に載っている {symbol} を named_unit が知らない"
        );
    }
}

/// 列挙表に重複が無いこと（同じ記号を 2 度出さない）。
#[test]
fn the_symbol_list_has_no_duplicates() {
    for (index, symbol) in NAMED_UNIT_SYMBOLS.iter().enumerate() {
        assert!(
            !NAMED_UNIT_SYMBOLS[index + 1..].contains(symbol),
            "{symbol} が 2 度出ている"
        );
    }
}

/// affine 単位は列挙にも含める（単独でのみ有効、という性質ごと見せるため）。
#[test]
fn affine_units_are_listed() {
    assert!(NAMED_UNIT_SYMBOLS.contains(&"degC"));
    assert!(named_unit("degC").expect("degC is known").is_affine());
}

fn close(left: f64, right: f64) {
    assert!((left - right).abs() < 1e-9, "{left} != {right}");
}

#[test]
fn volts_convert_to_themselves() {
    let volt = named_unit("V").unwrap();
    close(volt.convert(12.5, &volt).unwrap(), 12.5);
}

#[test]
fn watt_hours_convert_to_joules() {
    let wh = named_unit("Wh").unwrap();
    let joule = named_unit("J").unwrap();
    close(wh.convert(1.0, &joule).unwrap(), 3600.0);
}

#[test]
fn celsius_and_kelvin_round_trip() {
    let celsius = named_unit("degC").unwrap();
    let kelvin = named_unit("K").unwrap();
    close(celsius.convert(0.0, &kelvin).unwrap(), 273.15);
    close(celsius.convert(100.0, &kelvin).unwrap(), 373.15);
    close(kelvin.convert(273.15, &celsius).unwrap(), 0.0);
}

#[test]
fn fahrenheit_converts_to_celsius() {
    let fahrenheit = named_unit("degF").unwrap();
    let celsius = named_unit("degC").unwrap();
    // 氷点・沸点・往復で affine 変換の係数とオフセットを網羅する。
    close(fahrenheit.convert(32.0, &celsius).unwrap(), 0.0);
    close(fahrenheit.convert(212.0, &celsius).unwrap(), 100.0);
    close(celsius.convert(100.0, &fahrenheit).unwrap(), 212.0);
}

#[test]
#[should_panic(expected = "scale must be finite and non-zero")]
fn unit_new_rejects_zero_scale() {
    let _ = Unit::new(length(), 0.0, 0.0);
}

#[test]
fn convert_rejects_dimension_mismatch() {
    let meter = named_unit("m").unwrap();
    let second = named_unit("s").unwrap();
    assert_eq!(
        meter.convert(1.0, &second),
        Err(UnitError::DimensionMismatch)
    );
}

#[test]
fn affine_units_cannot_be_multiplied() {
    let celsius = named_unit("degC").unwrap();
    let meter = named_unit("m").unwrap();
    assert_eq!(celsius.try_mul(&meter), Err(UnitError::AffineInArithmetic));
}

#[test]
fn try_mul_reports_scale_overflow_instead_of_panicking() {
    // 回帰: 各オペランドの scale は有限・非ゼロという不変条件を満たしていても、
    // 乗算した結果が inf にオーバーフローすると以前は Unit::new の assert で panic した。
    let huge = Unit::new(length(), 1e200, 0.0);
    assert_eq!(huge.try_mul(&huge), Err(UnitError::ScaleOutOfRange));
}

#[test]
fn try_div_reports_scale_underflow_instead_of_panicking() {
    // 回帰: 除算の結果が 0.0 にアンダーフローすると以前は Unit::new の assert で panic した。
    let tiny = Unit::new(length(), 1e-200, 0.0);
    let huge = Unit::new(length(), 1e200, 0.0);
    assert_eq!(tiny.try_div(&huge), Err(UnitError::ScaleOutOfRange));
}
