//! 電池の充放電（電力収支 `power.energy_balance` が使う）。
//!
//! 蓄えたエネルギー E [J] を `sat_astro::ode::rk4_linear_step`（`sat_astro::ode::rk4_step`）で積分する（積分器は書かない）。
//! 刻みの間のバス側の正味電力 P = 発電 − 負荷 は両端の値の線形補間とし、
//! `dE/dt = η_c · min(P, P_cmax)`（P > 0）、`dE/dt = P / η_d`（P ≤ 0）。
//! E は 0〜容量に収め、満充電で余る電力はシャントで捨てたものとして数える。

use crate::spec::{
    AnalysisError, Check, Verdict, ensure_fraction, ensure_non_negative, ensure_positive,
    quantity_text,
};
use sat_units::typed::{Energy, Fraction, Power, Qty, Voltage};
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

/// 集中電池モデルで先頭の電池の値を採るのに、電池どうしで値が違った欄。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryField {
    /// 公称電圧
    Voltage,
    /// 充電効率（省略は 1 として比べる）
    ChargeEfficiency,
    /// 放電効率（省略は 1 として比べる）
    DischargeEfficiency,
    /// 初期の SOC（省略は 1 として比べる）
    InitialSoc,
    /// 許容する DoD の上限
    DodLimit,
}

impl BatteryField {
    fn label(self) -> &'static str {
        match self {
            Self::Voltage => "Nominal voltage",
            Self::ChargeEfficiency => "Charge efficiency",
            Self::DischargeEfficiency => "Discharge efficiency",
            Self::InitialSoc => "Initial SOC",
            Self::DodLimit => "DoD limit",
        }
    }
}

/// 電池を合成した結果と、先頭の電池の値で代えた欄。
#[derive(Debug, Clone)]
pub struct CombinedBattery {
    /// 合成した電池（集中電池モデル）
    pub battery: EnergyBalanceBattery,
    /// 先頭の電池と値が違った欄（全部同じなら空）
    pub mismatched: Vec<BatteryField>,
}

impl CombinedBattery {
    /// 値が揃っていないときの注意（揃っていれば `None`）。**電池の不一致の判定はここだけ**で作り、
    /// 合成を呼ぶ解析（電力収支・サイジング）がそれぞれの `checks` に入れる。
    pub fn mismatch_check(&self) -> Option<Check> {
        if self.mismatched.is_empty() {
            return None;
        }
        let b = &self.battery;
        let value = |f: BatteryField| match f {
            BatteryField::Voltage => quantity_text::<Voltage>(b.voltage.value()),
            BatteryField::ChargeEfficiency => quantity_text::<Fraction>(b.charge_eff()),
            BatteryField::DischargeEfficiency => quantity_text::<Fraction>(b.discharge_eff()),
            BatteryField::InitialSoc => quantity_text::<Fraction>(b.start_soc()),
            BatteryField::DodLimit => quantity_text::<Fraction>(b.dod_limit),
        };
        let fields = self
            .mismatched
            .iter()
            .map(|f| format!("{} {}", f.label(), value(*f)))
            .collect::<Vec<_>>()
            .join(", ");
        Some(Check::new(
            "battery_mismatch",
            "Battery value mismatch",
            Verdict::Warning,
            format!(
                "Battery values differ. The lumped battery model uses the first battery's values ({fields}) for all, \
                 so the result depends on the order"
            ),
        ))
    }
}

impl EnergyBalanceBattery {
    /// 集中電池モデルに合成する。容量と充電電力の上限（バス側。電池ごとの値の和。1 つでも上限なしなら上限なし）は
    /// 合計し、電圧・効率・初期 SOC・DoD の上限は先頭の電池の値を採る（比は電池を増やしても変わらない）。
    /// 先頭と違う値があった欄は [`CombinedBattery::mismatched`] に返す。
    /// 各容量と電圧・効率を検査する。空なら入力エラー。
    pub fn combine(batteries: &[Self]) -> Result<CombinedBattery, AnalysisError> {
        let first = batteries
            .first()
            .ok_or_else(|| AnalysisError::InvalidInput("No battery".to_owned()))?;
        let mut total = first.clone();
        total.capacity = Qty::ZERO;
        let mut charge_limit = Some(Qty::ZERO);
        for battery in batteries {
            validate_battery(battery)?;
            total.capacity = total.capacity + battery.capacity;
            charge_limit = charge_limit
                .zip(battery.max_charge_power)
                .map(|(sum, p)| sum + p);
        }
        total.max_charge_power = charge_limit;
        validate_battery(&total)?;
        let differs = |f: fn(&Self) -> f64| batteries.iter().any(|b| f(b) != f(first));
        let mismatched = [
            (BatteryField::Voltage, differs(|b| b.voltage.value())),
            (BatteryField::ChargeEfficiency, differs(Self::charge_eff)),
            (
                BatteryField::DischargeEfficiency,
                differs(Self::discharge_eff),
            ),
            (BatteryField::InitialSoc, differs(Self::start_soc)),
            (BatteryField::DodLimit, differs(|b| b.dod_limit)),
        ]
        .into_iter()
        .filter_map(|(field, differ)| differ.then_some(field))
        .collect();
        Ok(CombinedBattery {
            battery: total,
            mismatched,
        })
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
    ensure_positive("Battery capacity", b.capacity.value())?;
    ensure_positive("Battery voltage", b.voltage.value())?;
    for (name, v) in [
        ("Battery charge efficiency", b.charge_eff()),
        ("Battery discharge efficiency", b.discharge_eff()),
    ] {
        ensure_fraction(name, v)?;
        ensure_positive(name, v)?;
    }
    ensure_fraction("Initial SOC", b.start_soc())?;
    ensure_fraction("DoD limit", b.dod_limit)?;
    if let Some(p) = b.max_charge_power {
        ensure_non_negative("Charge power limit", p.value())?;
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

/// [`simulate`] の刻みごとの増分 `delta`（[`BatteryTrace::delta_unclamped`]。容量によらない）から、
/// DoD が `dod_limit` を超えない最小の容量 [J]（[`simulate`] の逆。`power.sizing` の時系列の逆算が使う）。
///
/// 満充電で切る（シャント）積分では、満充電からの不足 `d` は `d_0 = (1 − start_soc)·容量`、
/// `d_{k+1} = max(0, d_k − delta_k)` なので、増分の累積を `X_k` とすると
/// `d_k = max(d_0 − X_k, max_{1≤j≤k} X_j − X_k)`（空にならない限り）。
/// `d_k ≤ dod_limit·容量` を容量について解くと
/// `容量 ≥ max(D / dod_limit, max(0, −min X) / (dod_limit − (1 − start_soc)))`（`D` は最深の下がり幅）。
/// 初期の不足だけで上限に届く（`dod_limit ≤ 1 − start_soc`）のに満充電から下がる分があれば None。
pub fn required_capacity(delta: &[f64], start_soc: f64, dod_limit: f64) -> Option<f64> {
    let (mut x, mut peak, mut drawdown, mut lowest) =
        (0.0_f64, f64::NEG_INFINITY, 0.0_f64, 0.0_f64);
    for de in delta {
        x += de;
        peak = peak.max(x);
        drawdown = drawdown.max(peak - x);
        lowest = lowest.min(x);
    }
    let headroom = dod_limit - (1.0 - start_soc);
    let from_start = -lowest;
    if dod_limit <= 0.0 {
        return (drawdown <= 0.0 && from_start <= 0.0).then_some(0.0);
    }
    let by_drawdown = drawdown / dod_limit;
    if from_start <= 0.0 {
        return Some(by_drawdown);
    }
    (headroom > 0.0).then(|| by_drawdown.max(from_start / headroom))
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
    fn required_capacity_by_hand() {
        // 増分: −600, +300, −900, +2000 J。累積 X = −600, −300, −1200, 800。
        // 満充電から: 最深の下がり幅 D = max(peak − X) = −300 − (−1200) = 900 と初めの 1200（X_0 = 0 から）→ 1200。
        let delta = [-600.0, 300.0, -900.0, 2000.0];
        let full = required_capacity(&delta, 1.0, 0.3).unwrap();
        assert!((full - 1200.0 / 0.3).abs() < 1e-9);
        // 初期 SOC 0.9: D = 900（j ≥ 1 の峰から）、初めの不足 0.1·C − X ≤ 0.3·C → C ≥ 1200 / 0.2 = 6000。
        let partial = required_capacity(&delta, 0.9, 0.3).unwrap();
        assert!((partial - 6000.0).abs() < 1e-9);
        assert!(required_capacity(&delta, 0.7, 0.3).is_none());
        assert_eq!(required_capacity(&[10.0, 20.0], 0.5, 0.3), Some(0.0));
    }

    #[test]
    fn required_capacity_inverts_simulate() {
        // 効率と充電の上限のある電池で、増分から求めた容量で回し直すと最大 DoD がちょうど上限になる。
        let t: Vec<f64> = (0..=40).map(|k| f64::from(k) * 30.0).collect();
        let net: Vec<f64> = t
            .iter()
            .map(|s| {
                if (s / 300.0).floor() as i64 % 2 == 0 {
                    12.0
                } else {
                    -9.0
                }
            })
            .collect();
        let mut b = battery();
        b.max_charge_power = Some(Qty::from_raw(10.0));
        for soc in [1.0, 0.95] {
            b.initial_soc = Some(soc);
            let delta = simulate(&b, &t, &net).delta_unclamped;
            let c = required_capacity(&delta, soc, 0.3).unwrap();
            let mut sized = b.clone();
            sized.capacity = Qty::from_raw(c);
            let tr = simulate(&sized, &t, &net);
            let min = tr.energy.iter().copied().fold(f64::INFINITY, f64::min);
            assert!(((1.0 - min / c) - 0.3).abs() < 1e-12, "{soc}");
            assert!(!tr.depleted);
        }
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

#[cfg(test)]
mod combine_tests;
