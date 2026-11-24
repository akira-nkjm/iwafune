//! # コンパイル時の次元解析
//!
//! 次元を型パラメータに持つ量 [`Qty<D, T>`](Qty) を提供する。加減算は同次元のみ、乗除算は
//! 次元の指数を加減算した次元を返す。指数は**有理数**なので `T^(1/2)` を含む次元
//! （[`AngularRandomWalk`] = `deg/h^(1/2)`、[`AccelerationNoiseDensity`] = `m/s^2/Hz^(1/2)`）も表せる。
//!
//! 次元は [`Length`] / [`Velocity`] / [`Force`] のエイリアスで指す。裏の型レベル表現
//! （typenum ポリフィル）は `#[doc(hidden)]` の中間エイリアスで隠してある。
//!
//! 実行時の [`crate::Dimension`] / [`crate::Quantity`] とは独立の層で、対応は
//! [`Qty::DIM`] が繋ぐ（一致はテストで固定）。
//!
//! ## 対数単位
//!
//! `dB` / `dBm` / `dBW` / `dBi` は SI 値と一次式で対応しないので、Qty の次元には載せない。
//! 出力を対数値のまま表示する専用型は `Decibel`・`DecibelWatt`・`DecibelPerKelvin`・`DecibelHertz`。
//! 実行時の [`crate::Unit`]（[`crate::named_unit`]）で境界の値を SI の線形値に直してから
//! [`Qty::from_raw`] で持ち上げる。
//!
//! ## 次元が守れないもの
//!
//! **[`Energy`] と [`Torque`] は次元が同一**で、`J` と `N*m` の取り違えは次元検査では
//! 弾けない（SI では rad が無次元なため。語彙の中で衝突するのはこの 1 組だけで、
//! そのことはテストで固定してある）。角度が絡む量の使い分けは型ではなく
//! **単位の綴り**と読み手の意図で担保する。
//!
//! ## 脱出口の使い方
//!
//! [`Qty::value`] と [`Qty::from_raw`] は型の保護を外す/かける操作で、**使ってよい場所を
//! 限る**。
//!
//! | 場所 | 可否 |
//! |---|---|
//! | 外部入力（設計データ・API・ファイル）を型付きに持ち上げる境界 | ✅ `from_raw` |
//! | 型付き API から外部（gRPC・OpenAPI DTO・テレメトリ）へ出す境界 | ✅ `value` |
//! | 数値積分・行列演算のホットループ | ✅ 入口で 1 回 `value`、出口で `from_raw` |
//! | `libm` など生 `f64` を取る関数へ渡す直前 | ✅ `value` |
//! | 型付き同士の加減乗除 | ❌ 演算子を使う（脱出口を挟むと次元検査が消える） |
//! | 「型が合わないので通す」ための一時しのぎ | ❌ 次元が違うなら設計が間違っている |
//!
//! **ホットループでは境界で 1 回だけ外す。** ループ内で要素ごとに `value` を呼ぶと、
//! 型で守られない区間が広がるうえ、意図も伝わらない:
//!
//! ```ignore
//! // ✅ 入口で外し、出口で戻す
//! let r = position.value();          // [m]
//! let v = velocity.value();          // [m/s]
//! let (r, v) = integrate(r, v, dt);  // 内側は生 f64
//! Qty::<Length>::from_raw(r)
//! ```
//!
//! `from_raw` は `D` がその値の次元であることを検証しない。**そこが唯一の嘘をつける
//! 場所**なので、境界に集めて目に見えるようにする。
//!
//! Addition and subtraction require identical dimensions:
//!
//! ```compile_fail
//! use sat_units::typed::{Length, Qty, Time};
//!
//! let length: Qty<Length> = Qty::from_raw(1.0);
//! let time: Qty<Time> = Qty::from_raw(1.0);
//! let _invalid = length + time;
//! ```
//!
//! Subtraction and comparison have the same restriction:
//!
//! ```compile_fail
//! use sat_units::typed::{Length, Qty, Time};
//!
//! let length: Qty<Length> = Qty::from_raw(1.0);
//! let time: Qty<Time> = Qty::from_raw(1.0);
//! let _invalid = length - time;
//! ```
//!
//! ```compile_fail
//! use sat_units::typed::{Length, Qty, Time};
//!
//! let length: Qty<Length> = Qty::from_raw(1.0);
//! let time: Qty<Time> = Qty::from_raw(1.0);
//! let _invalid = length < time;
//! ```

use core::cmp::Ordering;
use core::fmt;
use core::marker::PhantomData;
use core::ops::{Add, Div, Mul, Neg, Sub};

use crate::Dimension as RuntimeDimension;

mod decibel;
mod fraction;
#[cfg(feature = "alloc")]
mod named;
pub use crate::dimensions::*;
pub use decibel::{Decibel, DecibelHertz, DecibelPerKelvin, DecibelWatt};
#[cfg(feature = "alloc")]
pub use named::{AbsorbedDoseKind, NamedKind, TemperatureDifferenceKind, TorqueKind};

/// Numeric values supported as quantity scalars.
///
/// Downstream crates may implement this marker for their own numeric types.
pub trait Scalar {}

macro_rules! impl_scalar {
    ($($scalar:ty),+ $(,)?) => {
        $(impl Scalar for $scalar {})+
    };
}

impl_scalar!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
);

/// A scalar value whose physical dimension is checked at compile time.
#[repr(transparent)]
pub struct Qty<D: Dimension, T: Scalar = f64> {
    value: T,
    dimension: PhantomData<fn() -> D>,
}

impl<D: Dimension, T: Scalar> Qty<D, T> {
    /// Runtime metadata for `D`, sourced from the same exponents used by the type algebra.
    pub const DIM: RuntimeDimension = D::DIM;

    /// 生スカラから型付き量を作る（**境界でのみ使う脱出口**）。
    ///
    /// 単位が型で守られていない値——外部入力・生成コードの出力・積分器の内部状態——を
    /// 型付きの世界へ持ち上げる入口。`D` が実際にその値の次元であることは**呼び出し側の
    /// 責任**で、型システムは検証しない。
    ///
    /// 使ってよい場所は [`crate::typed`] の「脱出口の使い方」を参照。
    pub const fn from_raw(value: T) -> Self {
        Self {
            value,
            dimension: PhantomData,
        }
    }

    /// 型付き量から生スカラを取り出す（**境界とホットループでのみ使う脱出口**）。
    ///
    /// 取り出した時点で単位の保護は外れる。ホットループの入口で 1 回だけ呼び、
    /// ループ内は生スカラで回して、出口で [`Qty::from_raw`] に戻すのが想定した使い方。
    ///
    /// 使ってよい場所は [`crate::typed`] の「脱出口の使い方」を参照。
    pub fn value(self) -> T {
        self.value
    }
}

impl<D: Dimension, T: Scalar + Clone> Clone for Qty<D, T> {
    fn clone(&self) -> Self {
        Self::from_raw(self.value.clone())
    }
}

impl<D: Dimension, T: Scalar + Copy> Copy for Qty<D, T> {}

impl<D: Dimension, T: Scalar + fmt::Debug> fmt::Debug for Qty<D, T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("Qty").field(&self.value).finish()
    }
}

impl<D: Dimension, T: Scalar + PartialEq> PartialEq for Qty<D, T> {
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

impl<D: Dimension, T: Scalar + Eq> Eq for Qty<D, T> {}

impl<D: Dimension, T: Scalar + PartialOrd> PartialOrd for Qty<D, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<D: Dimension, T: Scalar + Ord> Ord for Qty<D, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl<D: Dimension, T> Add for Qty<D, T>
where
    T: Scalar + Add<Output = T>,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::from_raw(self.value + rhs.value)
    }
}

impl<D: Dimension, T> Sub for Qty<D, T>
where
    T: Scalar + Sub<Output = T>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::from_raw(self.value - rhs.value)
    }
}

impl<LeftDimension, RightDimension, LeftScalar, RightScalar> Mul<Qty<RightDimension, RightScalar>>
    for Qty<LeftDimension, LeftScalar>
where
    LeftDimension: Dimension + Mul<RightDimension>,
    RightDimension: Dimension,
    <LeftDimension as Mul<RightDimension>>::Output: Dimension,
    LeftScalar: Scalar + Mul<RightScalar>,
    RightScalar: Scalar,
    <LeftScalar as Mul<RightScalar>>::Output: Scalar,
{
    type Output = Qty<
        <LeftDimension as Mul<RightDimension>>::Output,
        <LeftScalar as Mul<RightScalar>>::Output,
    >;

    fn mul(self, rhs: Qty<RightDimension, RightScalar>) -> Self::Output {
        Qty::from_raw(self.value * rhs.value)
    }
}

impl<LeftDimension, RightDimension, LeftScalar, RightScalar> Div<Qty<RightDimension, RightScalar>>
    for Qty<LeftDimension, LeftScalar>
where
    LeftDimension: Dimension + Div<RightDimension>,
    RightDimension: Dimension,
    <LeftDimension as Div<RightDimension>>::Output: Dimension,
    LeftScalar: Scalar + Div<RightScalar>,
    RightScalar: Scalar,
    <LeftScalar as Div<RightScalar>>::Output: Scalar,
{
    type Output = Qty<
        <LeftDimension as Div<RightDimension>>::Output,
        <LeftScalar as Div<RightScalar>>::Output,
    >;

    fn div(self, rhs: Qty<RightDimension, RightScalar>) -> Self::Output {
        Qty::from_raw(self.value / rhs.value)
    }
}

impl<D: Dimension, T, S> Mul<S> for Qty<D, T>
where
    T: Scalar + Mul<S>,
    S: Scalar,
    <T as Mul<S>>::Output: Scalar,
{
    type Output = Qty<D, <T as Mul<S>>::Output>;

    fn mul(self, rhs: S) -> Self::Output {
        Qty::from_raw(self.value * rhs)
    }
}

impl<D: Dimension> Div<f64> for Qty<D, f64> {
    type Output = Qty<D, f64>;

    fn div(self, rhs: f64) -> Self::Output {
        Qty::from_raw(self.value / rhs)
    }
}

macro_rules! impl_scalar_mul_qty {
    ($($scalar:ty),+ $(,)?) => {
        $(
            impl<D: Dimension> Mul<Qty<D, $scalar>> for $scalar {
                type Output = Qty<D, $scalar>;

                fn mul(self, rhs: Qty<D, $scalar>) -> Self::Output {
                    Qty::from_raw(self * rhs.value)
                }
            }
        )+
    };
}

impl_scalar_mul_qty!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
);

impl<D: Dimension, T> Neg for Qty<D, T>
where
    T: Scalar + Neg,
    <T as Neg>::Output: Scalar,
{
    type Output = Qty<D, <T as Neg>::Output>;

    fn neg(self) -> Self::Output {
        Qty::from_raw(-self.value)
    }
}

pub use fraction::Fraction;

impl<D: Dimension> Qty<D, f64> {
    /// Additive identity for any dimension.
    pub const ZERO: Self = Self::from_raw(0.0);

    /// Returns whether the stored scalar is finite without leaving the typed domain.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.value.is_finite()
    }
}

impl Qty<Dimensionless, f64> {
    /// Multiplicative identity for dimensionless ratios.
    pub const ONE: Self = Self::from_raw(1.0);
}

/// JSON では SI のスカラ値（f64）として出し入れする。次元は型が保証する。
#[cfg(feature = "serde")]
impl<D: Dimension> serde::Serialize for Qty<D, f64> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // 出力側が次元を添えられるよう記録する（[`crate::take_last_dimension`]）。JSON は変えない。
        #[cfg(feature = "alloc")]
        crate::last_dimension::record(D::DIM);
        serializer.serialize_f64(self.value)
    }
}

#[cfg(feature = "serde")]
impl<'de, D: Dimension> serde::Deserialize<'de> for Qty<D, f64> {
    fn deserialize<De: serde::Deserializer<'de>>(deserializer: De) -> Result<Self, De::Error> {
        <f64 as serde::Deserialize>::deserialize(deserializer).map(Self::from_raw)
    }
}

/// TS では number（SI 値）として型生成する。
#[cfg(feature = "ts")]
impl<D: Dimension> ts_rs::TS for Qty<D, f64> {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &ts_rs::Config) -> String {
        String::from("number")
    }
    fn inline(cfg: &ts_rs::Config) -> String {
        <Self as ts_rs::TS>::name(cfg)
    }
}
