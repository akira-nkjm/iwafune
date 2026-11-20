//! サーモスタット制御のヒータ（入力の型・モードでの有効/無効・オンオフの法則）。**ヒータの制御則（いつオンにし、いつオフにするか）はここだけ。**
//!
//! ヒータは節点の温度を測り、`on_below` を下回ればオン、`off_above` を上回ればオフ、その間は直前の状態を保つ（ヒステリシス）。
//! オンのときの発熱は `power × count`（消費電力はすべて熱になる）。サーモスタットのあるヒータは平均電力の発熱（`ThermalLoad`）に
//! 数えない（二重に数えない）。サーモスタットのないヒータは、これまでどおり部品として発熱に入る。
//!
//! モードとの関係: モードでのヒータの状態が停止状態（`off`）だけならサーモスタットを切っている（発熱 0）。
//! `off` 以外の状態の時間があれば（`on`、または `off` に `on` の duty を書いたとき）サーモスタットが働く
//! （電源の状態の読み方は `modes::power_states`。duty は電力側の見積りで、サーモスタットが働く間は熱側では使わない）。
//! ケースにモードが無い・モードにヒータの割り当てが無いときは働く。

use super::Compiled;
use crate::modes::{ModeDef, Population, Redundancy, power_states};
use crate::spec::AnalysisError;
use sat_units::typed::{Power, Qty, Temperature};
use serde::{Deserialize, Serialize};

/// 節点のヒータ（サーモスタット付き）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalHeater {
    /// 部品 ID
    pub id: String,
    /// 温度を測り、熱を入れる節点（部品の属する節点。発熱の場所を指定していればその節点）
    pub node: String,
    /// この温度を下回るとオン
    pub on_below: Qty<Temperature>,
    /// この温度を上回るとオフ（`on_below` より高い）
    pub off_above: Qty<Temperature>,
    /// 1 台がオンのときの消費電力（すべて熱になる）
    pub power: Qty<Power>,
    /// 員数（省略で 1）。オンのときの発熱は `power × count`
    #[serde(default = "crate::spec::default_count")]
    pub count: u32,
    /// 冗長の方式（モードでの電源の状態を数える）
    #[serde(default)]
    pub redundancy: Redundancy,
    /// 種別の状態の語彙（warm の予備状態を選ぶ）
    #[serde(default)]
    pub states: Vec<String>,
}

impl ThermalHeater {
    /// モード `mode` でサーモスタットが働くか（モードの説明を参照。`None` はモードの無いケース）。
    pub fn enabled_in(&self, mode: Option<&ModeDef>) -> Result<bool, AnalysisError> {
        let Some(a) = mode.and_then(|m| m.assignment(&self.id)) else {
            return Ok(true);
        };
        let s = power_states(
            a,
            Population {
                count: self.count,
                redundancy: self.redundancy,
                states: &self.states,
            },
        )?;
        Ok(s.on)
    }
}

/// 定常解でのヒータの状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ThermalHeaterState {
    /// モードでサーモスタットを切っている（発熱 0）
    Disabled,
    /// 働いているが、ヒータなしで節点が `on_below` 以上なのでオフ（発熱 0）
    Off,
    /// 節点を `on_below` に保つ電力を出している（0 < 電力 < 容量、または境界）
    Regulating,
    /// 容量いっぱいでも節点が `on_below` に届かない（発熱 = 容量）
    Saturated,
}

/// 設定温度の帯の幅の下限 [K]。時間積分が切り替えの時刻を温度の許容（`transient::heater_control`）で求めるので、
/// 切り替えた直後に反対側の設定温度の許容の中に入らないだけの幅を求める。
pub(crate) const MIN_BAND_K: f64 = 1e-3;

/// 添字に直したヒータ（SI の f64）。
#[derive(Debug, Clone)]
pub(crate) struct HeaterSpec {
    pub id: String,
    pub node: usize,
    /// オンにする温度 [K]
    pub on_below: f64,
    /// オフにする温度 [K]
    pub off_above: f64,
    /// オンのときの発熱 [W]（員数を掛けたもの）
    pub max_w: f64,
}

impl HeaterSpec {
    /// 温度 `t` [K] を測って次の状態を決める（`on` は直前の状態）。法則はモジュールの説明。
    /// 次の切り替えの温度から `tol` [K] 以内（越えた場合を含む）なら、切り替えに届いたとして状態を反転する
    /// （`tol = 0` なら、ちょうど設定温度にあるとき切り替える）。時間積分は切り替えの時刻を温度の許容 `tol` で求める。
    pub fn next_state(&self, on: bool, t: f64, tol: f64) -> bool {
        if self.margin(on, t) <= tol { !on } else { on }
    }

    /// 今の状態 `on` のまま温度が次の切り替えの温度に届くまでの余裕 [K]（オンなら `off_above` まで、オフなら `on_below` から。
    /// 0 以下で切り替わる）。時間積分はこの値が 0 になる時刻をイベントとして求める（`transient::heater_control`）。
    pub fn margin(&self, on: bool, t: f64) -> f64 {
        if on {
            self.off_above - t
        } else {
            t - self.on_below
        }
    }

    /// 初期温度 `t` [K] での最初の状態。帯の中はオフから始める（帯の中では直前の状態が決まらないので、保守的でない側に倒さない）。
    pub fn initial_state(&self, t: f64) -> bool {
        t < self.on_below
    }
}

impl Compiled {
    /// ヒータを検査して添字に直す（入力の順）。温度は正で `on_below < off_above`、消費電力は正で有限、
    /// 員数は 1 以上、固定温度の節点には置けず、1 つの節点にサーモスタットは 1 つ。
    pub(crate) fn heaters(
        &self,
        heaters: &[ThermalHeater],
    ) -> Result<Vec<HeaterSpec>, AnalysisError> {
        let mut out: Vec<HeaterSpec> = Vec::with_capacity(heaters.len());
        for h in heaters {
            let bad = |why: &str| AnalysisError::InvalidInput(format!("Heater {}: {why}", h.id));
            let (lo, hi) = (h.on_below.value(), h.off_above.value());
            if !(lo.is_finite() && hi.is_finite() && lo > 0.0 && hi - lo >= MIN_BAND_K) {
                return Err(bad(
                    "the on-below temperature must be positive and lower than the off-above temperature by at least 0.001 K",
                ));
            }
            let p = h.power.value();
            if !(p.is_finite() && p > 0.0 && h.count > 0) {
                return Err(bad("power must be positive and count must be 1 or more"));
            }
            let node = self.index(&h.node)?;
            if self.fixed[node].is_some() {
                return Err(bad("a heater cannot be on a fixed-temperature node"));
            }
            if let Some(other) = out.iter().find(|o| o.node == node) {
                return Err(bad(&format!(
                    "node {} already has the thermostat of heater {}",
                    h.node, other.id
                )));
            }
            out.push(HeaterSpec {
                id: h.id.clone(),
                node,
                on_below: lo,
                off_above: hi,
                max_w: p * f64::from(h.count),
            });
        }
        Ok(out)
    }
}

/// モード `mode` でサーモスタットが働くかをヒータごとに求める（入力の順）。
pub(crate) fn enabled_flags(
    heaters: &[ThermalHeater],
    mode: Option<&ModeDef>,
) -> Result<Vec<bool>, AnalysisError> {
    heaters.iter().map(|h| h.enabled_in(mode)).collect()
}

#[cfg(test)]
mod tests;
