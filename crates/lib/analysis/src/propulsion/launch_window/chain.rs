//! 通過（スイングバイ）のある遷移の日付の探索。直接の遷移（通過ゼロ）を一般化したもので、点の評価は同じ [`Evaluator`]
//! （= `propulsion.maneuvers` と同じ `plan` の経路。第 2 の Lambert や通過の式は持たない）。
//!
//! 変数は日付の列 `[出発, 通過 1, …, 通過 k, 到着]`（k ≤ [`MAX_FLYBYS`]）。出発 × 到着の格子（[`super::axes`]）の点ごとに、
//! 通過の日を ΔV が最小になるよう決める（出力の格子はこれまでと同じ形で、各点が「通過の日を最適にした値」になる）:
//!
//! 1. **種**: 出発と到着のあいだを等分した点（通過 1 つなら 24 点、2 つなら 14 点から選ぶ組 91、3 つなら 10 点から選ぶ組 120）と、
//!    書かれた通過の日（出発と到着のあいだに収まるなら）。通過が実現できない日付（下限の高度で曲がりきれない）は除いて数える。
//! 2. **局所の詰め**: 最良の種から、通過の日だけを動かすコンパス探索（刻みは種の間隔から 4 回半分にする）。実現できる範囲の縁では、動かす先を `CellValues::guide_m_s`（足りない回頭の分を足した連続な値）で決め、採るのは実現できる点だけ。
//!
//! 格子の ΔV の極小の点（上位 6 つ）からは、**全部の日付**を動かすコンパス探索で最小を詰める（刻みは 1 時間まで。[`REFINE_FLOOR_S`]）。
//! コンパス探索は各軸を −1・0・+1 動かす `3ⁿ − 1` 通り（全部の日付がそろって動く方向を含む）を試し、改善が無ければ刻みを半分にする（途中で止まる回数の上限は [`REFINE_MAX_EVALS`]）。
//! 終わったとき、最後の刻み（1 時間）で動かした近傍はどれも良くないか、探索の範囲の外か、実現できない。局所の最小であって大域ではない
//! （格子の点の数と種の数が探す範囲を決める）。

use super::axes::Axes;
use super::cell::{CellValues, Evaluator};
use crate::spec::AnalysisError;
use sat_astro::constants::SECONDS_PER_DAY;
use sat_astro::time::Epoch;

/// 探索できる通過の数の上限（変数が増えると解く回数が桁で増える）。
pub(super) const MAX_FLYBYS: usize = 3;

/// 局所の詰めの最小の刻み [s]（日付は整数秒。書き出した日付を読み直すと同じ点になる）。
pub(super) const REFINE_FLOOR_S: f64 = 3600.0;

/// 全部の日付の詰め 1 回で評価する点の数の上限（これを超えて止まったら注記する）。
pub(super) const REFINE_MAX_EVALS: u32 = 1500;

/// 通過の日だけの詰めの刻みを半分にする回数。
const INNER_HALVINGS: u32 = 4;

/// 通過の数ごと（添字 = 通過の数）の、通過の日の等分点の数。組の数は 24 / 91 / 120。
const LATTICE_POINTS: [usize; MAX_FLYBYS + 1] = [0, 24, 14, 10];

/// 解いた回数の集計（通過のある探索の結果に載せる）。
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Counts {
    /// 日付の列ごとに連鎖を解いた回数（局所の詰めも含む）
    pub chain_solves: u32,
    /// そのうち、通過のどれかが実現できなくて除いた回数
    pub infeasible_solves: u32,
    /// 実現できる通過の日が 1 つも無かった格子の点の数
    pub infeasible_cells: u32,
    /// 全部の日付の詰めで評価した点の数（`chain_solves` の内数）
    pub refine_evals: u32,
    /// 全部の日付の詰めが評価の上限で止まったか
    pub refine_capped: bool,
}

/// 探索の点（日付の列 [s] と、その値の全体）。
struct Point {
    x: Vec<f64>,
    values: CellValues,
}

fn epoch_of(s: f64) -> Epoch {
    Epoch { j2000_s: s }
}

fn epochs(x: &[f64]) -> Vec<Epoch> {
    x.iter().copied().map(epoch_of).collect()
}

/// 日付 `x`（通過の日だけ）が出発 `lo` と到着 `hi` のあいだで、`gap` 以上ずつ離れて増えているか。
fn ordered(lo: f64, x: &[f64], hi: f64, gap: f64) -> bool {
    let mut prev = lo;
    for v in x {
        if *v < prev + gap {
            return false;
        }
        prev = *v;
    }
    hi >= prev + gap
}

/// 動かす方向（各軸を −1・0・+1 にした `3ⁿ − 1` 通り。全部の日付がそろって動く方向（飛行全体の日のずれ）を含める）。
fn directions(n: usize) -> Vec<Vec<f64>> {
    let total = 3usize.pow(n as u32);
    (0..total)
        .map(|mut code| {
            (0..n)
                .map(|_| {
                    let digit = code % 3;
                    code /= 3;
                    digit as f64 - 1.0
                })
                .collect::<Vec<f64>>()
        })
        .filter(|d| d.iter().any(|v| *v != 0.0))
        .collect()
}

/// コンパス探索: `start`（実現できる点）から、刻み `floor × 2^halvings` で始めて改善が無ければ半分にし、`floor` の刻みで改善が無くなるまで。
/// 動く先は **探索の目安**（[`CellValues::guide_m_s`]。実現できない日付でも連続）で決め、返すのは見つけた**実現できる点の選ぶ基準（[`CellValues::objective_m_s`]）の最小**。
/// `eval` は日付の列の値（解けない・範囲の外は `None`。実現できない日付の値も返す）。
/// 評価が `max_evals` 回に達したら止めて、`false`（収束していない）を返す。
fn compass(
    start: Point,
    (floor, halvings): (f64, u32),
    max_evals: u32,
    eval: &mut dyn FnMut(&[f64]) -> Option<CellValues>,
) -> (Point, bool) {
    let dirs = directions(start.x.len());
    let mut best = Point {
        x: start.x.clone(),
        values: start.values.clone(),
    };
    let mut cur = start;
    let mut step = floor * f64::from(1u32 << halvings);
    let mut evals = 0;
    loop {
        let mut improved = false;
        for d in &dirs {
            if evals >= max_evals {
                return (best, false);
            }
            let x: Vec<f64> = cur.x.iter().zip(d).map(|(v, s)| v + s * step).collect();
            evals += 1;
            let Some(values) = eval(&x) else { continue };
            if values.feasible && values.objective_m_s < best.values.objective_m_s {
                best = Point {
                    x: x.clone(),
                    values: values.clone(),
                };
            }
            if values.guide_m_s < cur.values.guide_m_s {
                cur = Point { x, values };
                improved = true;
                break;
            }
        }
        if !improved {
            if step <= floor {
                return (best, true);
            }
            step /= 2.0;
        }
    }
}

/// 動力なしの通過がある探索の最後の段（刻みは 4096 s から 1 s まで半分にする）の、評価の上限。
const SECOND_STAGE_MAX_EVALS: u32 = 3000;
const SECOND_STAGE_HALVINGS: u32 = 12;

/// 実現できる範囲の縁（回頭の上限）では ΔV に折れ目があり、コンパス探索は縁に沿った細い谷で止まりやすい。
/// そこで仕上げに、最良の点のまわりを一様な乱数で探る（半径は 8 日から 1 時間まで 8 段、段ごとに [`POLISH_SAMPLES`] 点）。
/// 乱数は固定の種から始める（同じ入力は同じ出力）。改善した点は次の中心にする。
fn polish(
    start: Point,
    max_evals: u32,
    eval: &mut dyn FnMut(&[f64]) -> Option<CellValues>,
) -> Point {
    // xorshift64*（固定の種）
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut uniform = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let r = state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11;
        r as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    };
    let mut best = start;
    let mut evals = 0;
    for level in 0..POLISH_LEVELS {
        let radius = POLISH_START_RADIUS_S / f64::from(1u32 << level);
        for _ in 0..POLISH_SAMPLES {
            if evals >= max_evals {
                return best;
            }
            evals += 1;
            let x: Vec<f64> = best
                .x
                .iter()
                .map(|v| (v + uniform() * radius).round())
                .collect();
            if let Some(values) = eval(&x)
                && values.feasible
                && values.objective_m_s < best.values.objective_m_s
            {
                best = Point { x, values };
            }
        }
    }
    best
}

/// 仕上げの探索の段の数・段ごとの点の数・最初の半径 [s]（8 日。1 時間まで半分にして 8 段）。
const POLISH_LEVELS: u32 = 8;
const POLISH_SAMPLES: u32 = 500;
const POLISH_START_RADIUS_S: f64 = 8.0 * SECONDS_PER_DAY;

/// `k` 個を `0..m` から選ぶ増加する組（辞書順）。
fn combinations(m: usize, k: usize) -> Vec<Vec<usize>> {
    fn go(start: usize, m: usize, k: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if cur.len() == k {
            out.push(cur.clone());
            return;
        }
        for i in start..m {
            cur.push(i);
            go(i + 1, m, k, cur, out);
            cur.pop();
        }
    }
    let mut out = Vec::new();
    go(0, m, k, &mut Vec::new(), &mut out);
    out
}

/// 出発 `dep`・到着 `arr` のあいだの通過の日の種（等分点の組 + 書かれた通過の日）。
fn seeds(dep: f64, arr: f64, written: &[f64]) -> Vec<Vec<f64>> {
    let k = written.len();
    let m = LATTICE_POINTS[k];
    let at = |i: usize| (dep + (arr - dep) * (i + 1) as f64 / (m + 1) as f64).round();
    let mut out: Vec<Vec<f64>> = combinations(m, k)
        .into_iter()
        .map(|c| c.into_iter().map(at).collect())
        .collect();
    if ordered(dep, written, arr, REFINE_FLOOR_S) {
        out.push(written.to_vec());
    }
    out
}

/// 格子の点（出発 `dep`・到着 `arr`）の、通過の日を ΔV が最小になるよう決めた値。
/// 実現できる日付が無ければ `Ok(None)`（数えて除く）、1 つも解けなければ最初のエラー。
pub(super) fn best_for_cell(
    ev: &mut Evaluator,
    counts: &mut Counts,
    (dep, arr): (Epoch, Epoch),
) -> Result<Option<CellValues>, AnalysisError> {
    let (d0, a0) = (dep.j2000_s, arr.j2000_s);
    let written: Vec<f64> = ev.written_flyby_dates().iter().map(|e| e.j2000_s).collect();
    let (mut first_error, mut solved_any) = (None, false);
    let mut eval = |x: &[f64]| -> Option<CellValues> {
        if !ordered(d0, x, a0, REFINE_FLOOR_S) {
            return None;
        }
        counts.chain_solves += 1;
        match ev.at_via(dep, &epochs(x), arr) {
            Ok(v) => {
                solved_any = true;
                if !v.feasible {
                    counts.infeasible_solves += 1;
                }
                Some(v)
            }
            Err(e) => {
                first_error.get_or_insert(e);
                None
            }
        }
    };
    let mut start: Option<Point> = None;
    for x in seeds(d0, a0, &written) {
        if let Some(values) = eval(&x)
            && values.feasible
            && start
                .as_ref()
                .is_none_or(|b| values.objective_m_s < b.values.objective_m_s)
        {
            start = Some(Point { x, values });
        }
    }
    let Some(start) = start else {
        return match first_error {
            Some(e) if !solved_any => Err(e),
            _ => {
                counts.infeasible_cells += 1;
                Ok(None)
            }
        };
    };
    let spacing = (a0 - d0) / (LATTICE_POINTS[written.len()] + 1) as f64;
    let floor = (spacing / f64::from(1u32 << INNER_HALVINGS))
        .round()
        .max(REFINE_FLOOR_S);
    let (best, _) = compass(start, (floor, INNER_HALVINGS), 200, &mut eval);
    Ok(Some(best.values))
}

/// 格子の点の通過の日の候補（出発・到着・通過の日）。
pub(super) type Start = (Epoch, Epoch, Vec<Epoch>);

/// 格子の ΔV の極小の点（上位 [`REFINE_STARTS`] 個）それぞれから、全部の日付（出発・通過・到着）を動かして最小を詰め、最良のものを返す。
/// 出発の窓・飛行時間の範囲の外には出ない。始めの点が実現できなければその点は使わない（格子の値と同じ日付なので通常は起きない）。
pub(super) fn refine_best(
    ev: &mut Evaluator,
    counts: &mut Counts,
    axes: &Axes,
    starts: Vec<Start>,
) -> Option<(Epoch, Epoch, CellValues)> {
    let mut best: Option<(Epoch, Epoch, CellValues)> = None;
    for start in starts {
        let Some(found) = refine(ev, counts, axes, start) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|(_, _, b)| found.2.objective_m_s < b.objective_m_s)
        {
            best = Some(found);
        }
    }
    best
}

/// 全部の日付の詰めの始めの点の数（格子の極小のうち ΔV の小さい順。格子の最小は常に含む）。
pub(super) const REFINE_STARTS: usize = 6;

/// 1 つの始めの点からの詰め。
fn refine(
    ev: &mut Evaluator,
    counts: &mut Counts,
    axes: &Axes,
    (dep, arr, via): Start,
) -> Option<(Epoch, Epoch, CellValues)> {
    let k = via.len();
    let ev_has_unpowered = ev.has_unpowered_flyby();
    counts.chain_solves += 1;
    let values = ev.at_via(dep, &via, arr).ok().filter(|v| v.feasible)?;
    let mut x = vec![dep.j2000_s];
    x.extend(via.iter().map(|e| e.j2000_s));
    x.push(arr.j2000_s);
    let start = Point { x, values };
    let span = &axes.span;
    let (d_lo, d_hi) = (span.departure_start.j2000_s, span.departure_end.j2000_s);
    let (t_lo, t_hi) = (
        span.time_of_flight_min.value(),
        span.time_of_flight_max.value(),
    );
    let mut eval = |x: &[f64]| -> Option<CellValues> {
        let (d, a) = (x[0], x[k + 1]);
        let tof = a - d;
        if d < d_lo || d > d_hi || tof < t_lo || tof > t_hi {
            return None;
        }
        if !ordered(d, &x[1..=k], a, REFINE_FLOOR_S) {
            return None;
        }
        counts.chain_solves += 1;
        counts.refine_evals += 1;
        let v = ev
            .at_via(epoch_of(d), &epochs(&x[1..=k]), epoch_of(a))
            .ok()?;
        if !v.feasible {
            counts.infeasible_solves += 1;
        }
        Some(v)
    };
    // 格子の間隔（出発・到着の細かいほう）から 1 時間まで半分にしていく
    let spacing = (axes.departure_step_s().min(axes.arrival_step_s())).max(REFINE_FLOOR_S);
    let halvings = (spacing / REFINE_FLOOR_S).log2().ceil().max(0.0) as u32;
    let (best, converged) = compass(
        start,
        (REFINE_FLOOR_S, halvings),
        REFINE_MAX_EVALS,
        &mut eval,
    );
    counts.refine_capped |= !converged;
    let best = polish(best, REFINE_MAX_EVALS, &mut eval);
    // 仕上げで動いた点から、細かい刻みでもう一度
    let (best, converged) = compass(best, (REFINE_FLOOR_S, 3), REFINE_MAX_EVALS, &mut eval);
    counts.refine_capped |= !converged;
    // 動力なしの通過があれば、日付を書き出す精度（整数秒。`Epoch::to_iso`）の 1 s まで詰める。
    // 近点の噴射（超過速度の差）が弾道の範囲に入るまで、評価は [`SECOND_STAGE_MAX_EVALS`] 回まで
    let best = if ev_has_unpowered {
        let (b, _) = compass(
            best,
            (1.0, SECOND_STAGE_HALVINGS),
            SECOND_STAGE_MAX_EVALS,
            &mut eval,
        );
        b
    } else {
        best
    };
    let n = best.x.len();
    Some((epoch_of(best.x[0]), epoch_of(best.x[n - 1]), best.values))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combinations_count_matches_the_binomial() {
        assert_eq!(combinations(24, 1).len(), 24);
        assert_eq!(combinations(14, 2).len(), 91);
        assert_eq!(combinations(10, 3).len(), 120);
    }

    #[test]
    fn dates_must_increase_by_the_gap() {
        assert!(ordered(0.0, &[4000.0, 8000.0], 12000.0, 3600.0));
        assert!(!ordered(0.0, &[4000.0, 6000.0], 12000.0, 3600.0));
        assert!(!ordered(0.0, &[4000.0], 5000.0, 3600.0));
    }

    #[test]
    fn directions_are_all_nonzero_steps_of_minus_one_zero_one() {
        assert_eq!(directions(1).len(), 2);
        assert_eq!(directions(3).len(), 26);
        assert_eq!(directions(5).len(), 242);
        assert!(directions(3).contains(&vec![1.0, 1.0, 1.0]));
    }
}
