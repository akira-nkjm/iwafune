//! 非定常の時間積分の設定（許容誤差と最大刻みの既定値）と、使った設定・費用の報告。
//!
//! 積分は `sat_astro::ode::Dop853`（適応刻みの 8 次・密出力）。許容誤差の既定値は解析が持つ（入力は `Option`）。

use super::ThermalTransientInput;
use crate::spec::AnalysisError;
use sat_astro::ode::{Dop853, OdeStats};
use sat_units::typed::{Qty, Temperature, Time};
use serde::{Deserialize, Serialize};

/// 相対許容誤差の既定値（刻みごとの誤差が `abs_tol + rel_tol·|T|` の中に入るように刻みを決める）。
pub const DEFAULT_REL_TOL: f64 = 1e-8;

/// 絶対許容誤差の既定値 [K]。
pub const DEFAULT_ABS_TOL_K: f64 = 1e-6;

/// 1 つの区間（サンプルの間・発熱の区間の切り替わり・日陰の境で分けたもの）で許す最大のステップ数。
const MAX_STEPS_PER_SEGMENT: usize = 5_000_000;

/// 使った積分の設定と、積分の費用。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalIntegration {
    /// 使った相対許容誤差（入力に無ければ既定値）
    pub rel_tol: f64,
    /// 使った絶対許容誤差（入力に無ければ既定値）
    pub abs_tol: Qty<Temperature>,
    /// 積分の刻みの上限（入力に無ければ無し）
    pub max_step: Option<Qty<Time>>,
    /// 滑らかな区間の数（サンプルの時刻・発熱の区間の切り替わり・日陰の境で分けたもの）。ヒータの切り替えで止まった分は含まない
    pub smooth_segments: u32,
    /// 受理した刻みの数
    pub accepted_steps: u32,
    /// 誤差が許容を超えて棄却した刻みの数
    pub rejected_steps: u32,
    /// 右辺（入熱の評価を含む）の評価の回数
    pub rhs_evaluations: u32,
    /// ヒータの切り替えのイベントで積分を止めた回数
    pub events: u32,
}

impl Default for ThermalIntegration {
    fn default() -> Self {
        Self {
            rel_tol: DEFAULT_REL_TOL,
            abs_tol: Qty::from_raw(DEFAULT_ABS_TOL_K),
            max_step: None,
            smooth_segments: 0,
            accepted_steps: 0,
            rejected_steps: 0,
            rhs_evaluations: 0,
            events: 0,
        }
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

impl ThermalIntegration {
    /// 設定と、積分の費用の合計から作る。
    pub(super) fn new(
        cfg: &Dop853,
        input: &ThermalTransientInput,
        segments: usize,
        cost: &OdeStats,
    ) -> Self {
        Self {
            rel_tol: cfg.rtol,
            abs_tol: Qty::from_raw(cfg.atol),
            max_step: input.max_step,
            smooth_segments: count(segments),
            accepted_steps: count(cost.accepted),
            rejected_steps: count(cost.rejected),
            rhs_evaluations: count(cost.rhs_evals),
            events: count(cost.events),
        }
    }
}

/// 入力から積分器の設定を作る（許容誤差は入力か既定値、最大刻みは入力があれば）。値の検査もここ。
pub(super) fn integrator(input: &ThermalTransientInput) -> Result<Dop853, AnalysisError> {
    let rtol = input.rel_tol.unwrap_or(DEFAULT_REL_TOL);
    let atol = input.abs_tol.map_or(DEFAULT_ABS_TOL_K, Qty::value);
    if !(rtol.is_finite() && atol.is_finite() && rtol >= 0.0 && atol >= 0.0 && rtol + atol > 0.0) {
        return Err(AnalysisError::InvalidInput(
            "Integration tolerances must be finite, not negative, and not both zero".to_owned(),
        ));
    }
    let max_step = input.max_step.map_or(f64::INFINITY, Qty::value);
    if max_step.is_nan() || max_step <= 0.0 {
        return Err(AnalysisError::InvalidInput(
            "The maximum integration step must be positive".to_owned(),
        ));
    }
    Ok(Dop853 {
        rtol,
        atol,
        max_step,
        max_steps: MAX_STEPS_PER_SEGMENT,
    })
}
