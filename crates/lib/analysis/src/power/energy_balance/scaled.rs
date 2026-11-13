//! 発電に倍率をかけて、ケースとモードの並びを電力収支と同じ運用で評価し直す入口（`power.sizing` の時系列の逆算が使う）。
//!
//! 姿勢則ごとの発電の時系列は [`ScaledBalance::new`] で 1 回だけ作り、評価のたびに倍率をかける（発電は面積に比例する）。
//! 電池の積分・周回の収支・最長継続時間の最悪の開始・並びの切り替えは、電力収支の評価（`case`・`duration`・`sequence`）を
//! そのまま通す。判定に使う運用（[`EnergyBalanceEvaluation`]）も電力収支と同じ規則で決まる。

use super::context::{Context, LawGeneration, law_generation, profile, with_law};
use super::{
    EnergyBalanceBattery, EnergyBalanceEvaluation, EnergyBalanceInput, SolarProfile, case,
    duration, sequence,
};
use crate::spec::AnalysisError;

/// 倍率をかけたときの、判定に使う運用での値。
#[derive(Debug, Clone)]
pub struct ScaledJudgement {
    /// 判定をどの運用で評価したか
    pub evaluation: EnergyBalanceEvaluation,
    /// 判定に使う 1 周あたりのエネルギー余裕 [J]（電池の効率込み、容量の上下限で切る前）。回り切った周回が無ければ None
    pub margin: Option<f64>,
    /// 判定に使う運用での、刻みごとのエネルギーの増分 [J]（[`crate::power::battery::BatteryTrace::delta_unclamped`]。
    /// 容量の上下限で切る前なので容量によらない。最長継続時間の評価では最悪の開始のもの）
    pub delta: Vec<f64>,
}

/// 姿勢則ごとの発電を作り置いた電力収支。
pub struct ScaledBalance<'a> {
    input: &'a EnergyBalanceInput,
    battery: EnergyBalanceBattery,
    period_s: f64,
    /// 上段の姿勢則（`timeline.attitude`）での時系列
    main: SolarProfile,
    /// ケースごとの姿勢則での時系列（ケースが姿勢則を替えるときだけ）
    own: Vec<Option<SolarProfile>>,
    /// 基本のモードと並びのモードの姿勢則での発電（倍率をかける前）
    laws: LawGeneration,
}

impl<'a> ScaledBalance<'a> {
    /// 入力を検査し、姿勢則ごとの時系列を作る。
    pub fn new(input: &'a EnergyBalanceInput) -> Result<Self, AnalysisError> {
        let battery = EnergyBalanceBattery::combine(&input.batteries)?;
        input.array.validate()?;
        crate::power::mode_load::validate(&input.loads)?;
        sequence::validate(input)?;
        let own = input
            .cases
            .iter()
            .map(|c| {
                c.attitude
                    .as_ref()
                    .map(|law| profile(input, &with_law(input, law)))
                    .transpose()
            })
            .collect::<Result<_, _>>()?;
        let mut laws = LawGeneration::new();
        let base = input
            .base_mode
            .iter()
            .chain(input.sequence.first().map(|s| &s.mode));
        for mode in base.chain(input.sequence.iter().map(|s| &s.mode)) {
            if !laws.contains_key(mode)
                && let Some(g) = law_generation(input, mode)?
            {
                laws.insert(mode.clone(), g);
            }
        }
        Ok(Self {
            period_s: input.timeline.orbit().period_s(),
            main: profile(input, &input.timeline)?,
            input,
            battery,
            own,
            laws,
        })
    }

    /// ケースの数（`input.cases` と同じ並び）。
    pub fn case_count(&self) -> usize {
        self.input.cases.len()
    }

    /// 発電を `scale` 倍にして、ケース `index` を電力収支の判定と同じ運用で評価する。
    pub fn case(&self, index: usize, scale: f64) -> Result<ScaledJudgement, AnalysisError> {
        let c =
            self.input.cases.get(index).ok_or_else(|| {
                AnalysisError::InvalidInput(format!("ケース {index} がありません"))
            })?;
        let main = self.context(&self.main, scale);
        let own = self
            .own
            .get(index)
            .and_then(Option::as_ref)
            .map(|p| self.context(p, scale));
        let ctx = own.as_ref().unwrap_or(&main);
        let base = duration::BaseMode::new(&main)?;
        let (mut result, trace) = case::evaluate_case(ctx, c)?;
        let judged = duration::apply(ctx, c, base.as_ref(), &mut result)?;
        Ok(match judged {
            Some(delta) => ScaledJudgement {
                evaluation: result.evaluation,
                margin: result
                    .by_max_duration
                    .and_then(|d| d.energy_margin_per_orbit)
                    .map(|m| m.value()),
                delta,
            },
            None => ScaledJudgement {
                evaluation: result.evaluation,
                margin: result.energy_margin_per_orbit.map(|m| m.value()),
                delta: trace.delta_unclamped,
            },
        })
    }

    /// 発電を `scale` 倍にして、モードの並びで回す（並びが無ければ None）。
    pub fn sequence(&self, scale: f64) -> Result<Option<ScaledJudgement>, AnalysisError> {
        let main = self.context(&self.main, scale);
        Ok(
            sequence::evaluate(&main)?.map(|(s, trace)| ScaledJudgement {
                evaluation: s.balance.evaluation,
                margin: s.balance.energy_margin_per_orbit.map(|m| m.value()),
                delta: trace.delta_unclamped,
            }),
        )
    }

    fn context<'s>(&'s self, profile: &'s SolarProfile, scale: f64) -> Context<'s> {
        Context::new(self.input, &self.battery, profile, self.period_s).scaled(scale, &self.laws)
    }
}
