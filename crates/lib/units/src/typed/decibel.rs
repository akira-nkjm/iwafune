//! dB 出力の表示種別。中身は対数値そのもの（SI の線形値ではない）。

macro_rules! decibel {
    ($name:ident, $doc:literal, $kind:ident) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
        pub struct $name(f64);

        impl $name {
            /// 次元名の代わりに記録する表示種別。
            pub const KIND: &'static str = crate::display_kind::$kind;

            /// 既に対数の値から作る。換算しない。
            #[must_use]
            pub const fn new(value: f64) -> Self {
                Self(value)
            }

            /// 対数値を取り出す。
            #[must_use]
            pub const fn value(self) -> f64 {
                self.0
            }
        }

        impl From<f64> for $name {
            fn from(value: f64) -> Self {
                Self(value)
            }
        }

        #[cfg(feature = "serde")]
        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                #[cfg(feature = "alloc")]
                crate::last_dimension::record_kind(Self::KIND);
                serializer.serialize_f64(self.0)
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                <f64 as serde::Deserialize>::deserialize(deserializer).map(Self)
            }
        }

        #[cfg(feature = "ts")]
        impl ts_rs::TS for $name {
            type WithoutGenerics = Self;
            type OptionInnerType = Self;
            fn name(_: &ts_rs::Config) -> String {
                String::from("number")
            }
            fn inline(cfg: &ts_rs::Config) -> String {
                <Self as ts_rs::TS>::name(cfg)
            }
        }
    };
}

decibel!(Decibel, "dB の出力値。", DECIBEL);
decibel!(DecibelWatt, "dBW の出力値。", DECIBEL_WATT);
decibel!(DecibelPerKelvin, "dB/K の出力値。", DECIBEL_PER_KELVIN);
decibel!(DecibelHertz, "dB·Hz の出力値。", DECIBEL_HERTZ);
