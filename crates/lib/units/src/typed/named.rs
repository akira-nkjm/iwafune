//! 表示の名前を型から取る（`quantity_text::<Power>`・`Channel::of::<Power>`）。
//! 次元名を文字列で書くと綴りの誤りが実行時まで分からないので、名前は型に持たせる。

use super::{Decibel, DecibelHertz, DecibelPerKelvin, DecibelWatt, Dimension, Fraction};

/// 表示の名前（次元名か表示の種類の名前）を持つ型。整形（[`crate::format::format_quantity`]）や
/// 時系列の次元に渡す名前を、文字列で書かずに型から取る。
///
/// 次元の型（`Power` など）・[`Fraction`]・dB の種類・[`TorqueKind`] が実装する。
/// **[`super::Torque`] は [`super::Energy`] と同じ型**なので、次元の型からは `Energy` になる。
/// 力のモーメントとして表示する（N·m で出す）ときは [`TorqueKind`] を使う。
pub trait NamedKind {
    /// 名前（[`crate::named_dimensions`] の綴りか、[`crate::display_kind`] の名前）。
    fn name() -> &'static str;
}

/// 次元の型は次元名（[`crate::format::dimension_name`]。名前の無い次元は空文字列で、整形は数だけになる）。
impl<D: Dimension> NamedKind for D {
    fn name() -> &'static str {
        crate::format::dimension_name(D::DIM).unwrap_or("")
    }
}

/// 力のモーメント（`Torque`）の表示の名前。次元の型 [`super::Torque`] は [`super::Energy`] と区別できないので、
/// 名前だけを持つ型で指す（`quantity_text::<TorqueKind>`）。
pub enum TorqueKind {}

impl NamedKind for TorqueKind {
    fn name() -> &'static str {
        "Torque"
    }
}

impl TorqueKind {
    /// トルクの出力欄を SI 数値のまま書き出し、次元の代わりに量の種類を記録する。
    /// `#[serde(serialize_with = "sat_units::typed::TorqueKind::serialize")]` で指定する。
    #[cfg(feature = "serde")]
    pub fn serialize<S: serde::Serializer>(
        value: &super::Qty<super::Torque>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        crate::last_dimension::record_kind(Self::name());
        serializer.serialize_f64(value.value())
    }
}

/// 温度差（`TemperatureDifference`）の表示の名前。次元の型 [`super::TemperatureDifference`] は [`super::Temperature`] と
/// 区別できないので、名前だけを持つ型で指す（`quantity_text::<TemperatureDifferenceKind>`。K で出し、273.15 を引かない）。
pub enum TemperatureDifferenceKind {}

impl NamedKind for TemperatureDifferenceKind {
    fn name() -> &'static str {
        "TemperatureDifference"
    }
}

impl TemperatureDifferenceKind {
    /// 温度差の出力欄を SI 数値のまま書き出し、次元の代わりに量の種類を記録する。
    /// `#[serde(serialize_with = "sat_units::typed::TemperatureDifferenceKind::serialize")]` で指定する。
    #[cfg(feature = "serde")]
    pub fn serialize<S: serde::Serializer>(
        value: &super::Qty<super::TemperatureDifference>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        crate::last_dimension::record_kind(Self::name());
        serializer.serialize_f64(value.value())
    }

    /// 省略できる温度差の欄（`Option`）。値があるときだけ種類を記録し、無ければ null。
    #[cfg(feature = "serde")]
    pub fn serialize_option<S: serde::Serializer>(
        value: &Option<super::Qty<super::TemperatureDifference>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(v) => Self::serialize(v, serializer),
            None => serializer.serialize_none(),
        }
    }
}

macro_rules! kind_name {
    ($($t:ty),+ $(,)?) => {
        $(impl NamedKind for $t {
            fn name() -> &'static str {
                <$t>::KIND
            }
        })+
    };
}

kind_name!(
    Fraction,
    Decibel,
    DecibelWatt,
    DecibelPerKelvin,
    DecibelHertz
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typed::{AngularMomentum, Energy, Length, Power, Temperature, Torque};

    #[test]
    fn names_come_from_the_types() {
        assert_eq!(<Power as NamedKind>::name(), "Power");
        assert_eq!(<Length as NamedKind>::name(), "Length");
        assert_eq!(<AngularMomentum as NamedKind>::name(), "AngularMomentum");
        // Torque は Energy と同じ型なので次元の型からは Energy。力のモーメントは TorqueKind
        assert_eq!(<Torque as NamedKind>::name(), "Energy");
        assert_eq!(<Energy as NamedKind>::name(), "Energy");
        assert_eq!(<TorqueKind as NamedKind>::name(), "Torque");
        // 温度差も同じ（次元の型からは Temperature）
        assert_eq!(<Temperature as NamedKind>::name(), "Temperature");
        assert_eq!(
            <TemperatureDifferenceKind as NamedKind>::name(),
            "TemperatureDifference"
        );
        assert_eq!(<Fraction as NamedKind>::name(), "Fraction");
        assert_eq!(<Decibel as NamedKind>::name(), "Decibel");
        assert_eq!(<DecibelHertz as NamedKind>::name(), "DecibelHertz");
    }

    #[test]
    fn every_named_dimension_round_trips() {
        // 名前の一覧のどれもが、正規の名前として整形に渡せる（次元の型から取った名前が整形で通る）
        for (name, _) in crate::named_dimensions() {
            assert!(
                crate::format::canonical_dimension_name(name).is_some(),
                "{name}"
            );
        }
    }
}
