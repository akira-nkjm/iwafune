//! 時間積分の本体。出力のサンプルの間を、滑らかな区間に分けて `Dop853` で積分する。
//!
//! - 区間はサンプルの時刻・発熱の区間（モード）の切り替わり・日陰の境（日照率が折れ曲がる時刻。`TimelineEvaluator::shadow_edges`）で分ける。
//! - 外部入熱は右辺の中で時刻ごとに求める（`TimelineEvaluator::at` の幾何から `absorbed`。サンプルの間も連続）。
//! - ヒータは測る節点の温度が設定温度に届く時刻をイベントとして求めて区間を止め、切り替えて続ける（`HeaterControl`）。

use super::heater_control::HeaterControl;
use super::phases::Phases;
use crate::spec::AnalysisError;
use crate::thermal::environment::ResolvedEnvironment;
use crate::thermal::network::Compiled;
use crate::thermal::orbit_heat::{Geometry, absorbed};
use sat_astro::ode::{Dop853, OdeStats};
use sat_astro::timeline::TimelineEvaluator;

/// 区間の端をまとめる幅 [s]（これより近い端は同じ時刻とみなす）。
const MIN_SEGMENT_S: f64 = 1e-6;

/// 区間の端で外部入熱を評価するとき、端から内側へずらす幅 [s]。日陰の境の時刻は `SHADOW_EDGE_TOL_S`（1e-6 s）で求めるので、
/// それより外側にずらして、端の段（積分器は区間の両端でも右辺を評価する）が必ず区間の側の日照を見るようにする
/// （円柱影のように日照が跳ぶ場合に、境の反対側の値を拾わない）。
const EDGE_INSET_S: f64 = 5e-6;

/// 1 つの区間の中でヒータの切り替えで止まってよい回数（帯の幅が下限以上なら起きない。起きたら入力か実装の誤り）。
const MAX_EVENTS_PER_SEGMENT: usize = 100_000;

/// 時間積分の道具（入熱・発熱の区間・積分器）。
pub(super) struct Simulation<'a> {
    pub net: &'a Compiled,
    pub env: &'a ResolvedEnvironment,
    pub phases: &'a Phases,
    pub timeline: &'a TimelineEvaluator<'a>,
    pub space_k: f64,
    pub cfg: Dop853,
}

/// 積分の結果。
pub(super) struct Run {
    /// サンプルごとの節点温度 [K]
    pub history: Vec<Vec<f64>>,
    /// サンプルごとの外部入熱の合計 [W]
    pub absorbed: Vec<f64>,
    pub cost: OdeStats,
    pub segments: usize,
}

/// 積分の途中経過（続きの刻みの目安と費用）。
struct Progress {
    h: Option<f64>,
    cost: OdeStats,
    segments: usize,
}

impl Simulation<'_> {
    /// 各サンプルの時刻 `t` の温度（初期値 `temps` から）と外部入熱の合計を求める。
    pub fn run(
        &self,
        t: &[f64],
        mut temps: Vec<f64>,
        control: &mut HeaterControl,
    ) -> Result<Run, AnalysisError> {
        let mut history = Vec::with_capacity(t.len());
        let mut absorbed_total = Vec::with_capacity(t.len());
        let mut progress = Progress {
            h: None,
            cost: OdeStats::default(),
            segments: 0,
        };
        for (i, &ti) in t.iter().enumerate() {
            let q_ext = absorbed(self.net, self.env, &self.geometry(ti));
            absorbed_total.push(q_ext.iter().sum());
            let t_next = t.get(i + 1).copied();
            // 区間の始めの状態（働くか・設定温度に届いているか）を決めてから、その時刻の発熱を記録する
            control.begin_segment(ti, &temps, self.phases.enabled_at(ti));
            history.push(temps.clone());
            control.record_power();
            if let Some(next) = t_next {
                self.advance(ti, next, &mut temps, control, &mut progress)?;
            }
        }
        Ok(Run {
            history,
            absorbed: absorbed_total,
            cost: progress.cost,
            segments: progress.segments,
        })
    }

    fn geometry(&self, t: f64) -> Geometry {
        Geometry::from_sample(&self.timeline.at(t))
    }

    /// 区間 `[a, b]` の中の時刻 `t` の入熱の幾何時刻（端は `EDGE_INSET_S` だけ内側。短い区間は中点）。
    fn inside(a: f64, b: f64, t: f64) -> f64 {
        if b - a > 2.0 * EDGE_INSET_S {
            t.clamp(a + EDGE_INSET_S, b - EDGE_INSET_S)
        } else {
            0.5 * (a + b)
        }
    }

    /// サンプルの間 `[a, b]` の区間の端（`a` と `b` を含む昇順）。
    fn cuts(&self, a: f64, b: f64) -> Vec<f64> {
        let mut inner = self.phases.boundaries(a, b);
        inner.extend(self.timeline.shadow_edges(a, b));
        inner.sort_by(f64::total_cmp);
        let mut cuts = vec![a];
        for t in inner {
            if t - cuts[cuts.len() - 1] > MIN_SEGMENT_S && b - t > MIN_SEGMENT_S {
                cuts.push(t);
            }
        }
        cuts.push(b);
        cuts
    }

    /// `[a, b]` を区間に分けて、温度 `temps` を `b` まで進める。
    fn advance(
        &self,
        a: f64,
        b: f64,
        temps: &mut Vec<f64>,
        control: &mut HeaterControl,
        progress: &mut Progress,
    ) -> Result<(), AnalysisError> {
        for (k, w) in self.cuts(a, b).windows(2).enumerate() {
            let mid = 0.5 * (w[0] + w[1]);
            if k > 0 {
                control.begin_segment(w[0], temps, self.phases.enabled_at(mid));
            }
            self.segment(w[0], w[1], temps, control, progress)?;
        }
        Ok(())
    }

    /// 滑らかな 1 区間 `[a, b]`（発熱の区間は一定、ヒータの状態は切り替えまで一定）を積分する。
    fn segment(
        &self,
        a: f64,
        b: f64,
        temps: &mut Vec<f64>,
        control: &mut HeaterControl,
        progress: &mut Progress,
    ) -> Result<(), AnalysisError> {
        progress.segments += 1;
        let base = self.phases.at(0.5 * (a + b));
        let (mut t, mut events) = (a, 0);
        while t < b {
            let q_fixed = control.heat(base);
            let f = |tn: f64, x: &Vec<f64>| {
                let mut q = absorbed(self.net, self.env, &self.geometry(Self::inside(a, b, tn)));
                for (qi, fi) in q.iter_mut().zip(&q_fixed) {
                    *qi += fi;
                }
                self.net.derivative(x, &q, self.space_k)
            };
            let seg = self.cfg.integrate_segment(
                f,
                t,
                std::mem::take(temps),
                b,
                progress.h,
                |_, x| control.margin(x),
            )?;
            control.account(t, seg.t, &seg.y);
            progress.cost.add(&seg.stats);
            progress.h = Some(seg.h_next);
            *temps = seg.y;
            t = seg.t;
            if seg.event {
                control.switch(t, temps);
                events += 1;
                if events > MAX_EVENTS_PER_SEGMENT {
                    return Err(AnalysisError::InvalidInput(
                        "Heater switching did not settle (too many switchings in one interval)"
                            .to_owned(),
                    ));
                }
            }
        }
        Ok(())
    }
}
