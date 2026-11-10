//! 出力の中の判定（`checks`）を、出てきた順に平らに集める。期間不足（出力の `span_fallback`）は語彙から注意を作る。

use super::labels::span_check;
use crate::dto::CheckView;
use sat_analysis::spec::{Check, SpanFallback, Verdict};
use serde_json::Value;

/// 期間が 1 周に満たないときの代わりの集計を入れる欄（sat-analysis の出力）。
const SPAN_FALLBACK: &str = "span_fallback";

/// 出力の中の `checks` を、出てきた順に集める（`series` の中は見ない）。
/// 場所（JSON Pointer）と、`checks` を持つオブジェクトの `label`（無ければ `id`）を添える。
/// `span_fallback` があれば、その場所に期間不足の注意（key `span`、文は語彙の日本語と英語）を足す。
pub fn collect_checks(output: Option<&Value>) -> Vec<CheckView> {
    let mut checks = Vec::new();
    if let Some(output) = output {
        walk(output, "", &mut checks);
    }
    checks
}

fn walk(value: &Value, at: &str, out: &mut Vec<CheckView>) {
    match value {
        Value::Object(map) => {
            let scope = ["label", "id"]
                .iter()
                .find_map(|k| map.get(*k).and_then(Value::as_str))
                .map(str::to_owned);
            let scope = if at.is_empty() { None } else { scope };
            for (key, child) in map {
                let here = format!("{at}/{}", pointer_token(key));
                if key == "checks" {
                    if let Ok(checks) = serde_json::from_value::<Vec<Check>>(child.clone()) {
                        out.extend(checks.into_iter().enumerate().map(|(i, c)| CheckView {
                            key: c.key,
                            label: c.label,
                            label_en: None,
                            verdict: c.verdict,
                            message: c.message,
                            message_en: None,
                            at: format!("{here}/{i}"),
                            scope: scope.clone(),
                        }));
                    }
                } else if key == SPAN_FALLBACK {
                    if let Ok(Some(f)) =
                        serde_json::from_value::<Option<SpanFallback>>(child.clone())
                    {
                        out.push(span_view(f, here, scope.clone()));
                    }
                } else if key != "series" {
                    walk(child, &here, out);
                }
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                walk(item, &format!("{at}/{i}"), out);
            }
        }
        _ => {}
    }
}

/// 期間不足の注意。
fn span_view(f: SpanFallback, at: String, scope: Option<String>) -> CheckView {
    let (label, message) = span_check(f);
    CheckView {
        key: "span".to_owned(),
        label: label.label,
        label_en: Some(label.label_en),
        verdict: Verdict::Warning,
        message: message.label,
        message_en: Some(message.label_en),
        at,
        scope,
    }
}

/// JSON Pointer（RFC 6901）のキーの書き方（`~` → `~0`、`/` → `~1`）。
fn pointer_token(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod tests {
    use super::collect_checks;
    use serde_json::json;

    #[test]
    fn collects_only_checks_and_skips_series() {
        let check =
            |key, verdict| json!({"key": key, "label": key, "verdict": verdict, "message": ""});
        let output = json!({"cases": [{"checks": [check("first", "warning")]}, {"checks": [check("second", "violation")]}], "series": {"checks": [check("ignored", "ok")]}, "other": [check("not_checks", "ok")]});
        let checks = collect_checks(Some(&output));
        assert_eq!(
            checks.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(),
            ["first", "second"]
        );
        assert_eq!(
            sat_analysis::spec::Verdict::worst(checks.iter().map(|c| c.verdict)),
            Some(sat_analysis::spec::Verdict::Violation)
        );
        assert!(collect_checks(None).is_empty());
    }

    #[test]
    fn checks_carry_a_unique_place_and_their_owner_name() {
        // 電力収支のように、モードごとに同じ key の判定が並ぶ（React のキーや表の行を区別できるように）
        let check = |key| json!({"key": key, "label": key, "verdict": "ok", "message": ""});
        let output = json!({
            "checks": [check("total")],
            "cases": [
                {"id": "DOWNLINK", "label": "ダウンリンク", "checks": [check("dod"), check("depleted")]},
                {"id": "SAFE", "checks": [check("dod")]},
            ],
        });
        let checks = collect_checks(Some(&output));
        let places: Vec<(&str, Option<&str>)> = checks
            .iter()
            .map(|c| (c.at.as_str(), c.scope.as_deref()))
            .collect();
        assert_eq!(
            places,
            [
                ("/cases/0/checks/0", Some("ダウンリンク")),
                ("/cases/0/checks/1", Some("ダウンリンク")),
                ("/cases/1/checks/0", Some("SAFE")),
                ("/checks/0", None),
            ]
        );
    }

    /// 期間不足は解析が文言を持たず、service が語彙の日本語と英語で注意にする（null なら何も足さない）。
    #[test]
    fn span_fallback_becomes_a_warning_in_both_languages() {
        let output = json!({"span_fallback": "orbit_mean_over_span", "checks": []});
        let checks = collect_checks(Some(&output));
        assert_eq!(checks.len(), 1);
        let c = &checks[0];
        assert_eq!((c.key.as_str(), c.at.as_str()), ("span", "/span_fallback"));
        assert_eq!(c.verdict, sat_analysis::spec::Verdict::Warning);
        assert!(c.message.contains("全期間の平均"));
        assert!(
            c.message_en
                .as_deref()
                .is_some_and(|m| m.contains("whole span"))
        );
        assert!(collect_checks(Some(&json!({"span_fallback": null}))).is_empty());
    }
}
