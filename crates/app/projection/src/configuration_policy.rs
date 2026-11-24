//! 解析が使う形態と選択元。打ち上げ向けの解析の宣言もここに置く。

use crate::case_policy::case_phase;
use fune_model::design::{Case, Design, Phase};
use sat_analysis::spec::Analysis;
use sat_analysis::structure::{LaunchFit, ModalEstimate, ModalFem, StaticLoad};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 解析が使った形態の選択元。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationSource {
    Explicit,
    Launch,
    Phase,
}

const LAUNCH_ANALYSES: &[&str] = &[
    LaunchFit::ID,
    StaticLoad::ID,
    ModalEstimate::ID,
    ModalFem::ID,
];

/// 明示 → 打ち上げ向けなら打ち上げ条件、それ以外ならケースのフェーズ → 基準。
pub fn effective_configuration<'a>(
    design: &'a Design,
    analysis_id: &str,
    case: Option<&'a Case>,
    explicit: Option<&'a str>,
) -> (Option<&'a str>, Option<ConfigurationSource>) {
    select(
        design,
        Some(analysis_id),
        || case_phase(design, case),
        explicit,
    )
}

/// フェーズは選択に必要なときだけ文脈から読む。
pub(crate) fn select<'a>(
    design: &'a Design,
    analysis_id: Option<&str>,
    phase: impl FnOnce() -> Option<&'a Phase>,
    explicit: Option<&'a str>,
) -> (Option<&'a str>, Option<ConfigurationSource>) {
    use ConfigurationSource::{Explicit, Launch, Phase};
    if let Some(id) = explicit {
        return (Some(id), Some(Explicit));
    }
    let (id, source) = if analysis_id.is_some_and(|id| LAUNCH_ANALYSES.contains(&id)) {
        let id = design
            .conditions
            .launch
            .as_ref()
            .and_then(|s| s.configuration.as_ref());
        (id, Launch)
    } else {
        (phase().and_then(|p| p.configuration.as_ref()), Phase)
    };
    (id.map(|id| id.as_str()), id.map(|_| source))
}
