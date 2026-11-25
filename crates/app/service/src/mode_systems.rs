//! モードの見方（`POST /system_modes/{system}`）を持つ系の一覧。`query`（`DefsView.mode_systems`）と `system_modes`
//! （対応の判定）のどちらからも使うため、ほかの module に依存しない葉に置く（循環を避ける）。
//! 系ごとの既定（設計のモードのページの合計の系・系統図で最初は隠す系）も、Web が系の名前を持たずに済むようここで返す。

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

/// 設計のモードのページ（系を選ばない）が足もとの合計を問い合わせる系。合計の行（`ModeSummaryScope::Totals`）を持つのは電源。
pub fn totals_system() -> System {
    System::Power
}

/// 系統図で最初は隠す系。信号も電力もつながない構造は、描くと線が増えるだけなので、指定がないときは隠す。
pub fn diagram_hidden_systems() -> Vec<System> {
    vec![System::Structure]
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 合計を問い合わせる系はモードの見方を持ち、系統図で隠す系は解析の系の一覧にある。
    #[test]
    fn web_defaults_name_known_systems() {
        assert!(mode_systems().contains(&totals_system()));
        let all: Vec<System> = fune_model::design::AnalysisSystem::ALL
            .iter()
            .map(|s| fune_projection::system::to_analysis(*s))
            .collect();
        assert!(
            diagram_hidden_systems()
                .iter()
                .all(|system| all.contains(system))
        );
    }
}
