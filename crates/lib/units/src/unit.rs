use crate::{DimError, Dimension};

/// 単位。SI 値との対応は 2 種類ある（[`Unit::to_si`] / [`Unit::from_si`]）:
///
/// - **一次式**: `SI = scale × x + offset`。`offset ≠ 0` のもの（`degC`/`degF`）が affine。
/// - **対数**: `SI = scale × 10^(x / factor)`。`dB`/`dBi`（無次元）・`dBm`/`dBW`（Power）・
///   `mag`（無次元。`factor = -2.5` なので比 = `10^(-0.4 x)`）。
///   `scale` は 0 dB に当たる基準値（`dBm` なら 1 mW = 1e-3 W）。
///
/// affine と対数は**単独でのみ有効**で、積・商・冪・接頭辞・複合式には入れない。
/// 境界で [`Unit::to_si`] によって SI の線形値に直してから計算する。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Unit {
    dimension: Dimension,
    scale: f64,
    offset: f64,
    mapping: Mapping,
}

/// 表示値 x と SI 値の対応の種類。
///
/// 対数の `factor` は電力比の 10（`10 log10`）。振幅比（`20 log10`）は今回は単位を
/// 定義していないが、`factor = 20` を渡せば同じ構造で表せる。
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mapping {
    Linear,
    Logarithmic { factor: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnitError {
    DimensionMismatch,
    AffineInArithmetic,
    /// 対数単位（`dB`/`dBm` 等）を積・商・冪・接頭辞・複合式に入れた。
    /// 対数の値どうしの和は線形の積であり、一次式の単位代数では意味が変わるため拒否する。
    LogarithmicInArithmetic,
    /// 対数単位へ変換する SI 値が正でない（0・負・NaN）。`10 log10` が有限にならない。
    NonPositiveForLogarithmic,
    UnknownUnit,
    /// `krad` / `Mrad` のように、倍率が 1 より大きい接頭辞を `rad` に付けた。
    /// 放射線分野の `krad`（1000 rad = 10 Gy、旧吸収線量単位）と角度の区別が付かないため拒否する。
    /// `mrad` / `urad` のような倍率 < 1 の接頭辞は角度として受ける。
    AmbiguousRadian,
    /// 単位式の冪に小数（`Hz^0.5`）を書いた。分数は括弧付きの `^(1/2)` で書く。
    DecimalExponent,
    UnknownDimension,
    Dim(DimError),
    /// try_mul/try_div の結果 scale が非有限またはゼロ（オーバー/アンダーフロー）。
    ScaleOutOfRange,
}

impl core::fmt::Display for UnitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            UnitError::DimensionMismatch => write!(f, "dimension mismatch"),
            UnitError::AffineInArithmetic => write!(f, "affine unit used in arithmetic"),
            UnitError::LogarithmicInArithmetic => {
                write!(f, "logarithmic unit used in arithmetic")
            }
            UnitError::NonPositiveForLogarithmic => {
                write!(f, "value must be positive to express in a logarithmic unit")
            }
            UnitError::UnknownUnit => write!(f, "unknown unit"),
            UnitError::UnknownDimension => write!(f, "unknown dimension"),
            UnitError::AmbiguousRadian => write!(
                f,
                "rad with a prefix larger than 1 is ambiguous: \
                 for radiation dose write Gy (1 krad = 10 Gy); for angles write rad or deg"
            ),
            UnitError::DecimalExponent => write!(
                f,
                "decimal exponents are not supported: write a fraction such as ^(1/2)"
            ),
            UnitError::Dim(error) => write!(f, "dimension error: {error:?}"),
            UnitError::ScaleOutOfRange => {
                write!(f, "resulting unit scale must be finite and non-zero")
            }
        }
    }
}

// no_std でも core::error::Error(Rust 1.81+)で error 型として公開できる。
impl core::error::Error for UnitError {}

impl Unit {
    /// 単位を構築する。const テーブル（`named_unit` の各単位）を作れるよう const fn。
    ///
    /// **不変条件**: `scale` は有限かつ非ゼロでなければならない（`from_si` は `scale` で
    /// 除算する）。const テーブルと `parse_unit`/`named_unit` 経由の構築は常にこれを満たす。
    /// 外部から直接 0/NaN/Inf を渡すのは契約違反（変換結果が NaN/Inf に静かに伝播する）。
    /// 不変条件は `assert!` で強制する: const 文脈ではコンパイル時に、実行時呼出では panic で弾く。
    pub const fn new(dimension: Dimension, scale: f64, offset: f64) -> Unit {
        assert!(
            scale.is_finite() && scale != 0.0,
            "Unit scale must be finite and non-zero"
        );
        Unit {
            dimension,
            scale,
            offset,
            mapping: Mapping::Linear,
        }
    }

    /// 対数単位を構築する: `SI = reference × 10^(x / factor)`。
    ///
    /// **不変条件**: `reference` は有限かつ正、`factor` は有限かつ非ゼロ。`reference` は
    /// 0 dB に当たる SI 値（`dBm` なら 1e-3 W）で、[`Unit::scale`] はこの値を返す。
    /// `factor` が負なのは等級（`mag`、`-2.5`）で、値が大きいほど比が小さい。違反は
    /// `Unit::new` と同じく `assert!` で弾く（const 文脈ではコンパイル時）。
    pub const fn logarithmic(dimension: Dimension, reference: f64, factor: f64) -> Unit {
        assert!(
            reference.is_finite() && reference > 0.0,
            "logarithmic reference must be finite and positive"
        );
        assert!(
            factor.is_finite() && factor != 0.0,
            "logarithmic factor must be finite and non-zero"
        );
        Unit {
            dimension,
            scale: reference,
            offset: 0.0,
            mapping: Mapping::Logarithmic { factor },
        }
    }

    pub fn dimension(&self) -> Dimension {
        self.dimension
    }

    pub fn scale(&self) -> f64 {
        self.scale
    }

    pub fn offset(&self) -> f64 {
        self.offset
    }

    pub fn is_affine(&self) -> bool {
        self.offset != 0.0
    }

    /// 対数単位（`dB`/`dBi`/`dBm`/`dBW`/`mag`）か。
    pub fn is_logarithmic(&self) -> bool {
        matches!(self.mapping, Mapping::Logarithmic { .. })
    }

    /// 対数単位なら `10 log10` の 10（等級なら `-2.5`）のような係数を返す。一次式の単位は `None`。
    pub fn log_factor(&self) -> Option<f64> {
        match self.mapping {
            Mapping::Linear => None,
            Mapping::Logarithmic { factor } => Some(factor),
        }
    }

    /// 一次式（offset 無し）で、算術にそのまま入れてよい単位か。
    ///
    /// 値の加減乗除を自前で行う側は `is_affine` ではなく
    /// これで判定する。対数単位は offset が 0 なので `is_affine` では弾けない。
    pub fn is_linear(&self) -> bool {
        !self.is_affine() && !self.is_logarithmic()
    }

    /// 表示値を SI 値に直す。
    pub fn to_si(&self, value: f64) -> f64 {
        match self.mapping {
            Mapping::Linear => value * self.scale + self.offset,
            Mapping::Logarithmic { factor } => self.scale * libm::exp10(value / factor),
        }
    }

    /// SI 値を表示値に直す。
    ///
    /// 対数単位では `si = 0` が `-∞`、負・NaN が NaN になる（`log10` の IEEE の振る舞い）。
    /// 黙って非有限値を流さないため、検査付きの変換は [`Unit::convert`] を使う。
    pub fn from_si(&self, si: f64) -> f64 {
        match self.mapping {
            Mapping::Linear => (si - self.offset) / self.scale,
            Mapping::Logarithmic { factor } => factor * libm::log10(si / self.scale),
        }
    }

    /// 単位間で値を変換する（SI の線形値を経由する）。
    ///
    /// 変換先が対数単位で SI 値が正でない（0 W・負の W・NaN）なら
    /// [`UnitError::NonPositiveForLogarithmic`]。`-∞ dBm` のような非有限値を作らない。
    pub fn convert(&self, value: f64, to: &Unit) -> Result<f64, UnitError> {
        if !self.dimension.is_compatible_with(to.dimension) {
            return Err(UnitError::DimensionMismatch);
        }
        let si = self.to_si(value);
        // NaN も拾うため「正である」を明示的に否定する（`si <= 0.0` だと NaN が素通りする）。
        let positive = si.partial_cmp(&0.0) == Some(core::cmp::Ordering::Greater);
        if to.is_logarithmic() && !positive {
            return Err(UnitError::NonPositiveForLogarithmic);
        }
        Ok(to.from_si(si))
    }

    pub fn try_mul(&self, rhs: &Unit) -> Result<Unit, UnitError> {
        self.ensure_linear(rhs)?;
        let dimension = self
            .dimension
            .try_mul(rhs.dimension)
            .map_err(UnitError::Dim)?;
        let scale = checked_scale(self.scale * rhs.scale)?;
        Ok(Unit::new(dimension, scale, 0.0))
    }

    pub fn try_div(&self, rhs: &Unit) -> Result<Unit, UnitError> {
        self.ensure_linear(rhs)?;
        let dimension = self
            .dimension
            .try_div(rhs.dimension)
            .map_err(UnitError::Dim)?;
        let scale = checked_scale(self.scale / rhs.scale)?;
        Ok(Unit::new(dimension, scale, 0.0))
    }

    pub(crate) fn with_scale(self, factor: f64) -> Unit {
        Unit::new(self.dimension, self.scale * factor, self.offset)
    }

    fn ensure_linear(&self, rhs: &Unit) -> Result<(), UnitError> {
        if self.is_affine() || rhs.is_affine() {
            return Err(UnitError::AffineInArithmetic);
        }
        if self.is_logarithmic() || rhs.is_logarithmic() {
            return Err(UnitError::LogarithmicInArithmetic);
        }
        Ok(())
    }
}

/// `Unit::new` の不変条件（有限・非ゼロ）を事前に検査する。
///
/// `try_mul`/`try_div` は fallible API（`try_` 接頭）でありながら、両オペランドが
/// 不変条件を満たしていても乗算のオーバーフロー・除算のアンダーフローで結果 scale が
/// inf/0.0 になりうる。そのまま `Unit::new` に渡すと assert で panic するため、
/// ここで検査して `UnitError::ScaleOutOfRange` を返す。
fn checked_scale(scale: f64) -> Result<f64, UnitError> {
    if scale.is_finite() && scale != 0.0 {
        Ok(scale)
    } else {
        Err(UnitError::ScaleOutOfRange)
    }
}

mod named;

pub use named::{NAMED_UNIT_SYMBOLS, named_unit};

#[cfg(test)]
mod tests;
