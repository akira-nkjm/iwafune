//! 弧の連鎖: 時間の順に並べた、それぞれ期間を持つ軌道（弧）。**どの時刻がどの弧に属すか、折れ目（境）での扱いを決めるのはここだけ**。
//!
//! スイングバイのある巡航は、通過の日を境に、通過の天体に合わせた日心の二体の弧が替わる。境で位置は連続（どちらの弧も通過の日の
//! 天体の位置を通る）で、速度は跳ぶ（通過で日心の速度が変わる）。通過の間（天体の影響圏の中）は解かず、通過は一瞬とみなす。
//! 単発の遷移は弧 1 本の場合で、[`Trajectory::chain`] は弧が 1 本ならその弧をそのまま返す（連鎖の変種は 2 本以上のときだけ）。
//!
//! - **時刻と弧**: 元期（先頭の弧の始め）からの時刻 `t` は、始めが `t` 以前の最後の弧で評価する。境ちょうどは後の弧（出ていく弧）。
//!   先頭の始めより前は先頭の弧、最後の終わりより後は最後の弧の延長（弧 1 本の [`Trajectory::Spanned`] が期間の外へ延長するのと同じ）。
//! - **表**（[`OrbitTrack`]）: 弧ごとに作り、境をまたいで補間しない（速度の跳びをなだらかにしない）。
//! - **付け替え**（[`Trajectory::starting_at`]）: 始めの時刻を含む弧の残りと、それより後の弧。
//! - **検査**（[`ArcChain::new`]）: 弧の中心天体が同じ、弧が時間でつながる（許容 [`BREAK_TIME_TOL_S`]）、境で位置が一致する
//!   （許容 [`BREAK_POSITION_TOL_M`]。Lambert の弧どうしは両端が同じ天体の位置を通り、二分法の飛行時間の幅から 0.1 m 未満）。

use super::Trajectory;
use crate::AstroError;
use crate::ephem::CentralBody;
use crate::orbit::{OrbitState, OrbitTrack, arc_index};
use crate::time::Epoch;

/// 隣り合う弧の、前の弧の終わりと次の弧の始めの時刻の食い違いの許容 [s]（影響圏の境を詰める二分法の許容と同じ 1 ms）。
pub const BREAK_TIME_TOL_S: f64 = 1.0e-3;
/// 境での位置の食い違いの許容 [m]。Lambert の弧の両端は暦の天体の位置を通り（飛行時間の二分法の幅 `1e-13` は速さ 30 km/s で 0.1 m 未満）、
/// 接触要素への往復の丸めはそれより小さい。これを超える連鎖は、弧が実際には接していないので作らない。
pub const BREAK_POSITION_TOL_M: f64 = 1.0;

/// 時間の順の弧の連鎖（2 本以上。作るのは [`Trajectory::chain`] か [`ArcChain::new`]）。
#[derive(Debug, Clone)]
pub struct ArcChain {
    arcs: Vec<Trajectory>,
    /// 各弧の始め（連鎖の元期からの経過 [s]。先頭は 0、昇順）
    starts_s: Vec<f64>,
    /// 全体の長さ（最後の弧の終わり [s]）
    span_s: f64,
}

/// 弧 `arc`（`index` 本目）が使えるか: 連鎖でなく、期間が正の有限の値。
pub(super) fn check_arc(arc: &Trajectory, index: usize) -> Result<(), AstroError> {
    match (arc, arc.span_s()) {
        (Trajectory::Arcs(_), _) => Err(AstroError::InvalidInput(format!(
            "Arc {index} is itself a chain; chains do not nest"
        ))),
        (_, Some(span)) if span.is_finite() && span > 0.0 => Ok(()),
        _ => Err(AstroError::InvalidInput(format!(
            "Arc {index} needs a positive, finite span (span_s)"
        ))),
    }
}

impl ArcChain {
    /// 弧の並び（時間の順）から作る。2 本以上、どれも期間つき、中心天体が同じ、隣り合う弧が時間でつながり（[`BREAK_TIME_TOL_S`]）
    /// 境で位置が一致する（[`BREAK_POSITION_TOL_M`]）ことを確かめる。1 本だけなら [`Trajectory::chain`] を使う。
    pub fn new(arcs: Vec<Trajectory>) -> Result<Self, AstroError> {
        if arcs.len() < 2 {
            return Err(AstroError::InvalidInput(
                "A chain needs at least two arcs".to_owned(),
            ));
        }
        for (i, arc) in arcs.iter().enumerate() {
            check_arc(arc, i)?;
        }
        let (epoch, central) = (arcs[0].epoch(), arcs[0].central());
        let mut starts_s = vec![0.0];
        for (i, arc) in arcs.iter().enumerate().skip(1) {
            if arc.central() != central {
                return Err(AstroError::InvalidInput(format!(
                    "Arc {i} is about {:?} but the chain is about {central:?}",
                    arc.central()
                )));
            }
            starts_s.push(arc.epoch().seconds_since(epoch));
        }
        let chain = Self {
            span_s: 0.0,
            arcs,
            starts_s,
        };
        chain.check_breaks()?;
        let last = chain.arcs.len() - 1;
        Ok(Self {
            span_s: chain.starts_s[last] + chain.arc_span(last),
            ..chain
        })
    }

    /// 弧 `i` の期間 [s]（[`check_arc`] を通っているので必ずある）。
    fn arc_span(&self, i: usize) -> f64 {
        self.arcs[i].span_s().unwrap_or(0.0)
    }

    /// 境ごとに、時間のつながりと位置の一致を確かめる。
    fn check_breaks(&self) -> Result<(), AstroError> {
        for i in 1..self.arcs.len() {
            let end_s = self.starts_s[i - 1] + self.arc_span(i - 1);
            let gap_s = self.starts_s[i] - end_s;
            if gap_s.is_nan() || gap_s.abs() > BREAK_TIME_TOL_S {
                return Err(AstroError::InvalidInput(format!(
                    "Arc {i} starts {gap_s:+.3} s from the end of the previous arc; consecutive arcs must meet (tolerance {BREAK_TIME_TOL_S} s)"
                )));
            }
            let before = self.arcs[i - 1].state_at(self.arc_span(i - 1))?;
            let after = self.arcs[i].state_at(0.0)?;
            let miss_m = (before.r_eci - after.r_eci).norm();
            if miss_m.is_nan() || miss_m > BREAK_POSITION_TOL_M {
                return Err(AstroError::InvalidInput(format!(
                    "The position of arc {i} differs by {miss_m:.3} m from the end of the previous arc; consecutive arcs must meet (tolerance {BREAK_POSITION_TOL_M} m)"
                )));
            }
        }
        Ok(())
    }

    /// 弧（時間の順）。
    pub fn arcs(&self) -> &[Trajectory] {
        &self.arcs
    }

    /// 先頭の弧。
    pub(super) fn first(&self) -> &Trajectory {
        &self.arcs[0]
    }

    /// 元期（先頭の弧の始め）。
    pub(super) fn epoch(&self) -> Epoch {
        self.arcs[0].epoch()
    }

    /// 中心天体（どの弧も同じ）。
    pub(super) fn central(&self) -> CentralBody {
        self.arcs[0].central()
    }

    /// 全体の長さ [s]（元期から最後の弧の終わりまで）。
    pub fn span_s(&self) -> f64 {
        self.span_s
    }

    /// 境の時刻（元期からの経過 [s]、昇順）。
    pub(super) fn breakpoints_s(&self) -> Vec<f64> {
        self.starts_s[1..].to_vec()
    }

    /// 弧ごとの表の格子の刻みの最小 [s]。
    pub(super) fn table_step_s(&self) -> f64 {
        self.arcs
            .iter()
            .map(Trajectory::table_step_s)
            .fold(f64::INFINITY, f64::min)
    }

    /// 元期から `t_s` 秒後の状態（時刻を含む弧で）。
    pub(super) fn state_at(&self, t_s: f64) -> Result<OrbitState, AstroError> {
        let k = arc_index(&self.starts_s, t_s);
        self.arcs[k].state_at(t_s - self.starts_s[k])
    }

    /// 時刻列の状態。続く時刻が同じ弧に入る範囲ごとに、その弧の `states_at` に渡す（弧の中は弧の評価そのもの）。
    pub(super) fn states_at(&self, times_s: &[f64]) -> Result<Vec<OrbitState>, AstroError> {
        let mut out = Vec::with_capacity(times_s.len());
        let mut from = 0;
        while from < times_s.len() {
            let k = arc_index(&self.starts_s, times_s[from]);
            let run = times_s[from..]
                .iter()
                .take_while(|t| arc_index(&self.starts_s, **t) == k)
                .count();
            let local: Vec<f64> = times_s[from..from + run]
                .iter()
                .map(|t| t - self.starts_s[k])
                .collect();
            out.extend(self.arcs[k].states_at(&local)?);
            from += run;
        }
        Ok(out)
    }

    /// 任意の時刻の表を弧ごとに作る（[`Trajectory::track`]。最後の弧は `span_s` まで）。
    pub(super) fn track(&self, span_s: f64, max_grid_s: f64) -> Result<OrbitTrack, AstroError> {
        let last = self.arcs.len() - 1;
        let tracks = self
            .arcs
            .iter()
            .enumerate()
            .map(|(i, arc)| {
                let own = self.arc_span(i);
                let local = if i == last {
                    (span_s - self.starts_s[i]).max(own)
                } else {
                    own
                };
                arc.track(local, max_grid_s)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(OrbitTrack::from_arcs(self.starts_s.clone(), tracks))
    }

    /// `start` を元期にした同じ連鎖（[`Trajectory::starting_at`]。範囲の検査は呼ぶ側）: `start` を含む弧の残りと、それより後の弧。
    /// 残りが 1 本の弧なら、その弧そのもの。
    pub(super) fn starting_at(&self, start: Epoch) -> Result<Trajectory, AstroError> {
        // 弧ごとの `starting_at` が見る経過（`start − 弧の元期`）で選ぶ（境ちょうどは後の弧。丸めで前の弧の外に出ない）
        let k = (0..self.arcs.len())
            .rev()
            .find(|&i| start.seconds_since(self.arcs[i].epoch()) >= 0.0)
            .unwrap_or(0);
        let mut arcs = vec![self.arcs[k].starting_at(start)?];
        arcs.extend(self.arcs[k + 1..].iter().cloned());
        Trajectory::chain(arcs)
    }

    /// 全体の期間を `span_s` にした連鎖（始めが `span_s` 以降の弧は落とし、最後の弧の終わりを `span_s` にする）。
    /// 残りが 1 本の弧なら、その弧そのもの。
    pub(super) fn with_span(self, span_s: f64) -> Trajectory {
        let keep = 1 + self.starts_s[1..]
            .iter()
            .take_while(|s| **s < span_s)
            .count();
        let last_start = self.starts_s[keep - 1];
        let mut arcs = self.arcs;
        arcs.truncate(keep);
        let mut starts_s = self.starts_s;
        starts_s.truncate(keep);
        if let Some(last) = arcs.pop() {
            arcs.push(last.with_span(span_s - last_start));
        }
        match arcs.len() {
            1 => arcs.remove(0),
            _ => Trajectory::Arcs(Self {
                arcs,
                starts_s,
                span_s,
            }),
        }
    }
}

#[cfg(test)]
mod tests;
