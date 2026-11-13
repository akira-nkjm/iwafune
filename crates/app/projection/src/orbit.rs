//! 軌道の季節解析。既定値や物理計算を持たず、条件ファイルから写す。
use crate::quantity::{setting, setting_opt};
use crate::{Context, MissingInputs};
use fune_model::design::setting_dims as dims;
use sat_analysis::orbit::EclipseSeasonInput;

/// 軌道・ミッション期間・任意の季節刻み。
pub fn eclipse_season(ctx: &Context<'_>) -> Result<EclipseSeasonInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let orbit = ctx.orbit(&mut m);
    let o = ctx.design.conditions.orbit.as_ref();
    let duration = setting(
        &dims::ORBIT_DURATION,
        &[],
        o.and_then(|o| o.duration.as_ref()),
        "ミッション期間",
        &mut m,
    );
    let step = setting_opt(
        &dims::ORBIT_ECLIPSE_SEASON_STEP,
        &[],
        o.and_then(|o| o.eclipse_season.as_ref())
            .and_then(|s| s.step.as_ref()),
        &mut m,
    );
    m.finish(|| {
        Some(EclipseSeasonInput {
            orbit: orbit?,
            duration: duration?,
            step,
        })
    })
}
