//! 質量行列の 2 つの形（対角の集中質量／一般の対称行列）を 1 つの型で扱う。

use super::SubspaceError;
use crate::math::skyline::{Skyline, SkylineError};

/// 質量行列 M（対称正定値）。
#[derive(Debug, Clone, Copy)]
pub enum Mass<'a> {
    /// 対角の集中質量（対角成分の並び）。
    Diagonal(&'a [f64]),
    /// 小さな対称ブロックを持つ一般の質量（K のプロファイルに収まること）。
    Skyline(&'a Skyline),
}

impl Mass<'_> {
    /// 次数。
    pub fn dim(&self) -> usize {
        match self {
            Mass::Diagonal(d) => d.len(),
            Mass::Skyline(s) => s.dim(),
        }
    }

    /// `y = M x`。
    pub fn mul_vec(&self, x: &[f64], y: &mut [f64]) -> Result<(), SubspaceError> {
        match self {
            Mass::Diagonal(d) => {
                if x.len() != d.len() || y.len() != d.len() {
                    return Err(SubspaceError::Mass(SkylineError::SizeMismatch {
                        expected: d.len(),
                        got: x.len().min(y.len()),
                    }));
                }
                y.iter_mut()
                    .zip(d.iter().zip(x))
                    .for_each(|(o, (m, v))| *o = m * v);
                Ok(())
            }
            Mass::Skyline(s) => s.mul_vec(x, y).map_err(SubspaceError::Mass),
        }
    }

    /// `target += scale · M`。
    pub fn add_scaled_into(&self, target: &mut Skyline, scale: f64) -> Result<(), SubspaceError> {
        match self {
            Mass::Diagonal(d) => {
                for (i, m) in d.iter().enumerate() {
                    target.add(i, i, scale * m).map_err(SubspaceError::Mass)?;
                }
                Ok(())
            }
            Mass::Skyline(s) => target.add_scaled(s, scale).map_err(SubspaceError::Mass),
        }
    }
}
