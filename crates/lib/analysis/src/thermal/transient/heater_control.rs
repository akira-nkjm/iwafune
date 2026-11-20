//! 非定常のヒータの制御と集計。制御則は `network::heater`（[`HeaterSpec`]）。ここは時間積分の区間ごとに状態を持ち、
//! オン時間・切り替え回数・最長のオン時間を数える。
//!
//! 状態は切り替えまで一定（発熱は一定）。切り替えは、測る節点の温度が設定温度に届く時刻を積分器がイベントとして
//! 求める（[`HeaterControl::margin`] が 0 になる時刻。`Dop853::integrate_segment`）ので、切り替えの時刻は積分の刻みに
//! 量子化されず、切り替えたときの温度は設定温度に [`EVENT_TOL_K`] で一致する（帯からはみ出さない）。
//! 切り替えた直後は反対側の設定温度までの余裕が帯の幅（`MIN_BAND_K` 以上）あるので、同じ時刻に続けて切り替わらない。

use crate::thermal::heater::ThermalHeaterRun;
use crate::thermal::network::HeaterSpec;
use sat_units::typed::{Fraction, Qty};

/// 切り替えの温度の許容 [K]。イベントの時刻の求め方の誤差（刻み内の補間と二分法）よりずっと大きく、帯の幅の下限
/// （`network::heater::MIN_BAND_K`）よりずっと小さい。
pub(super) const EVENT_TOL_K: f64 = 1e-5;

/// 制御中の状態と集計。
struct State {
    /// ヒータごとの今の状態（オンか）
    on: Vec<bool>,
    /// ヒータごとにサーモスタットが働いているか（働かなければ常にオフ）
    enabled: Vec<bool>,
    /// 最後の 1 周の始まり [s]
    window_start: f64,
    on_time: Vec<f64>,
    on_time_window: Vec<f64>,
    /// 最後の 1 周にオフの時間があったか
    off_in_window: Vec<bool>,
    /// 最後の 1 周の節点の最高温度 [K]
    max_t_window: Vec<f64>,
    switchings: Vec<u32>,
    on_since: Vec<Option<f64>>,
    max_on: Vec<f64>,
    /// サンプルごとの発熱 [W]（ヒータの順）
    power: Vec<Vec<f64>>,
}

/// ヒータの制御器。
pub(super) struct HeaterControl<'a> {
    specs: &'a [HeaterSpec],
    state: State,
}

impl<'a> HeaterControl<'a> {
    /// 時刻 `t0` の初期温度 `temps`・働くか `enabled` から始める。`window_start` は最後の 1 周の始まり。
    pub fn new(
        specs: &'a [HeaterSpec],
        temps: &[f64],
        enabled: &[bool],
        t0: f64,
        window_start: f64,
    ) -> Self {
        let n = specs.len();
        let on: Vec<bool> = specs
            .iter()
            .zip(enabled)
            .map(|(h, e)| *e && h.initial_state(temps[h.node]))
            .collect();
        let state = State {
            on_since: on.iter().map(|o| o.then_some(t0)).collect(),
            on,
            enabled: enabled.to_vec(),
            window_start,
            on_time: vec![0.0; n],
            on_time_window: vec![0.0; n],
            off_in_window: vec![false; n],
            max_t_window: vec![f64::NEG_INFINITY; n],
            switchings: vec![0; n],
            max_on: vec![0.0; n],
            power: vec![Vec::new(); n],
        };
        Self { specs, state }
    }

    /// 発熱の区間の入熱 `base` [W]（節点ごと）に、今の状態のヒータの発熱を足した入熱。
    pub fn heat(&self, base: &[f64]) -> Vec<f64> {
        let mut q = base.to_vec();
        for (h, on) in self.specs.iter().zip(&self.state.on) {
            if *on {
                q[h.node] += h.max_w;
            }
        }
        q
    }

    /// 温度 `temps` での、働いているヒータの次の切り替えまでの余裕 [K] の最小値（イベント関数。0 以下で切り替わる）。
    /// 働いているヒータが無ければ無限大。
    pub fn margin(&self, temps: &[f64]) -> f64 {
        let s = &self.state;
        self.specs
            .iter()
            .enumerate()
            .filter(|(i, _)| s.enabled[*i])
            .map(|(i, h)| h.margin(s.on[i], temps[h.node]))
            .fold(f64::INFINITY, f64::min)
    }

    /// 発熱の区間の始めに、働くか `enabled` を更新し、温度 `temps` が設定温度に届いているヒータを切り替える
    /// （働かないヒータはオフにする）。時刻は `t`。
    pub fn begin_segment(&mut self, t: f64, temps: &[f64], enabled: &[bool]) {
        self.state.enabled.copy_from_slice(enabled);
        self.switch(t, temps);
    }

    /// 時刻 `t` の温度 `temps` で設定温度に届いている（`EVENT_TOL_K` 以内）ヒータを切り替える。働かないヒータはオフにする。
    /// 切り替えを数える。積分器がイベントで止めたときに呼ぶ。
    pub fn switch(&mut self, t: f64, temps: &[f64]) {
        let s = &mut self.state;
        for (i, h) in self.specs.iter().enumerate() {
            let next = s.enabled[i] && h.next_state(s.on[i], temps[h.node], EVENT_TOL_K);
            if next == s.on[i] {
                continue;
            }
            s.switchings[i] += 1;
            if next {
                s.on_since[i] = Some(t);
            } else if let Some(since) = s.on_since[i].take() {
                s.max_on[i] = s.max_on[i].max(t - since);
            }
            s.on[i] = next;
        }
    }

    /// サンプルの時刻の発熱 [W] を記録する。
    pub fn record_power(&mut self) {
        let s = &mut self.state;
        for ((series, on), h) in s.power.iter_mut().zip(&s.on).zip(self.specs) {
            series.push(if *on { h.max_w } else { 0.0 });
        }
    }

    /// 区間 `[a, b]`（今の状態が続いた）が温度 `temps` で終わったとき、オン時間などを集計する。
    pub fn account(&mut self, a: f64, b: f64, temps: &[f64]) {
        let s = &mut self.state;
        let in_window = (b - a.max(s.window_start)).max(0.0);
        for (i, h) in self.specs.iter().enumerate() {
            if s.on[i] {
                s.on_time[i] += b - a;
                s.on_time_window[i] += in_window;
            } else if in_window > 0.0 {
                s.off_in_window[i] = true;
            }
            if b > s.window_start {
                s.max_t_window[i] = s.max_t_window[i].max(temps[h.node]);
            }
        }
    }

    /// 時刻 `t_end`（最初の時刻 `t0`）までの結果。`inputs` はヒータの入力（同じ順）。
    pub fn finish(
        self,
        inputs: &[crate::thermal::network::ThermalHeater],
        t0: f64,
        t_end: f64,
    ) -> (Vec<ThermalHeaterRun>, Vec<Vec<f64>>) {
        let s = self.state;
        let span = t_end - t0;
        let window = t_end - s.window_start;
        let ratio = |x: f64, whole: f64| Fraction::new(if whole > 0.0 { x / whole } else { 0.0 });
        let runs = self
            .specs
            .iter()
            .zip(inputs)
            .enumerate()
            .map(|(i, (h, input))| {
                let open = s.on_since[i].map_or(0.0, |since| t_end - since);
                let duty = ratio(s.on_time[i], span);
                ThermalHeaterRun {
                    id: input.id.clone(),
                    node: input.node.clone(),
                    capacity: Qty::from_raw(h.max_w),
                    on_below: Qty::from_raw(h.on_below),
                    off_above: Qty::from_raw(h.off_above),
                    duty_fraction: duty,
                    duty_fraction_last_orbit: ratio(s.on_time_window[i], window),
                    mean_power: Qty::from_raw(duty.value() * h.max_w),
                    switchings: s.switchings[i],
                    max_on_duration: Qty::from_raw(s.max_on[i].max(open)),
                    saturated: s.enabled[i]
                        && !s.off_in_window[i]
                        && s.on_time_window[i] > 0.0
                        && s.max_t_window[i] < h.on_below,
                }
            })
            .collect();
        (runs, s.power)
    }
}
