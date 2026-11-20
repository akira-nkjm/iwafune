//! 非定常のヒータの制御と集計。制御則は `network::heater`（[`HeaterSpec::next_state`]）。ここは時間積分の刻みごとに
//! 状態を更新し、オン時間・切り替え回数・最長のオン時間を数える。
//!
//! 状態は積分の刻みの間一定（発熱は刻みの間一定）で、刻みの終わりの温度で次の状態を決める（`observe`）。
//! 刻みはサンプルの間隔以下なので、切り替えは少なくともサンプルごとに評価され、設定温度の帯からのはみ出しは
//! 最大で 1 刻みの間の温度変化（オーバーシュート）になる。

use crate::thermal::heater::ThermalHeaterRun;
use crate::thermal::network::HeaterSpec;
use sat_units::typed::{Fraction, Qty};
use std::cell::RefCell;

/// 制御中の状態と集計。
struct State {
    /// ヒータごとの今の状態（オンか）
    on: Vec<bool>,
    /// ヒータごとにサーモスタットが働いているか（働かなければ常にオフ）
    enabled: Vec<bool>,
    /// 直前に集計した時刻 [s]
    t_prev: f64,
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
    state: RefCell<State>,
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
            t_prev: t0,
            window_start,
            on_time: vec![0.0; n],
            on_time_window: vec![0.0; n],
            off_in_window: vec![false; n],
            max_t_window: vec![f64::NEG_INFINITY; n],
            switchings: vec![0; n],
            max_on: vec![0.0; n],
            power: vec![Vec::new(); n],
        };
        Self {
            specs,
            state: RefCell::new(state),
        }
    }

    /// 今の状態の発熱を節点ごとの入熱 `q` [W] に足す。
    pub fn add_heat(&self, q: &mut [f64]) {
        let s = self.state.borrow();
        for (h, on) in self.specs.iter().zip(&s.on) {
            if *on {
                q[h.node] += h.max_w;
            }
        }
    }

    /// サンプルの時刻 `t` の始めに、働くか `enabled` を更新して状態を決め、そのときの発熱を記録する。
    pub fn begin_sample(&self, t: f64, temps: &[f64], enabled: &[bool]) {
        let mut s = self.state.borrow_mut();
        s.enabled.copy_from_slice(enabled);
        self.decide(&mut s, t, temps);
        let powers: Vec<f64> = self
            .specs
            .iter()
            .zip(&s.on)
            .map(|(h, on)| if *on { h.max_w } else { 0.0 })
            .collect();
        for (series, p) in s.power.iter_mut().zip(powers) {
            series.push(p);
        }
    }

    /// 積分の刻みが時刻 `t`・温度 `temps` で終わったとき: 刻みの間の状態で集計し、次の状態を決める。
    pub fn observe(&self, t: f64, temps: &[f64]) {
        let mut s = self.state.borrow_mut();
        let (a, b) = (s.t_prev, t);
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
        s.t_prev = t;
        self.decide(&mut s, t, temps);
    }

    /// 温度 `temps` を測って各ヒータの状態を決める（働かなければオフ）。切り替えを数える。
    fn decide(&self, s: &mut State, t: f64, temps: &[f64]) {
        for (i, h) in self.specs.iter().enumerate() {
            let next = s.enabled[i] && h.next_state(s.on[i], temps[h.node]);
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

    /// 時刻 `t_end`（最初の時刻 `t0`）までの結果。`inputs` はヒータの入力（同じ順）。
    pub fn finish(
        self,
        inputs: &[crate::thermal::network::ThermalHeater],
        t0: f64,
        t_end: f64,
    ) -> (Vec<ThermalHeaterRun>, Vec<Vec<f64>>) {
        let s = self.state.into_inner();
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
