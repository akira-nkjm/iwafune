//! 通信の回線・解析ケースを足すダイアログの選択肢（`GET /design` の `add_choices`）。
//! 部品の絞り込みは足りない入力の候補（`fixes`）と同じ関数、帯域と熱の環境条件は設計が持つ名前そのまま。

use crate::dto::AddChoices;
use crate::fixes::parts_of_roles;
use fune_model::design::Design;
use fune_model::role::Role;
use std::collections::BTreeSet;

/// 回線・ケースを足すときの選択肢（どれも並びは名前の順）。
pub(crate) fn add_choices(design: &Design) -> AddChoices {
    let mut transceivers = parts_of_roles(design, &[Role::Transceiver]);
    transceivers.sort();
    let bands: BTreeSet<&String> = design
        .conditions
        .ground
        .values()
        .flat_map(|g| g.links.keys())
        .collect();
    AddChoices {
        transceivers,
        bands: bands.into_iter().cloned().collect(),
        thermal_cases: design
            .analysis
            .thermal
            .iter()
            .flat_map(|t| t.cases.keys().cloned())
            .collect(),
    }
}
