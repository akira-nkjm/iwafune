//! 電源の部品タブの役割・列の定義。
use crate::dto::{AnalysisLabel, PartRow, PowerPartsGroup, ValueData};
use fune_model::builtin::Role;

pub(super) fn groups(parts: &[PartRow]) -> Vec<PowerPartsGroup> {
    [
        (
            "loads",
            "負荷（状態ごとの電力）",
            "Loads (power per state)",
            vec!["power"],
            "power",
        ),
        (
            "solar",
            "太陽電池",
            "Solar Cells",
            vec![
                "area",
                "cell_efficiency",
                "normal",
                "packing_factor",
                "eol_factor",
            ],
            "cell_efficiency",
        ),
        (
            "batteries",
            "電池",
            "Batteries",
            vec![
                "capacity",
                "voltage",
                "charge_efficiency",
                "discharge_efficiency",
            ],
            "capacity",
        ),
    ]
    .into_iter()
    .map(|(key, ja, en, attributes, required)| PowerPartsGroup {
        key: key.into(),
        label: AnalysisLabel {
            label: ja.into(),
            label_en: en.into(),
        },
        attributes: attributes.into_iter().map(str::to_owned).collect(),
        parts: parts
            .iter()
            .filter(|p| {
                (key == "solar" && p.role == Some(Role::SolarPanel))
                    || (key == "batteries" && p.role == Some(Role::Battery))
                    || p.values
                        .get(required)
                        .is_some_and(|v| !matches!(v.value, ValueData::Unset))
            })
            .cloned()
            .collect(),
    })
    .collect()
}
