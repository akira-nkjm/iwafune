//! 時系列の解析に渡す `TimelineSpec`（軌道・姿勢則・期間・刻み）の組み立て。時系列そのものは sat-astro が作る。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::quantity::qty_opt;
use sat_astro::timeline::TimelineSpec;
use sat_units::typed::{Angle, Qty};

/// 時系列の既定の刻み [s]。
pub use sat_astro::timeline::DEFAULT_STEP_S;

/// 周回数を書かなかったときに解く周回数（電源の収支・熱の非定常・RW のアンロード）。
pub const DEFAULT_ORBITS: u32 = 3;

/// ケースが指すモードの ID。
pub fn case_mode_id(ctx: &Context<'_>) -> Option<String> {
    ctx.case_mode().map(|m| m.id.to_string())
}

/// 姿勢則が書かれているモード（ケースがあればそのモードだけ）。
pub fn modes_with_law(ctx: &Context<'_>) -> Vec<String> {
    let laws = ctx.design.analysis.attitude.as_ref().map(|a| &a.law);
    let Some(laws) = laws else { return Vec::new() };
    match case_mode_id(ctx) {
        Some(id) => vec![id],
        None => laws
            .keys()
            .filter(|k| ctx.design.modes.contains_key(k.as_str()))
            .cloned()
            .collect(),
    }
}

/// 軌道 `orbits` 周分の時系列の仕様（モードの姿勢則を使う）。
pub fn timeline(
    ctx: &Context<'_>,
    mode: Option<&str>,
    orbits: u32,
    m: &mut MissingInputs,
) -> Option<TimelineSpec> {
    let orbit = ctx.orbit(m);
    let law = ctx.attitude_law(mode, m);
    let orbit = orbit?;
    let period = orbit.period_s();
    let span = Qty::from_raw(period * f64::from(orbits.max(1)));
    let mut spec = TimelineSpec::new(orbit, law?, span, Qty::from_raw(DEFAULT_STEP_S));
    spec.beta = case_beta(ctx, m);
    Some(spec)
}

/// ケースが指定する β 角（RAAN の置き換えは sat-astro が行う）。
pub fn case_beta(ctx: &Context<'_>, m: &mut MissingInputs) -> Option<Qty<Angle>> {
    let case = ctx.case?;
    qty_opt::<Angle>(case.beta.as_ref(), &format!("cases.{}.beta", case.id), m)
}

/// 1 日分の時系列の期間 [s] と刻み [s]（可視・データ量の解析用）。
pub fn one_day() -> (f64, f64) {
    (sat_astro::constants::SECONDS_PER_DAY, DEFAULT_STEP_S)
}
