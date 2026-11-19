//! 内部発熱の区間（モードなど）を節点ごとの配列にし、時刻から引く。サーモスタットが働くかも区間ごと（モードごと）に決める。

use super::{DEFAULT_PHASE_DURATION_S, ThermalHeatPhase};
use crate::spec::{AnalysisError, Check};
use crate::thermal::network::{Compiled, ThermalHeater, enabled_flags};

/// 内部発熱の区間を節点ごとの配列にしたもの。
pub(super) struct Phases {
    /// (区間の終わり（周期の中の時刻）[s], 節点ごとの発熱 [W])
    pub(super) steps: Vec<(f64, Vec<f64>)>,
    /// 区間ごとの、ヒータのサーモスタットが働くか（`steps` と同じ順）
    enabled: Vec<Vec<bool>>,
    cycle_s: f64,
    zero: Vec<f64>,
    /// 発熱の区間が無いときのサーモスタット（すべて働く）
    all_enabled: Vec<bool>,
    pub(super) checks: Vec<Check>,
}

impl Phases {
    pub(super) fn new(
        net: &Compiled,
        phases: &[ThermalHeatPhase],
        heaters: &[ThermalHeater],
    ) -> Result<Self, AnalysisError> {
        let mut steps = Vec::with_capacity(phases.len());
        let mut enabled = Vec::with_capacity(phases.len());
        let mut end = 0.0;
        let mut checks = Vec::new();
        for (i, p) in phases.iter().enumerate() {
            let duration = p.duration.map_or(DEFAULT_PHASE_DURATION_S, |d| d.value());
            if duration.is_nan() || duration <= 0.0 {
                return Err(AnalysisError::InvalidInput(format!(
                    "Heat interval {} must have a positive length",
                    p.label
                )));
            }
            end += duration;
            let mut loads = p.loads.clone();
            if let Some(mode) = &p.mode_loads {
                let (mode_loads, warnings) = mode.evaluate()?;
                loads.extend(mode_loads);
                checks.extend(warnings.into_iter().map(|mut check| {
                    check.key = format!("phase:{i}:{}", check.key);
                    check
                }));
            }
            steps.push((end, net.loads(&loads)?));
            enabled.push(enabled_flags(
                heaters,
                p.mode_loads.as_ref().map(|l| &l.mode),
            )?);
        }
        Ok(Self {
            steps,
            enabled,
            checks,
            cycle_s: end,
            zero: vec![0.0; net.len()],
            all_enabled: vec![true; heaters.len()],
        })
    }

    /// 時刻 `t_s` の区間の添字（区間が無ければ `None`）。
    fn index_at(&self, t_s: f64) -> Option<usize> {
        if self.steps.is_empty() {
            return None;
        }
        let tau = t_s.rem_euclid(self.cycle_s);
        let i = self.steps.iter().position(|(end, _)| tau < *end);
        Some(i.unwrap_or(self.steps.len() - 1))
    }

    /// 時刻 `t_s` の発熱。
    pub(super) fn at(&self, t_s: f64) -> &[f64] {
        self.index_at(t_s).map_or(&self.zero, |i| &self.steps[i].1)
    }

    /// 時刻 `t_s` のヒータのサーモスタットが働くか。
    pub(super) fn enabled_at(&self, t_s: f64) -> &[bool] {
        self.index_at(t_s)
            .map_or(&self.all_enabled, |i| &self.enabled[i])
    }

    /// 周期で平均した発熱。
    pub(super) fn mean(&self) -> Vec<f64> {
        let mut out = self.zero.clone();
        let mut start = 0.0;
        for (end, q) in &self.steps {
            for (o, x) in out.iter_mut().zip(q) {
                *o += x * (end - start) / self.cycle_s;
            }
            start = *end;
        }
        out
    }
}
