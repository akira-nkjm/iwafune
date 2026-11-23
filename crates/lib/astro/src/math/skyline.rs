//! 対称行列のスカイライン（包絡線・可変バンド）格納と LDLᵀ 分解。
//!
//! 有限要素の剛性行列のように、各列の先頭の非零行が列ごとに違う疎な対称行列を、
//! 上三角を列ごとに「先頭の非零行から対角まで」だけ詰めて持つ。
//! 分解は A = Uᵀ D U（U は単位上三角）で、ピボットの入れ替えはしない。
//! 零または負のピボットは panic ではなく [`SkylineError`] で返し、
//! 負のピボットの個数（Sylvester の慣性則による負の固有値の個数）は [`SkylineLdl::negative_pivots`] で数えられる。

use thiserror::Error;

/// ピボットを零とみなす相対しきい値（元の対角成分の絶対値に対する比）。
pub const ZERO_PIVOT_RELATIVE: f64 = 1e-12;

/// スカイライン行列の組み立て・分解・求解の失敗。
#[derive(Debug, Clone, PartialEq, Error)]
pub enum SkylineError {
    /// 列の先頭行が対角より下にある（`first_rows[col] > col`）。
    #[error("Column {col} starts below its diagonal (first row {first_row})")]
    BadProfile { col: usize, first_row: usize },
    /// 要素 (row, col) がプロファイルの外（包絡線より上）にある。
    #[error("Entry ({row}, {col}) lies outside the skyline profile")]
    OutsideProfile { row: usize, col: usize },
    /// ベクトルの長さや 2 つの行列の大きさが合わない。
    #[error("Size mismatch: expected {expected}, got {got}")]
    SizeMismatch { expected: usize, got: usize },
    /// 値が有限でない。
    #[error("A non-finite value was given")]
    NotFinite,
    /// ピボットが（相対的に）零で、分解を続けられない。
    #[error("Zero pivot {pivot:e} at row {index}: the matrix is singular")]
    ZeroPivot { index: usize, pivot: f64 },
    /// 正定値を求める分解で、ピボットが負になった。
    #[error("Non-positive pivot {pivot:e} at row {index}: the matrix is not positive definite")]
    NonPositivePivot { index: usize, pivot: f64 },
}

/// 対称行列のスカイライン格納（分解前。組み立てと行列ベクトル積ができる）。
///
/// 列 j は行 `first_rows[j]..=j` の要素を持ち、最後が対角成分。
#[derive(Debug, Clone, PartialEq)]
pub struct Skyline {
    n: usize,
    first: Vec<usize>,
    ptr: Vec<usize>,
    data: Vec<f64>,
}

/// 分解済みの行列 A = Uᵀ D U。対角に D、対角より上に U（単位対角は持たない）が入る。
#[derive(Debug, Clone, PartialEq)]
pub struct SkylineLdl {
    inner: Skyline,
    negative: usize,
}

impl Skyline {
    /// 列ごとの先頭の非零行 `first_rows`（`first_rows[j] <= j`）から零行列を作る。
    pub fn new(first_rows: &[usize]) -> Result<Self, SkylineError> {
        let n = first_rows.len();
        let mut ptr = Vec::with_capacity(n + 1);
        ptr.push(0);
        for (col, &first_row) in first_rows.iter().enumerate() {
            if first_row > col {
                return Err(SkylineError::BadProfile { col, first_row });
            }
            ptr.push(ptr[col] + col - first_row + 1);
        }
        let total = ptr[n];
        Ok(Self {
            n,
            first: first_rows.to_vec(),
            ptr,
            data: vec![0.0; total],
        })
    }

    /// 結合のある (行, 列) の組（順不同・重複可）から、対角を必ず含む最小のプロファイルの零行列を作る。
    pub fn from_couplings(
        n: usize,
        couplings: impl IntoIterator<Item = (usize, usize)>,
    ) -> Result<Self, SkylineError> {
        let mut first: Vec<usize> = (0..n).collect();
        for (a, b) in couplings {
            let (row, col) = if a <= b { (a, b) } else { (b, a) };
            if col >= n {
                return Err(SkylineError::OutsideProfile { row, col });
            }
            first[col] = first[col].min(row);
        }
        Self::new(&first)
    }

    /// 行列の次数。
    pub fn dim(&self) -> usize {
        self.n
    }

    /// 持っている要素の数（メモリの目安）。
    pub fn stored_len(&self) -> usize {
        self.data.len()
    }

    /// 列 `col` の先頭の非零行。
    pub fn first_row(&self, col: usize) -> Option<usize> {
        self.first.get(col).copied()
    }

    fn index(&self, row: usize, col: usize) -> Result<usize, SkylineError> {
        let (r, c) = if row <= col { (row, col) } else { (col, row) };
        if c >= self.n || r < self.first[c] {
            return Err(SkylineError::OutsideProfile { row, col });
        }
        Ok(self.ptr[c] + r - self.first[c])
    }

    /// 要素 (row, col)（対称なので順不同）。プロファイルの外は 0 を返す。範囲外の添字は `None`。
    pub fn get(&self, row: usize, col: usize) -> Option<f64> {
        if row >= self.n || col >= self.n {
            return None;
        }
        Some(self.index(row, col).map_or(0.0, |i| self.data[i]))
    }

    /// 要素 (row, col) に `value` を足す（組み立て）。(col, row) は同じ要素を指す。
    pub fn add(&mut self, row: usize, col: usize, value: f64) -> Result<(), SkylineError> {
        let i = self.index(row, col)?;
        self.data[i] += value;
        Ok(())
    }

    /// `self += scale · other`。`other` の非零要素はこの行列のプロファイルに収まっていること。
    pub fn add_scaled(&mut self, other: &Skyline, scale: f64) -> Result<(), SkylineError> {
        if other.n != self.n {
            return Err(SkylineError::SizeMismatch {
                expected: self.n,
                got: other.n,
            });
        }
        for col in 0..other.n {
            for row in other.first[col]..=col {
                let v = other.data[other.ptr[col] + row - other.first[col]];
                if v != 0.0 {
                    self.add(row, col, scale * v)?;
                }
            }
        }
        Ok(())
    }

    /// 行列ベクトル積 `y = A x`。
    pub fn mul_vec(&self, x: &[f64], y: &mut [f64]) -> Result<(), SkylineError> {
        for len in [x.len(), y.len()] {
            if len != self.n {
                return Err(SkylineError::SizeMismatch {
                    expected: self.n,
                    got: len,
                });
            }
        }
        y.fill(0.0);
        for col in 0..self.n {
            let first = self.first[col];
            let column = &self.data[self.ptr[col]..self.ptr[col + 1]];
            let (off, diag) = column.split_at(column.len() - 1);
            let xc = x[col];
            let mut acc = diag[0] * xc;
            for (k, &a) in off.iter().enumerate() {
                y[first + k] += a * xc;
                acc += a * x[first + k];
            }
            y[col] += acc;
        }
        Ok(())
    }

    /// 正定値として分解する（負・零のピボットはエラー）。`self` は消費される（コピーしない）。
    pub fn into_ldl(self) -> Result<SkylineLdl, SkylineError> {
        self.factor_core(false)
    }

    /// 不定でもよい分解（慣性の計算用）。負のピボットは数えて進み、零のピボットだけがエラー。
    pub fn into_ldl_indefinite(self) -> Result<SkylineLdl, SkylineError> {
        self.factor_core(true)
    }

    /// 負の固有値の個数を数える（Sturm 列の判定）。内部で複製を分解するので `self` は変わらない。
    ///
    /// 零のピボットに当たったときは [`SkylineError::ZeroPivot`]（しきい値を少しずらして呼び直す）。
    pub fn count_negative_eigenvalues(&self) -> Result<usize, SkylineError> {
        Ok(self.clone().into_ldl_indefinite()?.negative_pivots())
    }

    fn factor_core(mut self, allow_negative: bool) -> Result<SkylineLdl, SkylineError> {
        let mut negative = 0;
        for j in 0..self.n {
            let pivot = self.factor_column(j);
            let (index, d) = (j, pivot.0);
            if d.abs() <= ZERO_PIVOT_RELATIVE * pivot.1.abs() || !d.is_finite() {
                return Err(SkylineError::ZeroPivot { index, pivot: d });
            }
            if d < 0.0 {
                if !allow_negative {
                    return Err(SkylineError::NonPositivePivot { index, pivot: d });
                }
                negative += 1;
            }
            let end = self.ptr[j + 1] - 1;
            self.data[end] = d;
        }
        Ok(SkylineLdl {
            inner: self,
            negative,
        })
    }

    /// 列 j の非対角を U の列に直し、(ピボット d_j, 元の対角成分) を返す（対角は書き換えない）。
    fn factor_column(&mut self, j: usize) -> (f64, f64) {
        let first_j = self.first[j];
        let (left, right) = self.data.split_at_mut(self.ptr[j]);
        let col = &mut right[..self.ptr[j + 1] - self.ptr[j]];
        let a_jj = col[j - first_j];
        for i in first_j..j {
            let first_i = self.first[i];
            let ci = &left[self.ptr[i]..self.ptr[i + 1]];
            let lo = first_i.max(first_j);
            let s: f64 = (lo..i).map(|k| ci[k - first_i] * col[k - first_j]).sum();
            col[i - first_j] -= s;
        }
        let mut d = a_jj;
        for k in first_j..j {
            let g = col[k - first_j];
            let d_k = left[self.ptr[k + 1] - 1];
            let u = g / d_k;
            d -= u * g;
            col[k - first_j] = u;
        }
        (d, a_jj)
    }
}

impl SkylineLdl {
    /// 行列の次数。
    pub fn dim(&self) -> usize {
        self.inner.n
    }

    /// 負のピボットの個数（= 元の行列の負の固有値の個数。Sylvester の慣性則）。
    pub fn negative_pivots(&self) -> usize {
        self.negative
    }

    /// ピボット D の対角成分 `d_j`。
    pub fn pivot(&self, j: usize) -> Option<f64> {
        (j < self.inner.n).then(|| self.inner.data[self.inner.ptr[j + 1] - 1])
    }

    /// `A x = b` を `b` の上で解く（`b` は解で置き換わる）。
    pub fn solve_in_place(&self, b: &mut [f64]) -> Result<(), SkylineError> {
        let s = &self.inner;
        if b.len() != s.n {
            return Err(SkylineError::SizeMismatch {
                expected: s.n,
                got: b.len(),
            });
        }
        // Uᵀ y = b
        for j in 0..s.n {
            let col = &s.data[s.ptr[j]..s.ptr[j + 1] - 1];
            let dot: f64 = col.iter().zip(&b[s.first[j]..j]).map(|(u, y)| u * y).sum();
            b[j] -= dot;
        }
        // z = D⁻¹ y
        for (j, bj) in b.iter_mut().enumerate() {
            *bj /= s.data[s.ptr[j + 1] - 1];
        }
        // U x = z
        for j in (0..s.n).rev() {
            let col = &s.data[s.ptr[j]..s.ptr[j + 1] - 1];
            let xj = b[j];
            for (u, bi) in col.iter().zip(&mut b[s.first[j]..j]) {
                *bi -= u * xj;
            }
        }
        Ok(())
    }

    /// 複数の右辺（各ベクトルが 1 本）を同じ分解で順に解く。
    pub fn solve_many(&self, rhs: &mut [Vec<f64>]) -> Result<(), SkylineError> {
        rhs.iter_mut().try_for_each(|b| self.solve_in_place(b))
    }
}

#[cfg(test)]
mod tests;
