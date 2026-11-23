//! 1 つの姿勢則での時系列と集計の文脈（ケース・最長継続時間・モードの並びの評価が共有する）。
//!
//! 発電は姿勢則の時系列（[`SolarProfile`]）から作り、倍率 `scale` をかけて使う（電力収支は 1、
//! `power.sizing` の時系列の逆算は面積の比。発電は面積に比例する）。

use super::{EnergyBalanceBattery, EnergyBalanceInput, SolarProfile};
use crate::spec::{AnalysisError, orbit_ranges};
use sat_astro::attitude::AttitudeLaw;
use sat_astro::orbit::Cycle;
use sat_astro::timeline::{self, TimelineSpec};
use std::collections::BTreeMap;
use std::ops::Range;

/// モードの姿勢則（`mode_attitudes`）での発電（倍率をかける前）。モードの ID → 標本ごとの発電 [W]。
pub(super) type LawGeneration = BTreeMap<String, Vec<f64>>;

/// 軌道・期間・刻み・β はそのままで姿勢則だけを替えた時系列。
pub(super) fn with_law(input: &EnergyBalanceInput, law: &AttitudeLaw) -> TimelineSpec {
    let mut spec = input.timeline.clone();
    spec.attitude = law.clone();
    spec
}

/// 時系列に沿った発電。
pub(super) fn profile(
    input: &EnergyBalanceInput,
    spec: &TimelineSpec,
) -> Result<SolarProfile, AnalysisError> {
    Ok(SolarProfile::build(&input.array, &timeline::build(spec)?))
}

/// モード `mode` の姿勢則での発電（倍率をかける前）。姿勢則の無いモードは None。
pub(super) fn law_generation(
    input: &EnergyBalanceInput,
    mode: &str,
) -> Result<Option<Vec<f64>>, AnalysisError> {
    match input.mode_attitudes.get(mode) {
        None => Ok(None),
        Some(law) => Ok(Some(
            profile(input, &with_law(input, law))?.total(input.life()),
        )),
    }
}

/// モードの姿勢則（`mode_attitudes`）での発電（倍率込み）。姿勢則の無いモードは `ctx` の発電のまま。
/// 作り置き（`laws`）があればそれを使い、無ければその場で時系列を作る。
pub(super) fn mode_generation(ctx: &Context, mode: &str) -> Result<Vec<f64>, AnalysisError> {
    let cached = ctx.laws.and_then(|l| l.get(mode));
    let unscaled = match cached {
        Some(g) => Some(g.clone()),
        None => law_generation(ctx.input, mode)?,
    };
    Ok(match unscaled {
        None => ctx.generation.clone(),
        Some(g) => g.into_iter().map(|p| p * ctx.scale).collect(),
    })
}

/// 1 つの姿勢則での時系列と集計（ケースが姿勢則を替えなければ全ケースで共通）。
pub(super) struct Context<'a> {
    pub(super) input: &'a EnergyBalanceInput,
    pub(super) battery: &'a EnergyBalanceBattery,
    pub(super) profile: &'a SolarProfile,
    /// 発電（倍率込み）[W]
    pub(super) generation: Vec<f64>,
    pub(super) ranges: Vec<Range<usize>>,
    /// 繰り返しの長さ（周期、または 1 回限りの期間。長さは [`Context::period_s`]）
    pub(super) cycle: Cycle,
    /// モードの姿勢則での発電の作り置き（無ければその場で作る）
    laws: Option<&'a LawGeneration>,
    /// 発電の倍率
    scale: f64,
}

impl<'a> Context<'a> {
    /// 繰り返しの長さ [s]（周回の区切り・1 周あたりの量に使う）。
    pub(super) fn period_s(&self) -> f64 {
        self.cycle.length_s()
    }

    pub(super) fn new(
        input: &'a EnergyBalanceInput,
        battery: &'a EnergyBalanceBattery,
        profile: &'a SolarProfile,
        cycle: Cycle,
    ) -> Self {
        Self {
            generation: profile.total(input.life()),
            ranges: orbit_ranges(&profile.t, cycle),
            input,
            battery,
            profile,
            cycle,
            laws: None,
            scale: 1.0,
        }
    }

    /// 発電に倍率 `scale` をかけ、モードの姿勢則での発電は作り置き `laws` から取る文脈。
    pub(super) fn scaled(mut self, scale: f64, laws: &'a LawGeneration) -> Self {
        for g in &mut self.generation {
            *g *= scale;
        }
        self.scale = scale;
        self.laws = Some(laws);
        self
    }

    /// 時刻・日照・周回はそのままで、発電だけを替えた文脈（モードの並びで姿勢則が切り替わるとき）。
    pub(super) fn with_generation(&self, generation: Vec<f64>) -> Self {
        Self {
            input: self.input,
            battery: self.battery,
            profile: self.profile,
            generation,
            ranges: self.ranges.clone(),
            cycle: self.cycle,
            laws: self.laws,
            scale: self.scale,
        }
    }
}
