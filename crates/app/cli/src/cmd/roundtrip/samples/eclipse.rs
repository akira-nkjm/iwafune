//! 季節解析から保存したケースの往復見本。SetCase は日付・符号付き β を保つ。
use fune_service::dto::{Case, Op};
use serde_json::json;

pub(in super::super) fn eclipse_cases() -> Vec<Op> {
    [
        ("season_longest", "longest", -0.15),
        ("season_beta_min", "beta_min", -0.45),
    ]
    .into_iter()
    .map(|(id, at, beta)| {
        let mut case: Case = serde_json::from_value(json!({
            "description": format!("orbit.eclipse_season / {at} / 2027-07-01T00:00:00Z"),
            "mode": "NOMINAL", "beta": {"value": beta, "unit": "rad"}
        }))
        .unwrap();
        case.id = id.into();
        Op::SetCase { case }
    })
    .collect()
}
