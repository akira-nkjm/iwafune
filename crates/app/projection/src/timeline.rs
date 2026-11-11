//! 時系列の解析に渡す `TimelineSpec`（軌道・姿勢則・期間・刻み・β）と、地上局のパス・地磁気の平均に渡す `OrbitSampling` の組み立て。
//! 時系列そのものは sat-astro が作る。β はどれにもケースの指定を当てる。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::quantity::qty_opt;
use fune_model::Target;
use sat_astro::timeline::{OrbitSampling, TimelineSpec};
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
    let laws = ctx.design.analysis.aocs.as_ref().map(|a| &a.law);
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
    let span = orbit.period_s() * f64::from(orbits.max(1));
    let sampling = sampling(ctx, orbit, span, m);
    Some(TimelineSpec::from_sampling(sampling, law?))
}

/// 軌道 `orbits` 周分の標本の取り方（刻みは既定、β はケースの指定）。姿勢則の要らない集計（地磁気の平均）に使う。
pub fn orbits_sampling(
    ctx: &Context<'_>,
    orbits: u32,
    m: &mut MissingInputs,
) -> Option<OrbitSampling> {
    let orbit = ctx.orbit(m)?;
    let span = orbit.period_s() * f64::from(orbits.max(1));
    Some(sampling(ctx, orbit, span, m))
}

/// 1 日分の標本の取り方（地上局の可視・送信量。刻みは既定、β はケースの指定）。
pub fn day_sampling(ctx: &Context<'_>, m: &mut MissingInputs) -> Option<OrbitSampling> {
    let orbit = ctx.orbit(m)?;
    Some(sampling(
        ctx,
        orbit,
        sat_astro::constants::SECONDS_PER_DAY,
        m,
    ))
}

/// 期間 `span_s` [s]・既定の刻みで、ケースの β を当てた標本の取り方。
fn sampling(
    ctx: &Context<'_>,
    orbit: sat_astro::orbit::OrbitSpec,
    span_s: f64,
    m: &mut MissingInputs,
) -> OrbitSampling {
    let mut s = OrbitSampling::new(orbit, Qty::from_raw(span_s), Qty::from_raw(DEFAULT_STEP_S));
    s.beta = case_beta(ctx, m);
    s
}

/// ケースが指定する β 角（RAAN の置き換えは sat-astro が行う）。
pub fn case_beta(ctx: &Context<'_>, m: &mut MissingInputs) -> Option<Qty<Angle>> {
    let case = ctx.case?;
    let target = Target::case(case.id.as_str(), Some("beta"));
    qty_opt::<Angle>(case.beta.as_ref(), target, m)
}
