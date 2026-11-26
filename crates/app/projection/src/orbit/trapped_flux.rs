//! `orbit.trapped_flux` の射影: 軌道の 1 日分の標本の取り方（地上軌跡が 1 周する長さ。ケースの β があれば当てる）と、ミッション期間
//! （`orbit.total_dose` が区間を持たない設計で使うものと同じ）。既定値や物理の計算を持たない。
//! 放射線帯の地図と古い地磁気モデル（配布データ）が読み込まれていなければ、それも不足入力にして、そろえる手順を示す
//! （読み込みは `fune_service::data` が起動時に `sat_astro::radiation::registry` へ登録する。ここは読み込みの有無を見るだけ）。

use crate::{Context, MissingInputs};
use fune_model::Target;
use sat_analysis::orbit::{TrappedFlux, TrappedFluxInput};
use sat_analysis::spec::Analysis;
use sat_astro::radiation::RadiationTables;

/// 不足入力の理由に出す、データをそろえる手順。`radiation` は `fune_service::data` の群の ID（試験が一致を確かめる）。
pub const INSTALL_HINT: &str = "fune data fetch radiation (or, from a directory that holds the files: fune data fetch radiation --from <DIR>)";

/// `orbit.trapped_flux` の入力。
pub fn trapped_flux(ctx: &Context<'_>) -> Result<TrappedFluxInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let sampling = crate::timeline::day_sampling(ctx, &mut m);
    // ミッション期間は要求が与える（`input = "mission_duration"`）。無ければ入力不足
    let duration = m.need(
        ctx.mission_duration(),
        Context::mission_duration_target(),
        "Mission duration (an at_least requirement with input = \"mission_duration\")",
    );
    let missing = RadiationTables::installed().missing();
    if !missing.is_empty() {
        m.push(
            Target::Analysis {
                id: TrappedFlux::ID.to_owned(),
            },
            format!(
                "The AP-8/AE-8 trapped-radiation data is not installed (missing: {}). Install it with: {INSTALL_HINT}",
                missing.join(", ")
            ),
        );
    }
    m.finish(|| {
        Some(TrappedFluxInput {
            sampling: sampling?,
            mission_duration: duration?,
        })
    })
}
