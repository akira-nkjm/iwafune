//! ヒータのある熱ネットワークの定常解。**サーモスタット付きヒータの定常の定義はここだけ**:
//! ヒータは、節点を `on_below`（下側の設定温度）に保つのに要る電力 P（0 ≤ P ≤ 容量）を出す。
//! ヒータなしで節点が `on_below` 以上ならオフ（P = 0）、容量いっぱいでも届かなければ飽和（P = 容量、節点は `on_below` 未満）。
//!
//! 解き方（有効集合法）: ヒータをオフ・制御中・飽和に分けて解く。制御中の節点は温度を `on_below` に固定して Newton 法で解き、
//! その節点の熱収支（ヒータ以外の正味の流入 F）から要る電力 P = −F を読む。飽和は容量を入熱に足す。
//! 解の後に整合を調べる: 制御中で P < 0 ならオフへ、P > 容量なら飽和へ、オフで節点が `on_below` 未満なら制御中へ、
//! 飽和で節点が `on_below` を超えたら制御中へ。動かなくなるまで繰り返す。

use super::heater::{HeaterSpec, ThermalHeaterState};
use super::newton::SteadySolution;
use super::{Compiled, invalid};
use crate::spec::AnalysisError;

/// 状態の整合を調べる温度の許容 [K]（Newton 法の収束より十分大きい）。
const STATE_TOLERANCE_K: f64 = 1e-7;

/// ヒータ 1 つの定常の結果。
#[derive(Debug, Clone)]
pub(crate) struct HeaterSteady {
    pub state: ThermalHeaterState,
    /// 出している電力 [W]（オフ・無効は 0、飽和は容量）
    pub power_w: f64,
}

/// ヒータありの定常解。
#[derive(Debug, Clone)]
pub(crate) struct HeatedSolution {
    pub solution: SteadySolution,
    /// ヒータの順
    pub heaters: Vec<HeaterSteady>,
}

impl Compiled {
    /// ヒータ `heaters`（`enabled` は働くか。入力の順）を含めた定常解。`q` はヒータを含まない入熱 [W]。
    pub(crate) fn solve_steady_heated(
        &self,
        q: &[f64],
        space_k: f64,
        guess: f64,
        heaters: &[HeaterSpec],
        enabled: &[bool],
    ) -> Result<HeatedSolution, AnalysisError> {
        use ThermalHeaterState::{Disabled, Off, Regulating, Saturated};
        let mut state: Vec<ThermalHeaterState> = enabled
            .iter()
            .map(|e| if *e { Off } else { Disabled })
            .collect();
        for _ in 0..(4 * heaters.len() + 8) {
            let (solution, power) = self.solve_with_states(q, space_k, guess, heaters, &state)?;
            let mut changed = false;
            for (i, h) in heaters.iter().enumerate() {
                let t = solution.temps[h.node];
                let next = match state[i] {
                    Disabled => Disabled,
                    Off if t < h.on_below - STATE_TOLERANCE_K => Regulating,
                    Regulating if power[i] < 0.0 => Off,
                    Regulating if power[i] > h.max_w => Saturated,
                    Saturated if t > h.on_below + STATE_TOLERANCE_K => Regulating,
                    s => s,
                };
                changed |= next != state[i];
                state[i] = next;
            }
            if !changed {
                let heaters = state
                    .iter()
                    .zip(&power)
                    .map(|(s, p)| HeaterSteady {
                        state: *s,
                        power_w: *p,
                    })
                    .collect();
                return Ok(HeatedSolution { solution, heaters });
            }
        }
        Err(invalid(
            "The heater states (off, regulating, saturated) do not settle".to_owned(),
        ))
    }

    /// 状態を固定して 1 回解き、ヒータの電力 [W] も返す（制御中は節点の熱収支から、飽和は容量、他は 0）。
    /// 制御中の電力は負や容量超えもそのまま返す（呼び出し側が状態を直す）。
    fn solve_with_states(
        &self,
        q: &[f64],
        space_k: f64,
        guess: f64,
        heaters: &[HeaterSpec],
        state: &[ThermalHeaterState],
    ) -> Result<(SteadySolution, Vec<f64>), AnalysisError> {
        let mut held = self.clone();
        let mut q = q.to_vec();
        for (h, s) in heaters.iter().zip(state) {
            match s {
                ThermalHeaterState::Regulating => held.fixed[h.node] = Some(h.on_below),
                ThermalHeaterState::Saturated => q[h.node] += h.max_w,
                ThermalHeaterState::Disabled | ThermalHeaterState::Off => {}
            }
        }
        let solution = held.solve_steady(&q, space_k, guess)?;
        let flow = held.net_heat(&solution.temps, &q, space_k);
        let power = heaters
            .iter()
            .zip(state)
            .map(|(h, s)| match s {
                ThermalHeaterState::Regulating => -flow[h.node],
                ThermalHeaterState::Saturated => h.max_w,
                ThermalHeaterState::Disabled | ThermalHeaterState::Off => 0.0,
            })
            .collect();
        Ok((solution, power))
    }
}
