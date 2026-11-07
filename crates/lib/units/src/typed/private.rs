//! 次元の型レベルの表現（`Dim`・指数の有理数 `Rat`）と封印のトレイト。外からは名前付きの別名（`aliases`）だけを使う。

use core::marker::PhantomData;
use core::ops::{Add, Div, Mul, Sub};

use typenum::{Diff, Gcf, Integer, Prod, Quot, Sum};

use super::{Rational, RuntimeDimension};

pub trait Sealed {}

pub trait Exponent {
    const VALUE: Rational;
}

pub struct Rat<N, D>(PhantomData<(N, D)>);

impl<N, D> Exponent for Rat<N, D>
where
    N: Integer,
    D: Integer,
{
    const VALUE: Rational = {
        assert!(N::I16 >= i8::MIN as i16 && N::I16 <= i8::MAX as i16);
        assert!(D::I16 > 0 && D::I16 <= i8::MAX as i16);
        Rational::new(N::I16 as i8, D::I16 as i8)
    };
}

pub trait Normalize {
    type Output;
}

impl<N, D> Normalize for Rat<N, D>
where
    N: Integer + typenum::Gcd<D>,
    D: Integer,
    Gcf<N, D>: Integer,
    N: Div<Gcf<N, D>>,
    D: Div<Gcf<N, D>>,
    Quot<N, Gcf<N, D>>: Integer,
    Quot<D, Gcf<N, D>>: Integer,
{
    type Output = Rat<Quot<N, Gcf<N, D>>, Quot<D, Gcf<N, D>>>;
}

impl<LeftNum, LeftDen, RightNum, RightDen> Add<Rat<RightNum, RightDen>> for Rat<LeftNum, LeftDen>
where
    LeftNum: Integer + Mul<RightDen>,
    LeftDen: Integer + Mul<RightNum> + Mul<RightDen>,
    RightNum: Integer,
    RightDen: Integer,
    Prod<LeftNum, RightDen>: Add<Prod<LeftDen, RightNum>>,
    Sum<Prod<LeftNum, RightDen>, Prod<LeftDen, RightNum>>: Integer,
    Prod<LeftDen, RightDen>: Integer,
    Rat<Sum<Prod<LeftNum, RightDen>, Prod<LeftDen, RightNum>>, Prod<LeftDen, RightDen>>: Normalize,
{
    type Output = <Rat<
        Sum<Prod<LeftNum, RightDen>, Prod<LeftDen, RightNum>>,
        Prod<LeftDen, RightDen>,
    > as Normalize>::Output;

    fn add(self, _rhs: Rat<RightNum, RightDen>) -> Self::Output {
        unreachable!()
    }
}

impl<LeftNum, LeftDen, RightNum, RightDen> Sub<Rat<RightNum, RightDen>> for Rat<LeftNum, LeftDen>
where
    LeftNum: Integer + Mul<RightDen>,
    LeftDen: Integer + Mul<RightNum> + Mul<RightDen>,
    RightNum: Integer,
    RightDen: Integer,
    Prod<LeftNum, RightDen>: Sub<Prod<LeftDen, RightNum>>,
    Diff<Prod<LeftNum, RightDen>, Prod<LeftDen, RightNum>>: Integer,
    Prod<LeftDen, RightDen>: Integer,
    Rat<Diff<Prod<LeftNum, RightDen>, Prod<LeftDen, RightNum>>, Prod<LeftDen, RightDen>>: Normalize,
{
    type Output = <Rat<
        Diff<Prod<LeftNum, RightDen>, Prod<LeftDen, RightNum>>,
        Prod<LeftDen, RightDen>,
    > as Normalize>::Output;

    fn sub(self, _rhs: Rat<RightNum, RightDen>) -> Self::Output {
        unreachable!()
    }
}

pub struct Dim<Length, Mass, Time, Current, Temperature, Amount, Luminosity, Angle, Information> {
    length: PhantomData<Length>,
    mass: PhantomData<Mass>,
    time: PhantomData<Time>,
    current: PhantomData<Current>,
    temperature: PhantomData<Temperature>,
    amount: PhantomData<Amount>,
    luminosity: PhantomData<Luminosity>,
    angle: PhantomData<Angle>,
    information: PhantomData<Information>,
}

impl<Length, Mass, Time, Current, Temperature, Amount, Luminosity, Angle, Information> Sealed
    for Dim<Length, Mass, Time, Current, Temperature, Amount, Luminosity, Angle, Information>
where
    Length: Exponent,
    Mass: Exponent,
    Time: Exponent,
    Current: Exponent,
    Temperature: Exponent,
    Amount: Exponent,
    Luminosity: Exponent,
    Angle: Exponent,
    Information: Exponent,
{
}

impl<Length, Mass, Time, Current, Temperature, Amount, Luminosity, Angle, Information>
    super::Dimension
    for Dim<Length, Mass, Time, Current, Temperature, Amount, Luminosity, Angle, Information>
where
    Length: Exponent,
    Mass: Exponent,
    Time: Exponent,
    Current: Exponent,
    Temperature: Exponent,
    Amount: Exponent,
    Luminosity: Exponent,
    Angle: Exponent,
    Information: Exponent,
{
    const DIM: RuntimeDimension = RuntimeDimension::from_exponents([
        Length::VALUE,
        Mass::VALUE,
        Time::VALUE,
        Current::VALUE,
        Temperature::VALUE,
        Amount::VALUE,
        Luminosity::VALUE,
        Angle::VALUE,
        Information::VALUE,
    ]);
}

impl<L1, M1, T1, I1, K1, N1, J1, A1, Q1, L2, M2, T2, I2, K2, N2, J2, A2, Q2>
    Mul<Dim<L2, M2, T2, I2, K2, N2, J2, A2, Q2>> for Dim<L1, M1, T1, I1, K1, N1, J1, A1, Q1>
where
    L1: Add<L2>,
    M1: Add<M2>,
    T1: Add<T2>,
    I1: Add<I2>,
    K1: Add<K2>,
    N1: Add<N2>,
    J1: Add<J2>,
    A1: Add<A2>,
    Q1: Add<Q2>,
    Sum<L1, L2>: Exponent,
    Sum<M1, M2>: Exponent,
    Sum<T1, T2>: Exponent,
    Sum<I1, I2>: Exponent,
    Sum<K1, K2>: Exponent,
    Sum<N1, N2>: Exponent,
    Sum<J1, J2>: Exponent,
    Sum<A1, A2>: Exponent,
    Sum<Q1, Q2>: Exponent,
{
    type Output = Dim<
        Sum<L1, L2>,
        Sum<M1, M2>,
        Sum<T1, T2>,
        Sum<I1, I2>,
        Sum<K1, K2>,
        Sum<N1, N2>,
        Sum<J1, J2>,
        Sum<A1, A2>,
        Sum<Q1, Q2>,
    >;

    fn mul(self, _rhs: Dim<L2, M2, T2, I2, K2, N2, J2, A2, Q2>) -> Self::Output {
        unreachable!()
    }
}

impl<L1, M1, T1, I1, K1, N1, J1, A1, Q1, L2, M2, T2, I2, K2, N2, J2, A2, Q2>
    Div<Dim<L2, M2, T2, I2, K2, N2, J2, A2, Q2>> for Dim<L1, M1, T1, I1, K1, N1, J1, A1, Q1>
where
    L1: Sub<L2>,
    M1: Sub<M2>,
    T1: Sub<T2>,
    I1: Sub<I2>,
    K1: Sub<K2>,
    N1: Sub<N2>,
    J1: Sub<J2>,
    A1: Sub<A2>,
    Q1: Sub<Q2>,
    Diff<L1, L2>: Exponent,
    Diff<M1, M2>: Exponent,
    Diff<T1, T2>: Exponent,
    Diff<I1, I2>: Exponent,
    Diff<K1, K2>: Exponent,
    Diff<N1, N2>: Exponent,
    Diff<J1, J2>: Exponent,
    Diff<A1, A2>: Exponent,
    Diff<Q1, Q2>: Exponent,
{
    type Output = Dim<
        Diff<L1, L2>,
        Diff<M1, M2>,
        Diff<T1, T2>,
        Diff<I1, I2>,
        Diff<K1, K2>,
        Diff<N1, N2>,
        Diff<J1, J2>,
        Diff<A1, A2>,
        Diff<Q1, Q2>,
    >;

    fn div(self, _rhs: Dim<L2, M2, T2, I2, K2, N2, J2, A2, Q2>) -> Self::Output {
        unreachable!()
    }
}
