//! 対数単位（`dB` / `dBi` / `dBm` / `dBW` / `mag`）の補助。
//!
//! 単位そのものは [`crate::named_unit`] が持つ（真実源は 1 つ）。ここに置くのは、回線計算で
//! 「dBm + dB = dBm」をしたい場面のための**線形値での**補助だけ。対数の値を `+` で足す
//! 操作は単位カーネルの算術には入れないので、境界で SI の線形値に直してから
//! 利得を掛ける。
//!
//! 対数単位は実行時の [`crate::Unit`] にだけ存在し、型レベルの [`crate::typed`] には無い。
//!
//! 等級（`mag`）は dB と同じ流儀で線形の比との往復だけを置く。比 = `10^(-0.4 m)` なので
//! 等級が 1 大きいと約 2.512 倍暗い。どの明るさを 0 等とするか（零点）は単位ではなく
//! 呼び出し側の文脈が持つ。

/// SI の線形値（電力など）に dB の利得を掛ける: `value_si × 10^(gain_db / 10)`。
///
/// 電力比（`10 log10`）の利得を前提にする。`10 dBm` の信号に `3 dB` の利得を掛ける場合は
/// `apply_db_gain(dbm.to_si(10.0), 3.0)` で W が得られ、`dbm.convert` 等で表示単位に戻す。
#[must_use]
pub fn apply_db_gain(value_si: f64, gain_db: f64) -> f64 {
    value_si * db_to_ratio(gain_db)
}

/// 電力比を dB に直す: `10 log10(ratio)`。**dB ⇄ 比の換算はここ（と [`db_to_ratio`]）だけ。**
///
/// 比が正でない（0・負・NaN）と有限にならない（`-inf` / NaN）。検査付きの変換が要るなら
/// `named_unit("one")` から `named_unit("dB")` への [`crate::Unit::convert`] を使う。
#[must_use]
pub fn ratio_to_db(ratio: f64) -> f64 {
    10.0 * libm::log10(ratio)
}

/// dB を電力比に直す: `10^(db / 10)`。[`apply_db_gain`] は `value × db_to_ratio(gain)`。
#[must_use]
pub fn db_to_ratio(db: f64) -> f64 {
    libm::exp10(db / 10.0)
}

/// 等級差を線形の明るさの比に直す: `10^(-0.4 · magnitude)`。
///
/// `magnitude_to_ratio(5.0)` は 0.01（5 等暗いと 1/100）。`named_unit("mag")` の
/// [`crate::Unit::to_si`] と同じ値を返す。
#[must_use]
pub fn magnitude_to_ratio(magnitude: f64) -> f64 {
    libm::exp10(-0.4 * magnitude)
}

/// 線形の明るさの比を等級差に直す: `-2.5 · log10(ratio)`。
///
/// 比が正でない（0・負・NaN）と有限にならない。検査付きの変換が要るなら
/// `named_unit("one")` から `named_unit("mag")` への [`crate::Unit::convert`] を使う。
#[must_use]
pub fn ratio_to_magnitude(ratio: f64) -> f64 {
    -2.5 * libm::log10(ratio)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Dimension, Unit, UnitError, named_unit};

    #[test]
    fn db_and_ratio_round_trip_and_match_the_db_unit() {
        assert!((ratio_to_db(100.0) - 20.0).abs() < 1e-12);
        assert!((db_to_ratio(3.0) - 1.995_262_314_968_879_5).abs() < 1e-12);
        let db = named_unit("dB").expect("dB");
        for r in [1e-3, 0.5, 2.0, 1234.5] {
            assert!((db_to_ratio(ratio_to_db(r)) / r - 1.0).abs() < 1e-12);
            assert!((db.from_si(r) - ratio_to_db(r)).abs() < 1e-12);
        }
        assert!(ratio_to_db(0.0).is_infinite());
    }

    fn close(left: f64, right: f64) {
        assert!(
            (left - right).abs() <= 1e-12 * right.abs().max(1.0),
            "{left} != {right}"
        );
    }

    fn unit(symbol: &str) -> Unit {
        named_unit(symbol).expect(symbol)
    }

    #[test]
    fn dbm_reference_points() {
        let dbm = unit("dBm");
        let watt = unit("W");
        close(dbm.convert(0.0, &watt).unwrap(), 1.0e-3);
        close(dbm.convert(30.0, &watt).unwrap(), 1.0);
        close(dbm.convert(-30.0, &watt).unwrap(), 1.0e-6);
        close(watt.convert(1.0, &dbm).unwrap(), 30.0);
        close(watt.convert(1.0e-3, &dbm).unwrap(), 0.0);
    }

    #[test]
    fn dbm_round_trips_through_watts_and_milliwatts() {
        let dbm = unit("dBm");
        let watt = unit("W");
        let milliwatt = crate::parse_unit("mW").unwrap();
        close(dbm.convert(0.0, &milliwatt).unwrap(), 1.0);
        close(milliwatt.convert(1.0, &dbm).unwrap(), 0.0);
        for value in [-120.0, -3.0, 0.0, 13.5, 47.0] {
            let watts = dbm.convert(value, &watt).unwrap();
            let milliwatts = watt.convert(watts, &milliwatt).unwrap();
            close(milliwatt.convert(milliwatts, &dbm).unwrap(), value);
        }
    }

    #[test]
    fn dbw_and_dbm_differ_by_thirty() {
        let dbw = unit("dBW");
        let dbm = unit("dBm");
        close(dbw.convert(0.0, &unit("W")).unwrap(), 1.0);
        close(dbw.convert(0.0, &dbm).unwrap(), 30.0);
        close(dbm.convert(-10.0, &dbw).unwrap(), -40.0);
    }

    #[test]
    fn db_is_a_dimensionless_power_ratio() {
        let db = unit("dB");
        assert_eq!(db.dimension(), Dimension::DIMENSIONLESS);
        close(db.to_si(10.0), 10.0);
        close(db.to_si(3.0), 1.995_262_314_968_879_5);
        close(db.convert(20.0, &unit("one")).unwrap(), 100.0);
        close(db.convert(-10.0, &unit("%")).unwrap(), 10.0);
        close(unit("one").convert(1000.0, &db).unwrap(), 30.0);
    }

    #[test]
    fn dbi_converts_like_db_but_keeps_its_own_symbol() {
        // 換算は dB と同一。区別は記号（読み手の意図）だけで担保する。
        assert_eq!(unit("dBi"), unit("dB"));
        close(unit("dBi").convert(6.0, &unit("dB")).unwrap(), 6.0);
    }

    #[test]
    fn magnitude_is_a_dimensionless_pogson_ratio() {
        let mag = unit("mag");
        let one = unit("one");
        assert_eq!(mag.dimension(), Dimension::DIMENSIONLESS);
        assert!(mag.is_logarithmic());
        assert_eq!(mag.log_factor(), Some(-2.5));
        // 5 等差 = 100 倍（Pogson の定義）。大きい等級ほど暗い。
        close(mag.convert(5.0, &one).unwrap(), 0.01);
        close(mag.convert(-5.0, &one).unwrap(), 100.0);
        close(mag.convert(0.0, &one).unwrap(), 1.0);
        close(one.convert(0.01, &mag).unwrap(), 5.0);
        close(mag.to_si(1.0), 0.398_107_170_553_497_25);
        // 線形の比が正でなければ等級にできない（dB と同じ検査）。
        assert_eq!(
            one.convert(0.0, &mag),
            Err(UnitError::NonPositiveForLogarithmic)
        );
        // dB とは同じ無次元なので相互に換算できる: 1 等暗い = -4 dB。
        close(mag.convert(1.0, &unit("dB")).unwrap(), -4.0);
    }

    #[test]
    fn magnitude_helpers_match_the_unit() {
        let mag = unit("mag");
        for magnitude in [-1.46, 0.0, 2.5, 6.0, 12.3] {
            close(magnitude_to_ratio(magnitude), mag.to_si(magnitude));
            close(ratio_to_magnitude(magnitude_to_ratio(magnitude)), magnitude);
        }
        close(magnitude_to_ratio(5.0), 0.01);
        close(ratio_to_magnitude(100.0), -5.0);
    }

    #[test]
    fn logarithmic_units_report_their_mapping() {
        for symbol in ["dB", "dBi", "dBm", "dBW"] {
            let unit = unit(symbol);
            assert!(unit.is_logarithmic());
            assert!(!unit.is_affine());
            assert!(!unit.is_linear());
            assert_eq!(unit.log_factor(), Some(10.0));
        }
        close(unit("dBm").scale(), 1.0e-3);
        assert_eq!(unit("W").log_factor(), None);
        assert!(unit("W").is_linear());
        assert!(!unit("degC").is_linear());
    }

    #[test]
    fn power_dimension_mismatch_is_still_rejected() {
        // dB（無次元）と dBm（Power）は次元が違うので相互変換できない。
        assert_eq!(
            unit("dB").convert(3.0, &unit("dBm")),
            Err(UnitError::DimensionMismatch)
        );
        assert_eq!(
            unit("dBm").convert(3.0, &unit("V")),
            Err(UnitError::DimensionMismatch)
        );
    }

    #[test]
    fn non_positive_values_cannot_be_expressed_in_logarithmic_units() {
        // 0 W は -∞ dBm、負の W は対数が無い。convert は非有限値を作らずエラーにする。
        let watt = unit("W");
        let dbm = unit("dBm");
        for value in [0.0, -0.0, -1.0, f64::NAN, f64::NEG_INFINITY] {
            assert_eq!(
                watt.convert(value, &dbm),
                Err(UnitError::NonPositiveForLogarithmic),
                "{value} W"
            );
        }
        // 無次元の 0 も同じ（0 倍の電力比は -∞ dB）。
        assert_eq!(
            unit("one").convert(0.0, &unit("dB")),
            Err(UnitError::NonPositiveForLogarithmic)
        );
        // 素の from_si は IEEE の log10 のまま（検査しない入口であることを固定する）。
        assert_eq!(dbm.from_si(0.0), f64::NEG_INFINITY);
        assert!(dbm.from_si(-1.0).is_nan());
        // 逆向き（-∞ dBm → 0 W）は線形単位への変換なので通る。
        assert_eq!(dbm.convert(f64::NEG_INFINITY, &watt), Ok(0.0));
    }

    #[test]
    fn logarithmic_units_cannot_enter_arithmetic() {
        let dbm = unit("dBm");
        let db = unit("dB");
        let hertz = unit("Hz");
        assert_eq!(dbm.try_div(&hertz), Err(UnitError::LogarithmicInArithmetic));
        assert_eq!(hertz.try_mul(&db), Err(UnitError::LogarithmicInArithmetic));
        assert_eq!(dbm.try_mul(&db), Err(UnitError::LogarithmicInArithmetic));
        // affine と対数が両方なら affine を先に報告する（既存の振る舞いを変えない）。
        assert_eq!(
            unit("degC").try_mul(&db),
            Err(UnitError::AffineInArithmetic)
        );
    }

    #[test]
    fn db_gain_is_applied_in_linear_power() {
        let dbm = unit("dBm");
        let watt = unit("W");
        // 10 dBm + 3 dB ≈ 13 dBm を線形値で行う。
        let amplified = apply_db_gain(dbm.to_si(10.0), 3.0);
        close(watt.convert(amplified, &dbm).unwrap(), 13.0);
        close(apply_db_gain(1.0, -10.0), 0.1);
        close(apply_db_gain(2.0, 0.0), 2.0);
    }

    #[test]
    #[should_panic(expected = "logarithmic reference must be finite and positive")]
    fn logarithmic_rejects_non_positive_reference() {
        let _ = Unit::logarithmic(Dimension::DIMENSIONLESS, 0.0, 10.0);
    }

    #[test]
    #[should_panic(expected = "logarithmic factor must be finite and non-zero")]
    fn logarithmic_rejects_zero_factor() {
        let _ = Unit::logarithmic(Dimension::DIMENSIONLESS, 1.0, 0.0);
    }
}
