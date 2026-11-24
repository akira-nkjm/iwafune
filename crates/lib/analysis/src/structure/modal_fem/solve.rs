//! 固有値の計算。`sat_astro::math::lowest_modes`（シフト付き部分空間反復）を呼ぶ。
//! 拘束が無い・足りない系は K が特異なので、K + σM が正定値になるシフト σ > 0 を使う（0 に近い固有値はそのまま 0 付近に出る）。

use super::assemble::System;
use crate::spec::AnalysisError;
use sat_astro::math::{Mass, Modes, SubspaceError, SubspaceOptions, lowest_modes};

/// シフト σ を、並進の自由度の K_ii / M_ii の平均に掛ける比。
/// 平均は要素の大きさで決まる高い側の固有値の目安で、これに 1e-6 を掛けると概念設計の機体の低次のモードの付近になる
/// （反復の収束は (λ + σ) の比で決まるので、低次の固有値と同じ桁なら速い）。
pub const SHIFT_RATIO: f64 = 1.0e-6;
/// 反復の収束の許容値（固有値の相対変化）。
pub const SOLVER_TOLERANCE: f64 = 1.0e-10;
/// 反復の上限回数。
pub const SOLVER_MAX_ITERATIONS: usize = 500;

/// 固有値問題の解。
#[derive(Debug, Clone)]
pub struct Solved {
    /// 固有値 λ = ω² [rad²/s²]（昇順。丸め誤差で 0 付近は負になりうる）
    pub values: Vec<f64>,
    /// 質量で正規化した固有ベクトル（自由な自由度の順）
    pub vectors: Vec<Vec<f64>>,
    pub iterations: usize,
    /// 求めた最後の固有値が重根の途中で切れている
    pub cluster_split: bool,
}

/// 並進の自由度の K_ii / M_ii の平均からシフトを決める。
fn shift(sys: &System) -> f64 {
    let (mut sum, mut count) = (0.0, 0_usize);
    for (r, g) in sys.global.iter().enumerate() {
        if g % 6 < 3
            && let Some(k) = sys.k.get(r, r)
        {
            sum += k / sys.mass[r];
            count += 1;
        }
    }
    SHIFT_RATIO * sum / count.max(1) as f64
}

/// 低いほうから `count` 個の固有対を求める。
///
/// 反復ベクトルの数は既定（2·count + 8）。要素が粗すぎて低いモードが数個しか無い小さな問題では、
/// 余分なベクトルが縮退して射影した問題が解けないことがあるので、その場合だけ `count` 本で解き直す。
pub fn solve(sys: &System, count: usize) -> Result<Solved, AnalysisError> {
    let shift = shift(sys);
    let run = |vectors: Option<usize>| -> Result<Modes, SubspaceError> {
        let options = SubspaceOptions {
            tolerance: SOLVER_TOLERANCE,
            max_iterations: SOLVER_MAX_ITERATIONS,
            vectors,
            ..SubspaceOptions::default()
        };
        lowest_modes(&sys.k, &Mass::Diagonal(&sys.mass), count, shift, &options)
    };
    let modes = match run(None) {
        Err(SubspaceError::Projected(_)) => run(Some(count)),
        other => other,
    }
    .map_err(|e| AnalysisError::InvalidInput(format!("The eigenvalue solver failed: {e}")))?;
    Ok(Solved {
        values: modes.values,
        vectors: modes.vectors,
        iterations: modes.iterations,
        cluster_split: modes.cluster_split,
    })
}
