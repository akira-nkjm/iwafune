//! 熱の定常解析の部品ごとの代表の温度（`dto::PartHeat`）。**節点と部品の対応（入力の `network.nodes[].parts`）と、
//! 代表の選び方（全ケースを通した節点の最高温度とそのケース・判定）はここだけ。** 熱の 3D は部品の ID で色を引くだけにする。
//! 節点に入っていない部品は載せない（Web は色を付けない）。

use crate::dto::{AnalysisRunResult, AnalysisRunStatus, PartHeat};
use crate::units::quantity_auto;
use sat_analysis::spec::Analysis;
use sat_analysis::thermal::{ThermalSteady, ThermalSteadyInput, ThermalSteadyOutput};
use std::collections::BTreeMap;

/// 実行の結果が熱の定常解析のときだけ、部品ごとの代表の温度を返す（部品の ID 順）。
pub(crate) fn part_heat(result: &AnalysisRunResult) -> Option<Vec<PartHeat>> {
    if result.id != ThermalSteady::ID || result.status != AnalysisRunStatus::Ok {
        return None;
    }
    let input: ThermalSteadyInput = serde_json::from_value(result.input.clone()?).ok()?;
    let output: ThermalSteadyOutput = serde_json::from_value(result.output.clone()?).ok()?;
    Some(of_parts(&input, &output))
}

/// 節点の包絡（全ケースの最高）を、節点に入る部品へ配る。部品が複数の節点に入っていれば、高いほうを代表にする。
pub(crate) fn of_parts(input: &ThermalSteadyInput, output: &ThermalSteadyOutput) -> Vec<PartHeat> {
    let mut by_part: BTreeMap<&str, PartHeat> = BTreeMap::new();
    for node in &input.network.nodes {
        let Some(env) = output.envelope.iter().find(|e| e.node == node.id) else {
            continue;
        };
        for part in &node.parts {
            let heat = PartHeat {
                part: part.id.clone(),
                node: node.id.clone(),
                per_part: node.per_part,
                temperature: quantity_auto(env.max.value(), "Temperature"),
                case: env.max_case.clone(),
                verdict: env.verdict,
            };
            match by_part.get(part.id.as_str()) {
                Some(cur) if cur.temperature.si >= heat.temperature.si => {}
                _ => {
                    by_part.insert(&part.id, heat);
                }
            }
        }
    }
    by_part.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp(k: f64) -> serde_json::Value {
        json!(k)
    }

    /// 2 節点（`panel` に A と B、`box` に C）。節点の ID と部品の ID は一致しない。
    #[test]
    fn representative_is_the_node_maximum_shared_by_its_parts() {
        let part = |id: &str| json!({ "id": id, "mass": 1.0, "specific_heat": 900.0 });
        let input: ThermalSteadyInput = serde_json::from_value(json!({
            "network": { "nodes": [
                { "id": "panel", "parts": [part("a"), part("b")] },
                { "id": "box", "parts": [part("c")], "per_part": true },
                { "id": "empty", "parts": [] },
            ] },
            "cases": [],
        }))
        .unwrap();
        let env = |node: &str, max: f64, case: &str, verdict: &str| {
            json!({
                "node": node, "min": temp(250.0), "min_case": "cold",
                "max": temp(max), "max_case": case,
                "ranges": [], "verdict": verdict,
            })
        };
        let output: ThermalSteadyOutput = serde_json::from_value(json!({
            "cases": [],
            "envelope": [
                env("panel", 320.0, "hot", "warning"),
                env("box", 300.0, "cold", "ok"),
            ],
            "margin": { "uncertainty": temp(0.0), "warning_band": temp(0.0) },
            "checks": [],
        }))
        .unwrap();
        let heat = of_parts(&input, &output);
        let ids: Vec<_> = heat.iter().map(|h| h.part.as_str()).collect();
        assert_eq!(ids, ["a", "b", "c"]);
        assert_eq!(heat[0].node, "panel");
        assert!(!heat[0].per_part && heat[2].per_part);
        assert_eq!(heat[0].temperature.si, 320.0);
        assert_eq!(heat[0].case, "hot");
        assert_eq!(heat[1].temperature.si, 320.0);
        assert_eq!(heat[2].node, "box");
        assert_eq!(heat[2].case, "cold");
    }
}
