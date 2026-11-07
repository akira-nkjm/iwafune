//! 比（0〜1 の割合）。JSON では数値のまま出し入れし、表示は % にする。

/// 比（DoD・SOC・日照率・稼働率・余裕などの 0〜1 の割合）。値は 1 = 100 %。
///
/// 中身は無次元の数（[`super::Qty<super::Dimensionless>`] と同じ）だが、表示を % にしたい量に使う。
/// JSON では数値（`0.25`）として出し入れし、書き出すときに表示の種類 [`Fraction::KIND`] を
/// 記録する（[`crate::take_last_dimension`]）。整形は [`crate::format::format_quantity`] に
/// `Fraction::KIND` を渡すと % で出る。
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Fraction(f64);

impl Fraction {
    /// 表示の種類の名前（次元名の代わりに使う。次元は無次元）。
    pub const KIND: &'static str = crate::display_kind::FRACTION;

    /// 割合 `value`（1 = 100 %）から作る。範囲は検査しない（余裕は負にも 1 超にもなる）。
    #[must_use]
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    /// 割合の値（1 = 100 %）。
    #[must_use]
    pub const fn value(self) -> f64 {
        self.0
    }
}

impl From<f64> for Fraction {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Fraction {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // 出力側が % で表示できるよう種類を記録する。JSON は数値のまま。
        #[cfg(feature = "alloc")]
        crate::last_dimension::record_kind(Self::KIND);
        serializer.serialize_f64(self.0)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Fraction {
    fn deserialize<De: serde::Deserializer<'de>>(deserializer: De) -> Result<Self, De::Error> {
        <f64 as serde::Deserialize>::deserialize(deserializer).map(Self)
    }
}

/// TS では number（1 = 100 %）として型生成する。
#[cfg(feature = "ts")]
impl ts_rs::TS for Fraction {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &ts_rs::Config) -> String {
        String::from("number")
    }
    fn inline(cfg: &ts_rs::Config) -> String {
        <Self as ts_rs::TS>::name(cfg)
    }
}
