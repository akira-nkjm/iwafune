//! 時系列の解析に渡す `TimelineSpec`（軌道・姿勢則・期間・刻み・β）と、地上局のパス・地磁気の平均に渡す `OrbitSampling` の組み立て。
//! 時系列そのものは sat-astro が作る。β はどれにもケースの指定を当てる。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::quantity::qty_opt;
use fune_model::Target;
use sat_analysis::spec::{SamplingSpan, daily_sampling, sampling};
use sat_astro::timeline::{OrbitSampling, TimelineSpec};
use sat_units::typed::{Angle, Qty};

/// ケースが指すモードの ID。
pub fn case_mode_id(ctx: &Context<'_>) -> Option<String> {
    ctx.case_mode().map(|m| m.id.to_string())
}

/// 姿勢則が書かれているモード（ケースがあればそのモードだけ）。
pub fn modes_with_law(ctx: &Context<'_>) -> Vec<String> {
    let laws = ctx.laws();
    if laws.is_empty() {
        return Vec::new();
    }
    match case_mode_id(ctx) {
        Some(id) => vec![id],
        None => ctx
            .ordered_modes()
            .into_iter()
            .filter(|mode| laws.contains_key(mode.id.as_str()))
            .map(|mode| mode.id.to_string())
            .collect(),
    }
}

/// 軌道 `orbits` 周分の時系列の仕様（モードの姿勢則を使う）。周回数が書かれていなければ `None`（既定は解析）。
pub fn timeline(
    ctx: &Context<'_>,
    mode: Option<&str>,
    orbits: Option<u32>,
    m: &mut MissingInputs,
) -> Option<TimelineSpec> {
    timeline_over(ctx, mode, SamplingSpan::Series(orbits), m)
}

/// 期間の取り方 `span`（解析が決める。履歴を見る期間か、1 周の平均か）の時系列の仕様（モードの姿勢則を使う）。
pub fn timeline_over(
    ctx: &Context<'_>,
    mode: Option<&str>,
    span: SamplingSpan,
    m: &mut MissingInputs,
) -> Option<TimelineSpec> {
    let orbit = ctx.orbit(m);
    let law = ctx.attitude_law(mode, m);
    let sampling = with_case_beta(ctx, sampling(orbit?, span), m);
    Some(TimelineSpec::from_sampling(sampling, law?))
}

/// 期間の取り方 `span` の標本の取り方（β はケースの指定）。姿勢則の要らない集計（地磁気の平均）に使う。
pub fn sampling_of(
    ctx: &Context<'_>,
    span: SamplingSpan,
    m: &mut MissingInputs,
) -> Option<OrbitSampling> {
    let orbit = ctx.orbit(m)?;
    Some(with_case_beta(ctx, sampling(orbit, span), m))
}

/// 1 日分の標本の取り方（地上局の可視・送信量。β はケースの指定）。
pub fn day_sampling(ctx: &Context<'_>, m: &mut MissingInputs) -> Option<OrbitSampling> {
    let orbit = ctx.orbit(m)?;
    Some(with_case_beta(ctx, daily_sampling(orbit), m))
}

/// 標本の取り方にケースの β を当てる。
fn with_case_beta(ctx: &Context<'_>, mut s: OrbitSampling, m: &mut MissingInputs) -> OrbitSampling {
    s.beta = case_beta(ctx, m);
    s
}

/// ケースが指定する β 角（RAAN の置き換えは sat-astro が行う）。
pub fn case_beta(ctx: &Context<'_>, m: &mut MissingInputs) -> Option<Qty<Angle>> {
    let case = ctx.case?;
    let target = Target::case(case.id.as_str(), Some("beta"));
    if case.beta.is_some() {
        ctx.note(target.clone());
    }
    qty_opt::<Angle>(case.beta.as_ref(), target, m)
}
