//! 電池の充放電（電力収支 `power.energy_balance` が使う）。
//!
//! 蓄えたエネルギー E [J] を `sat_astro::ode::rk4_linear_step`（`sat_astro::ode::rk4_step`）で積分する（積分器は書かない）。
//! 刻みの間のバス側の正味電力 P = 発電 − 負荷 は両端の値の線形補間とし、
//! `dE/dt = η_c · min(P, P_cmax)`（P > 0）、`dE/dt = P / η_d`（P ≤ 0）。
//! E は 0〜容量に収め、満充電で余る電力はシャントで捨てたものとして数える。

use crate::spec::{AnalysisError, ensure_fraction, ensure_non_negative, ensure_positive};
use sat_units::typed::{Energy, Power, Qty, Voltage};
use serde::{Deserialize, Serialize};
/// 電池。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EnergyBalanceBattery {
    /// 容量（満充電で蓄えるエネルギー）
    pub capacity: Qty<Energy>,
    /// 公称電圧（電流の換算に使う）
    pub voltage: Qty<Voltage>,
    /// 充電効率（0〜1、バス側の電力 × 効率 = 蓄える電力）。省略時 1（損失なし）
    #[serde(default)]
    pub charge_efficiency: Option<f64>,
    /// 放電効率（0〜1、蓄えた電力 × 効率 = バス側の電力）。省略時 1（損失なし）
    #[serde(default)]
    pub discharge_efficiency: Option<f64>,
    /// 初期の SOC（0〜1）。省略時 1（満充電）
    #[serde(default)]
    pub initial_soc: Option<f64>,
    /// 許容する DoD の上限（0〜1）
    pub dod_limit: f64,
    /// 充電電力の上限（バス側）。省略時は上限なし
    #[serde(default)]
    pub max_charge_power: Option<Qty<Power>>,
}

impl EnergyBalanceBattery {
    /// 容量を合計し、電圧・効率・制約は先頭の電池を採用する集中電池モデル。
    /// 各容量と電圧・効率を検査する。空なら入力エラー。
    pub fn combine(batteries: &[Self]) -> Result<Self, AnalysisError> {
        let first = batteries
            .first()
            .ok_or_else(|| AnalysisError::InvalidInput("電池がありません".to_owned()))?;
        let mut total = first.clone();
        total.capacity = Qty::ZERO;
        for battery in batteries {
            validate_battery(battery)?;
            total.capacity = total.capacity + battery.capacity;
        }
        validate_battery(&total)?;
        Ok(total)
    }

    /// 充電効率（省略時 1）。
    pub fn charge_eff(&self) -> f64 {
        self.charge_efficiency.unwrap_or(1.0)
    }

    /// 放電効率（省略時 1）。
    pub fn discharge_eff(&self) -> f64 {
        self.discharge_efficiency.unwrap_or(1.0)
    }

    /// 初期の SOC（省略時 1 = 満充電）。
    pub fn start_soc(&self) -> f64 {
        self.initial_soc.unwrap_or(1.0)
    }
}

fn validate_battery(b: &EnergyBalanceBattery) -> Result<(), AnalysisError> {
    ensure_positive("電池の容量", b.capacity.value())?;
    ensure_positive("電池の電圧", b.voltage.value())?;
    for (name, v) in [
        ("電池の充電効率", b.charge_eff()),
        ("電池の放電効率", b.discharge_eff()),
    ] {
        ensure_fraction(name, v)?;
        ensure_positive(name, v)?;
    }
    ensure_fraction("初期 SOC", b.start_soc())?;
    ensure_fraction("DoD の上限", b.dod_limit)?;
    if let Some(p) = b.max_charge_power {
        ensure_non_negative("充電電力の上限", p.value())?;
    }
    Ok(())
}

use sat_astro::ode::rk4_linear_step;

/// 充放電の時系列（値は SI の生の f64。添字はサンプル）。
#[derive(Debug, Clone, Default)]
pub struct BatteryTrace {
    /// 蓄えたエネルギー [J]
    pub energy: Vec<f64>,
    /// 刻み `k → k+1` のエネルギーの増分 [J]（容量の上下限で切る前。周回のエネルギー余裕に使う）
    pub delta_unclamped: Vec<f64>,
    /// 充電電力（バス → 電池、バス側）[W]
    pub charge: Vec<f64>,
    /// 放電電力（電池 → バス、バス側）[W]
    pub discharge: Vec<f64>,
    /// 満充電で捨てた電力 [W]
    pub shunt: Vec<f64>,
    /// 電池が空になり負荷をまかなえなかったサンプルがあるか
    pub depleted: bool,
}

/// エネルギーの変化率 [W]（バス側の正味電力 `p` [W] から）。
fn energy_rate(battery: &EnergyBalanceBattery, p: f64) -> f64 {
    if p > 0.0 {
        let p = battery.max_charge_power.map_or(p, |m| p.min(m.value()));
        battery.charge_eff() * p
    } else {
        p / battery.discharge_eff()
    }
}

/// 正味電力 `net[k]` [W]（時刻 `t[k]` [s]）で電池を充放電する。
pub fn simulate(battery: &EnergyBalanceBattery, t: &[f64], net: &[f64]) -> BatteryTrace {
    let capacity = battery.capacity.value();
    let n = t.len().min(net.len());
    let mut trace = BatteryTrace::default();
    let mut e = battery.start_soc() * capacity;
    for k in 0..n {
        trace.energy.push(e);
        let full = e >= capacity;
        let empty = e <= 0.0;
        record_powers(&mut trace, battery, net[k], full, empty);
        if k + 1 == n {
            break;
        }
        let (t0, dt) = (t[k], t[k + 1] - t[k]);
        let (p0, p1) = (net[k], net[k + 1]);
        let rate = |w: f64, _: &[f64; 1]| [energy_rate(battery, p0 + (p1 - p0) * w)];
        let de = rk4_linear_step(rate, t0, &[0.0], dt)[0];
        trace.delta_unclamped.push(de);
        e = (e + de).clamp(0.0, capacity);
    }
    trace
}

/// サンプル 1 点の充電・放電・シャントの電力を記録する。
fn record_powers(
    trace: &mut BatteryTrace,
    battery: &EnergyBalanceBattery,
    p: f64,
    full: bool,
    empty: bool,
) {
    let (charge, shunt) = if p > 0.0 && !full {
        let c = battery.max_charge_power.map_or(p, |m| p.min(m.value()));
        (c, p - c)
    } else {
        (0.0, p.max(0.0))
    };
    let discharge = (-p).max(0.0);
    if discharge > 0.0 && empty {
        trace.depleted = true;
    }
    trace.charge.push(charge);
    trace.discharge.push(discharge);
    trace.shunt.push(shunt);
}

#[cfg(test)]
mod tests {
    use super::*;
    use sat_units::typed::Qty;

    fn battery() -> EnergyBalanceBattery {
        EnergyBalanceBattery {
            capacity: Qty::from_raw(36_000.0), // 10 Wh
            voltage: Qty::from_raw(8.0),
            charge_efficiency: Some(0.9),
            discharge_efficiency: Some(0.95),
            initial_soc: Some(1.0),
            dod_limit: 0.3,
            max_charge_power: None,
        }
    }

    #[test]
    fn constant_discharge_by_hand() {
        // 9.5 W を 600 s 引く → 電池からは 9.5 / 0.95 = 10 W、6000 J 減る
        let t: Vec<f64> = (0..=60).map(|k| f64::from(k) * 10.0).collect();
        let net = vec![-9.5; t.len()];
        let tr = simulate(&battery(), &t, &net);
        let last = *tr.energy.last().unwrap();
        assert!((last - 30_000.0).abs() < 1e-6);
        assert!(!tr.depleted);
        assert!((tr.discharge[0] - 9.5).abs() < 1e-12);
    }

    #[test]
    fn charge_with_efficiency_and_full_clamp() {
        // SOC 0.95 から 10 W で充電 → 9 W で蓄える。1800 J を 200 s で満たし、その後はシャント
        let mut b = battery();
        b.initial_soc = Some(0.95);
        let t: Vec<f64> = (0..=30).map(|k| f64::from(k) * 10.0).collect();
        let net = vec![10.0; t.len()];
        let tr = simulate(&b, &t, &net);
        assert!((tr.energy[20] - 36_000.0).abs() < 1e-6);
        assert!((tr.energy[10] - (34_200.0 + 900.0)).abs() < 1e-6);
        assert_eq!(tr.charge[25], 0.0);
        assert!((tr.shunt[25] - 10.0).abs() < 1e-12);
        // 切る前の増分は 9 W × 10 s
        assert!((tr.delta_unclamped[25] - 90.0).abs() < 1e-9);
    }

    #[test]
    fn linear_ramp_is_integrated_exactly() {
        // 0 → −19 W へ線形に下がる 100 s: 平均 −9.5 W → 電池からは 1000 J
        let t = [0.0, 100.0];
        let net = [0.0, -19.0];
        let tr = simulate(&battery(), &t, &net);
        assert!((tr.delta_unclamped[0] + 1000.0).abs() < 1e-9);
    }

    #[test]
    fn depletion_is_flagged() {
        let mut b = battery();
        b.initial_soc = Some(0.0);
        let tr = simulate(&b, &[0.0, 10.0], &[-5.0, -5.0]);
        assert!(tr.depleted);
        assert_eq!(tr.energy[1], 0.0);
    }
}
