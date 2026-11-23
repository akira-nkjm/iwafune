//! Sturm 列の判定。K − ρM の負のピボット数 = ρ より小さい固有値の個数（Sylvester の慣性則）。

use super::{Mass, SubspaceError, shifted};
use crate::math::skyline::{Skyline, SkylineError};

/// 判定の結果。
pub(super) struct Verdict {
    /// 最大固有値の近傍に、返した個数を超える固有値が続く。
    pub cluster_split: bool,
}

/// `values`（昇順、先頭 `count` 個）が、最大値 `top` より下の固有値をすべて含むことを確かめる。
///
/// 数えるのは ρ = top − gap より小さい固有値の個数 N₋。返した個数のうち ρ より下にあるものは `returned_below`。
/// N₋ が `returned_below` より多ければ取りこぼし（エラー）。`top + gap` までの個数が `count` を超えるなら
/// 同じ固有値が打ち切り位置をまたいでいる（`cluster_split`。エラーにはしない）。
pub(super) fn check(
    k: &Skyline,
    m: &Mass<'_>,
    values: &[f64],
    top: f64,
    gap: f64,
) -> Result<Verdict, SubspaceError> {
    let below = count_below(k, m, top - gap, gap)?;
    let returned_below = values.iter().filter(|&&v| v < top - gap).count();
    if below > returned_below {
        return Err(SubspaceError::MissedModes {
            below,
            returned_below,
        });
    }
    let above = count_below(k, m, top + gap, gap)?;
    Ok(Verdict {
        cluster_split: above > values.len(),
    })
}

/// ρ より小さい固有値の個数。零のピボットに当たったら ρ を少しずつずらして数え直す。
fn count_below(k: &Skyline, m: &Mass<'_>, rho: f64, gap: f64) -> Result<usize, SubspaceError> {
    let mut rho = rho;
    let mut last = SkylineError::NotFinite;
    for attempt in 0..5 {
        match shifted(k, m, -rho)?.count_negative_eigenvalues() {
            Ok(n) => return Ok(n),
            Err(e @ SkylineError::ZeroPivot { .. }) => {
                last = e;
                rho -= gap * 0.1 * f64::from(attempt + 1);
            }
            Err(e) => return Err(SubspaceError::SturmFactor(e)),
        }
    }
    Err(SubspaceError::SturmFactor(last))
}
