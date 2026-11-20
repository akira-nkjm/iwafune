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

/// モードの表の部品ごとの列の出し方。設計のモードのページ（`system` が null）と電源は状態ごとの電力、ほかの系は状態の名前。
/// 列の見出しと行の文字をどちらにするかの正本はここだけ（Web は系の名前で決めない）。
pub fn mode_levels() -> Vec<crate::dto::ModeLevelsView> {
    use crate::dto::{ModeLevels, ModeLevelsView};
    let of = |system: Option<System>| ModeLevelsView {
        system,
        levels: if matches!(system, None | Some(System::Power)) {
            ModeLevels::PowerLevels
        } else {
            ModeLevels::States
        },
    };
    std::iter::once(None)
        .chain(mode_systems().into_iter().map(Some))
        .map(of)
        .collect()
}
