/// Small normalized rational number for dimension exponents.
///
/// 構築・演算 API は 2 系統ある。混同しないこと:
/// - [`Rational::new`] — `const fn`。**コンパイル時定数専用**（本 crate 内の単位定義テーブル
///   など）。境界超過は panic（ビルドを落として気付ける設計）。
/// - [`Rational::try_new`] / [`Rational::checked_add`] / [`Rational::checked_sub`] /
///   [`Rational::checked_mul`] — 動的な（実行時に決まる）値向け。境界超過は `None` を返し
///   panic しない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rational {
    num: i8,
    den: i8,
}

impl Rational {
    pub const ZERO: Rational = Rational { num: 0, den: 1 };
    pub const ONE: Rational = Rational { num: 1, den: 1 };

    /// `num/den` を既約分数として構築する。
    ///
    /// **`const fn` はコンパイル時定数専用**: `den == 0`・`den == i8::MIN`・既約化後に
    /// 分子/分母が `i8` 範囲を超える入力（例: `Rational::new(i8::MIN, -1)`）は `assert!`
    /// で panic する。これはコンパイル時定数テーブル（本 crate の `voltage()`/`power()`/
    /// `energy()` 等）が誤って範囲外の指数を埋め込んだ場合に**ビルドを落として**気付ける
    /// ようにするための意図的な設計であり、動的な（実行時に決まる）入力には使わないこと。
    /// 実行時の値・外部入力（呼び出し側が動的に構築する分数冪など）には
    /// [`Rational::try_new`] または [`Rational::checked_add`] 等の `checked_*` 系を使う
    /// （こちらは境界超過を `None` で返し panic しない）。
    pub const fn new(num: i8, den: i8) -> Rational {
        assert!(den != 0, "rational denominator must not be zero");
        assert!(
            den != i8::MIN,
            "rational denominator magnitude is too large"
        );

        let mut n = num as i16;
        let mut d = den as i16;
        if d < 0 {
            n = -n;
            d = -d;
        }

        if n == 0 {
            return Rational::ZERO;
        }

        let g = gcd_i16(abs_i16(n), d);
        n /= g;
        d /= g;
        // 既約化後でも i8::MIN/-1 = 128 のように i8 範囲を超える入力があり得る。
        assert!(
            n >= i8::MIN as i16 && n <= i8::MAX as i16,
            "rational numerator out of i8 range after reduction"
        );
        assert!(
            d <= i8::MAX as i16,
            "rational denominator out of i8 range after reduction"
        );
        Rational {
            num: n as i8,
            den: d as i8,
        }
    }

    /// [`Rational::new`] の fallible 版。動的な（コンパイル時定数でない）`num`/`den` を
    /// 扱うときはこちらを使う。既存の `checked_add`/`checked_sub`/`checked_mul` と同じ
    /// 「境界超過は `None`」という設計に揃えており、panic 経路を持たない。
    pub fn try_new(num: i8, den: i8) -> Option<Rational> {
        if den == 0 {
            return None;
        }
        // i8::MIN の絶対値は i8 で表現できないため、i16 へ昇格してから符号正規化する。
        normalize_i16(num as i16, den as i16)
    }

    pub const fn from_int(n: i8) -> Rational {
        Rational { num: n, den: 1 }
    }

    pub const fn numerator(self) -> i8 {
        self.num
    }

    pub const fn denominator(self) -> i8 {
        self.den
    }

    pub fn checked_add(self, rhs: Rational) -> Option<Rational> {
        let num = (self.num as i16)
            .checked_mul(rhs.den as i16)?
            .checked_add((rhs.num as i16).checked_mul(self.den as i16)?)?;
        let den = (self.den as i16).checked_mul(rhs.den as i16)?;
        normalize_i16(num, den)
    }

    pub fn checked_sub(self, rhs: Rational) -> Option<Rational> {
        let num = (self.num as i16)
            .checked_mul(rhs.den as i16)?
            .checked_sub((rhs.num as i16).checked_mul(self.den as i16)?)?;
        let den = (self.den as i16).checked_mul(rhs.den as i16)?;
        normalize_i16(num, den)
    }

    pub fn checked_mul(self, rhs: Rational) -> Option<Rational> {
        let num = (self.num as i16).checked_mul(rhs.num as i16)?;
        let den = (self.den as i16).checked_mul(rhs.den as i16)?;
        normalize_i16(num, den)
    }
}

const fn abs_i16(value: i16) -> i16 {
    if value < 0 { -value } else { value }
}

const fn gcd_i16(mut a: i16, mut b: i16) -> i16 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    if a == 0 { 1 } else { a }
}

fn normalize_i16(mut num: i16, mut den: i16) -> Option<Rational> {
    if den == 0 {
        return None;
    }
    if den < 0 {
        num = num.checked_neg()?;
        den = den.checked_neg()?;
    }
    if num == 0 {
        return Some(Rational::ZERO);
    }

    let gcd = gcd_i16(abs_i16(num), den);
    let num = num / gcd;
    let den = den / gcd;
    if num < i8::MIN as i16 || num > i8::MAX as i16 || den > i8::MAX as i16 {
        return None;
    }
    Some(Rational {
        num: num as i8,
        den: den as i8,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_to_lowest_terms() {
        assert_eq!(Rational::new(2, 4), Rational::new(1, 2));
    }

    #[test]
    fn normalizes_negative_denominator() {
        assert_eq!(Rational::new(1, -2), Rational::new(-1, 2));
    }

    #[test]
    fn normalizes_zero_to_one_denominator() {
        assert_eq!(Rational::new(0, 5), Rational::ZERO);
    }

    #[test]
    fn adds_rationals() {
        let sum = Rational::new(1, 2).checked_add(Rational::new(1, 3));
        assert_eq!(sum, Some(Rational::new(5, 6)));
    }

    #[test]
    fn returns_none_when_result_overflows_i8() {
        let result = Rational::from_int(127).checked_add(Rational::ONE);
        assert_eq!(result, None);
    }

    #[test]
    fn subtracts_rationals() {
        let diff = Rational::new(3, 4).checked_sub(Rational::new(1, 4));
        assert_eq!(diff, Some(Rational::new(1, 2)));
    }

    #[test]
    fn multiplies_rationals() {
        let product = Rational::new(2, 3).checked_mul(Rational::new(3, 4));
        assert_eq!(product, Some(Rational::new(1, 2)));
    }

    #[test]
    fn multiplication_can_overflow_to_none() {
        let result = Rational::from_int(127).checked_mul(Rational::from_int(127));
        assert_eq!(result, None);
    }

    #[test]
    fn try_new_normalizes_like_new() {
        assert_eq!(Rational::try_new(2, 4), Some(Rational::new(1, 2)));
    }

    #[test]
    fn try_new_rejects_zero_denominator() {
        assert_eq!(Rational::try_new(1, 0), None);
    }

    #[test]
    fn try_new_returns_none_instead_of_panicking_on_reduction_overflow() {
        // 回帰: Rational::new(i8::MIN, -1) は既約化後 n=128 となり assert! で panic した。
        // try_new は同じ入力に対し None を返す（checked_* 系と同じ設計）。
        assert_eq!(Rational::try_new(i8::MIN, -1), None);
    }

    #[test]
    fn try_new_rejects_min_magnitude_denominator() {
        assert_eq!(Rational::try_new(1, i8::MIN), None);
    }
}
