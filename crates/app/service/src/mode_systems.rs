//! モードの見方（`POST /system_modes/{system}`）を持つ系の一覧。`query`（`DefsView.mode_systems`）と `system_modes`
//! （対応の判定）のどちらからも使うため、ほかの module に依存しない葉に置く（循環を避ける）。

use sat_analysis::spec::System;

/// モードの見方を持つ系（系の宣言の順）。対応する系の一覧はここだけ。
pub fn mode_systems() -> Vec<System> {
    fune_model::design::AnalysisSystem::ALL
        .iter()
        .map(|s| fune_projection::system::to_analysis(*s))
        .filter(|s| {
            matches!(
                s,
                System::Aocs | System::Cdh | System::Power | System::Thermal
            )
        })
        .collect()
}
