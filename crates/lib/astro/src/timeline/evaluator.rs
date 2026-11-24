//! 時系列の標本の間の任意の時刻で、時系列の 1 点（位置・太陽方向・日照・姿勢）を求める。
//! 標本の時刻では [`build`](super::build) の点と同じ値になる。熱の積分のように、標本の間で入熱を連続に評価する解析が使う。

use super::{OrbitPoint, TimelineSample, TimelineSpec, sample};
use crate::AstroError;
use crate::math::bisect;
use crate::orbit::{OrbitTrack, Trajectory};

/// 日照率が「全日照」「全日陰」の境の判定に使う幅（日照率は 0〜1。境のすぐ外側の値で符号を分ける）。
const SHADOW_EDGE_EPS: f64 = 1e-9;
/// 日陰の境の時刻の許容幅 [s]。
const SHADOW_EDGE_TOL_S: f64 = 1e-6;
/// 任意の時刻の時系列の点を求める道具（軌道の伝搬の表を 1 回だけ作る）。
pub struct TimelineEvaluator<'a> {
    spec: &'a TimelineSpec,
    orbit: Trajectory,
    track: OrbitTrack,
}

impl<'a> TimelineEvaluator<'a> {
    /// `spec` の期間（元期から `sampling.span` まで）で使う道具を作る。β の当て込みと軌道の検査は [`build`](super::build) と同じ。
    pub fn new(spec: &'a TimelineSpec) -> Result<Self, AstroError> {
        let orbit = spec.resolved_orbit()?;
        let step = spec.sampling.step.value();
        let split = (step / orbit.table_step_s()).ceil().max(1.0);
        let track = orbit.track(spec.sampling.span.value(), step / split)?;
        Ok(Self { spec, orbit, track })
    }

    /// 元期から `t_s` 秒後の時系列の点。
    pub fn at(&self, t_s: f64) -> TimelineSample {
        sample(
            self.spec,
            &OrbitPoint {
                t_s,
                epoch: self.orbit.epoch().plus_seconds(t_s),
                state: self.track.state_at(t_s),
            },
        )
    }

    /// 区間 `[t0, t1]` の中で、日照率が全日照から外れる・全日陰になる（またはその逆の）時刻（昇順）。
    /// 日照率はこの時刻で折れ曲がる（円柱影ならとぶ）ので、入熱を積分するときはここで区間を分ける。
    /// 区間の両端の日照率の符号の変化だけを探す（区間の中で戻る短い食は見つけない）。
    pub fn shadow_edges(&self, t0: f64, t1: f64) -> Vec<f64> {
        let (a, b) = (self.at(t0).sunlit, self.at(t1).sunlit);
        let mut edges: Vec<f64> = [1.0 - SHADOW_EDGE_EPS, SHADOW_EDGE_EPS]
            .into_iter()
            .filter(|th| (a > *th) != (b > *th))
            .filter_map(|th| bisect(|t| self.at(t).sunlit - th, t0, t1, SHADOW_EDGE_TOL_S, 100))
            .collect();
        edges.sort_by(f64::total_cmp);
        edges
    }
}
