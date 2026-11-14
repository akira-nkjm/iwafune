//! 噴射中の電力収支（`power.thrust_balance`）。数値伝搬のマヌーバの噴射のモード（`constraints.mode`）の負荷で
//! 電力収支 [`super::EnergyBalance`] を回し、低推力の上昇の間にバッテリーがもつかを見る。
//! **電池の積分は電力収支のもの**で、ここは噴射の仕方を電力収支の負荷のかけ方に直すだけ。
//!
//! - 日照中だけ噴く: 日照中は噴射のモード、日陰は基本のモード（[`EnergyBalanceLoad::SunlitEclipse`]）。
//! - 日陰でも噴く・デューティ 1: 噴射のモードを常に（[`EnergyBalanceLoad::Mode`]）。
//! - 日陰でも噴く・デューティ d < 1: 1 周を「噴射のモード d × 周期 → 基本のモード (1 − d) × 周期」の並びにして回す
//!   （噴く時間を 1 周の中でまとめる。刻みごとに散らす伝搬より放電が深く出る保守側）。
//! - 日照中だけ噴く・デューティ d < 1: 日照中ずっと噴くとみなす（保守側。[`ThrustBalanceNote::DutyAsFullSunlit`]）。
//! - 基本のモードは電力収支の入力の基本のモード（設定か並びの最初）。要るのに無ければ誤り。

use super::energy_balance::{
    EnergyBalance, EnergyBalanceCase, EnergyBalanceCaseResult, EnergyBalanceInput,
    EnergyBalanceLoad, EnergyBalanceSegment,
};
use crate::spec::{Analysis, AnalysisArea, AnalysisError, AnalysisKind, System};
use sat_units::typed::{Fraction, Qty};
use serde::{Deserialize, Serialize};

/// 噴射 1 つ（数値伝搬のマヌーバ）の仕方。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThrustLoad {
    pub maneuver: String,
    /// 噴射中のモード
    pub mode: String,
    #[serde(default)]
    pub sunlit_only: bool,
    /// デューティ（省略で 1）
    #[serde(default)]
    pub duty: Option<Fraction>,
}

/// 噴射中の電力収支の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThrustBalanceInput {
    /// 電力収支の入力（ケースと並びは噴射ごとに置き換える）
    pub balance: EnergyBalanceInput,
    pub thrusts: Vec<ThrustLoad>,
}

/// 噴射の仕方の近似の注記。**文は持たない**（service の語彙が引く）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ThrustBalanceNote {
    /// 日照中だけ噴くデューティ < 1 を、日照中ずっと噴くとみなした
    DutyAsFullSunlit,
    /// デューティ < 1 を 1 周の中でまとめて噴くとみなした
    DutyAsBlockPerOrbit,
}

/// 噴射 1 つの電力収支。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThrustBalanceResult {
    pub maneuver: String,
    pub mode: String,
    /// 噴かない間のモード（使わなければ None）
    pub base_mode: Option<String>,
    pub sunlit_only: bool,
    pub duty: Fraction,
    pub note: Option<ThrustBalanceNote>,
    /// 電力収支の結果（`id` はマヌーバ）
    pub balance: EnergyBalanceCaseResult,
}

/// `power.thrust_balance` の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThrustBalanceOutput {
    pub thrusts: Vec<ThrustBalanceResult>,
}

/// 噴射中の電力収支。
pub struct ThrustBalance;

fn base_mode(input: &EnergyBalanceInput) -> Option<String> {
    input
        .base_mode
        .clone()
        .or_else(|| input.sequence.first().map(|s| s.mode.clone()))
}

/// 噴射の仕方 → 電力収支の入力（ケース 1 つか、噴射と休みの並び）と、並びで回すか。
fn balance_input(
    input: &EnergyBalanceInput,
    t: &ThrustLoad,
    duty: f64,
    base: &str,
) -> (EnergyBalanceInput, bool) {
    let mut run = input.clone();
    run.cases.clear();
    run.sequence.clear();
    let block = !t.sunlit_only && duty < 1.0;
    if block {
        let period = input.timeline.orbit().period_s();
        run.sequence = vec![
            EnergyBalanceSegment {
                mode: t.mode.clone(),
                duration: Qty::from_raw(duty * period),
            },
            EnergyBalanceSegment {
                mode: base.to_owned(),
                duration: Qty::from_raw((1.0 - duty) * period),
            },
        ];
    } else {
        let load = if t.sunlit_only {
            EnergyBalanceLoad::SunlitEclipse {
                sunlit_mode: t.mode.clone(),
                eclipse_mode: base.to_owned(),
            }
        } else {
            EnergyBalanceLoad::Mode {
                mode: t.mode.clone(),
            }
        };
        run.cases = vec![EnergyBalanceCase {
            id: t.maneuver.clone(),
            label: t.maneuver.clone(),
            load,
            attitude: None,
            max_duration: None,
        }];
    }
    (run, block)
}

/// 噴射 1 つを電力収支で回す。
fn evaluate(
    input: &EnergyBalanceInput,
    t: &ThrustLoad,
) -> Result<ThrustBalanceResult, AnalysisError> {
    let duty = t.duty.map_or(1.0, Fraction::value);
    if !(duty > 0.0 && duty <= 1.0) {
        return Err(AnalysisError::InvalidInput(format!(
            "マヌーバ {} のデューティは 0 より大きく 1 以下にしてください（{duty}）",
            t.maneuver
        )));
    }
    let needs_base = t.sunlit_only || duty < 1.0;
    let base = needs_base.then(|| base_mode(input)).flatten();
    if needs_base && base.is_none() {
        return Err(AnalysisError::InvalidInput(format!(
            "マヌーバ {} の噴かない間の負荷に、電力収支の基本のモードが要ります",
            t.maneuver
        )));
    }
    let note = match (t.sunlit_only, duty < 1.0) {
        (true, true) => Some(ThrustBalanceNote::DutyAsFullSunlit),
        (false, true) => Some(ThrustBalanceNote::DutyAsBlockPerOrbit),
        _ => None,
    };
    let (run, block) = balance_input(input, t, duty, base.as_deref().unwrap_or_default());
    let out = EnergyBalance::run(&run)?;
    let mut balance = match (block, out.sequence, out.cases.into_iter().next()) {
        (true, Some(s), _) => s.balance,
        (false, _, Some(c)) => c,
        _ => {
            return Err(AnalysisError::InvalidInput(
                "電力収支の結果がありません".into(),
            ));
        }
    };
    balance.id = t.maneuver.clone();
    balance.label = t.maneuver.clone();
    Ok(ThrustBalanceResult {
        maneuver: t.maneuver.clone(),
        mode: t.mode.clone(),
        base_mode: base,
        sunlit_only: t.sunlit_only,
        duty: Fraction::new(duty),
        note,
        balance,
    })
}

impl Analysis for ThrustBalance {
    const ID: &'static str = "power.thrust_balance";
    const TITLE: &'static str = "噴射中の電力収支";
    const TITLE_EN: &'static str = "Energy balance while thrusting";
    const AREA: AnalysisArea = AnalysisArea::System(System::Power);
    const KIND: AnalysisKind = AnalysisKind::TimeSeries;
    const DESCRIPTION: &'static str = "数値伝搬のマヌーバの噴射のモードの負荷で電力収支を回し、噴射の間にバッテリーがもつかを見る";
    const DESCRIPTION_EN: &'static str = "Runs the energy balance with the firing mode of each propagated maneuver to check the battery survives the thrust arcs";

    type Input = ThrustBalanceInput;
    type Output = ThrustBalanceOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let thrusts = input
            .thrusts
            .iter()
            .map(|t| evaluate(&input.balance, t))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ThrustBalanceOutput { thrusts })
    }
}

#[cfg(test)]
mod tests;
