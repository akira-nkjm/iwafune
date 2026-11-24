//! 標本の最小・最大と、その前後の区間で詰めた極値（[`extremes_by`]）。量の中身（距離・角度）は知らない。

use crate::AstroError;
use crate::math::golden_section_min;
use std::cell::RefCell;

/// 黄金分割の反復の上限（区間は 0.618^N 倍になる。N = 60 で 3e-13 倍）。
const REFINE_ITERATIONS: usize = 60;

/// 極値の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtremumKind {
    Min,
    Max,
}

impl ExtremumKind {
    /// 最小化する関数にするための符号（最大なら値を反転する。反転は厳密）。
    fn sign(self) -> f64 {
        match self {
            Self::Min => 1.0,
            Self::Max => -1.0,
        }
    }
}

/// 極値 1 つ（時刻は標本の始めからの経過 [s]）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extremum {
    pub t_s: f64,
    pub value: f64,
}

/// 最小と最大。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extremes {
    pub min: Extremum,
    pub max: Extremum,
}

/// 値の列の最小と最大の添字（同値は最初の標本）。空なら `None`。NaN は全順序（`total_cmp`）で扱う。
pub fn extreme_indices(values: &[f64]) -> Option<(usize, usize)> {
    let min = (0..values.len()).min_by(|&a, &b| values[a].total_cmp(&values[b]))?;
    let max = (0..values.len()).min_by(|&a, &b| values[b].total_cmp(&values[a]))?;
    Some((min, max))
}

/// 標本 `(times, values)` の最小・最大を、標本の前後の区間（`[t[i-1], t[i+1]]`、端は端）で黄金分割して詰める
/// （`f` は時刻 → 値。標本の値と同じ量）。端の標本が極値なら端のまま（黄金分割は内側に寄るので、標本の値が良ければ標本を採る）。
/// 区間の中で単峰とみなす。詰めた時刻の値は `f` で評価し直す。`f` が失敗したらそのエラーを返す。標本が空なら `None`。
/// 軌道に折れ目があるときは [`extremes_by_split`]。
pub fn extremes_by(
    times: &[f64],
    values: &[f64],
    f: impl Fn(f64) -> Result<f64, AstroError>,
) -> Result<Option<Extremes>, AstroError> {
    extremes_by_split(times, values, &[], f)
}

/// [`extremes_by`] に折れ目（`breaks`。`times` と同じ座標の時刻。弧の連鎖の境 [`crate::orbit::Trajectory::breakpoints_s`]）を足したもの。
/// 詰める区間は折れ目で分け（折れ目をまたいで単峰とみなさない）、折れ目そのものも候補にする（折れ目で値は連続だが傾きが跳ぶので、
/// 極値が折れ目にあることがある）。折れ目が区間に入らなければ [`extremes_by`] と同じ。
pub fn extremes_by_split(
    times: &[f64],
    values: &[f64],
    breaks: &[f64],
    f: impl Fn(f64) -> Result<f64, AstroError>,
) -> Result<Option<Extremes>, AstroError> {
    let Some((imin, imax)) = extreme_indices(values) else {
        return Ok(None);
    };
    let extremum = |i: usize, kind: ExtremumKind| -> Result<Extremum, AstroError> {
        let sign = kind.sign();
        let (lo, hi) = (
            times[i.saturating_sub(1)],
            times[(i + 1).min(times.len() - 1)],
        );
        // 区間を折れ目で分ける（折れ目の無い区間は 1 つのまま）
        let inner: Vec<f64> = breaks
            .iter()
            .copied()
            .filter(|b| *b > lo && *b < hi)
            .collect();
        let edges: Vec<f64> = std::iter::once(lo)
            .chain(inner.iter().copied())
            .chain(std::iter::once(hi))
            .collect();
        // 評価の失敗は最初の 1 つを覚えて NaN を返し、詰め終わってから返す
        let failure = RefCell::new(None);
        let g = |t: f64| match f(t) {
            Ok(v) => sign * v,
            Err(e) => {
                failure.borrow_mut().get_or_insert(e);
                f64::NAN
            }
        };
        let refined: Vec<f64> = edges
            .windows(2)
            .map(|w| golden_section_min(g, w[0], w[1], 0.0, REFINE_ITERATIONS).0)
            .collect();
        if let Some(e) = failure.into_inner() {
            return Err(e);
        }
        // 詰めた時刻・折れ目・標本の時刻のうち良い方（同値は先に挙げた方）
        let (t, value) = refined
            .into_iter()
            .chain(inner)
            .chain(std::iter::once(times[i]))
            .map(|t| f(t).map(|v| (t, v)))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .reduce(|a, b| if sign * b.1 < sign * a.1 { b } else { a })
            .unwrap_or((times[i], values[i]));
        Ok(Extremum { t_s: t, value })
    };
    Ok(Some(Extremes {
        min: extremum(imin, ExtremumKind::Min)?,
        max: extremum(imax, ExtremumKind::Max)?,
    }))
}
