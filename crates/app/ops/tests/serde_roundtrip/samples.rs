//! `serde_roundtrip` の Op の見本に使う部品（ネットとマヌーバ）。

use fune_model::design::{Net, NetKind};
use fune_model::value::Quantity;

pub fn net() -> Net {
    let mut n = Net::new("P5V_PL", NetKind::Power);
    n.label = Some("5 V 系".into());
    n.source = "PCDU.OUT2".parse().ok();
    n.rail = Some(Quantity::new(5.0, "V"));
    n
}

/// 目標の軌道と制約を持つマヌーバ。
pub fn maneuver() -> fune_model::design::Maneuver {
    serde_json::from_value(serde_json::json!({
        "id": "RAISE", "label": "軌道上昇", "category": "insertion", "method": "low_thrust",
        "thruster": "EPT", "after": "LEOP", "target": { "altitude": { "value": 600.0, "unit": "km" } },
        "constraints": { "sunlit_only": true, "mode": "THRUST" }
    }))
    .unwrap()
}

/// 範囲の制約・量化・確かめる先を持つ要求。
pub fn requirement() -> fune_model::design::Requirement {
    serde_json::from_value(serde_json::json!({
        "id": "R_MASS", "statement": "質量の上限", "statement_en": "Mass limit", "quantity": "Mass",
        "source": "sys_judgement",
        "constraint": { "kind": "at_most", "value": { "value": 12.0, "unit": "kg" } },
        "quantify": { "all_cases": true }, "verify": { "budget": "mass" }
    }))
    .unwrap()
}
