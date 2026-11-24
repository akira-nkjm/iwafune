//! 出力の中の判定（`checks`）を、出てきた順に平らに集める。期間不足（出力の `span_fallback`）と、MTQ を中心天体の磁場で
//! 評価できなかった理由（`mtq_field_gap`）は、語彙から判定を作る。

use super::dims::pointer_token;
use super::labels::{mtq_field_check, orbit_model_check, span_check};
use crate::dto::CheckView;
use sat_analysis::aocs::unload_time::{MTQ_FIELD_MODEL_CHECK, MtqFieldGap};
use sat_analysis::spec::{Check, SpanFallback, Verdict};
use serde_json::Value;

/// 期間が 1 周に満たないときの代わりの集計を入れる欄（sat-analysis の出力）。
const SPAN_FALLBACK: &str = "span_fallback";

/// MTQ を中心天体の磁場で評価できなかった理由を入れる欄（`aocs.unload_time` の出力）。
const MTQ_FIELD_GAP: &str = "mtq_field_gap";

/// 解析の入力の中の、円制限三体問題の軌道（`source = "libration"`）を暦の 2 天体の位置に載せた軌道について、そのことを伝える説明（key `orbit_model`。
/// 判定は ok で、結果を悪くしない）。軌道は入力のどこにあってもよく（時系列の軌道・フェーズの軌道・通信の軌道）、見つかった最初の場所を `at` に
/// （`/input` からの JSON Pointer）入れて 1 つだけ作る。ほかの軌道の入力では空（出力は変わらない）。
pub fn orbit_model_checks(input: Option<&Value>) -> Vec<CheckView> {
    fn find(value: &Value, at: &str) -> Option<String> {
        match value {
            Value::Object(map) => {
                if map.get("source").and_then(Value::as_str)
                    == Some(sat_astro::orbit::LIBRATION_SOURCE)
                    && map.contains_key("point")
                {
                    return Some(at.to_owned());
                }
                map.iter()
                    .find_map(|(k, v)| find(v, &format!("{at}/{}", pointer_token(k))))
            }
            Value::Array(items) => items
                .iter()
                .enumerate()
                .find_map(|(i, v)| find(v, &format!("{at}/{i}"))),
            _ => None,
        }
    }
    let Some(at) = input.and_then(|v| find(v, "/input")) else {
        return Vec::new();
    };
    let (label, message) = orbit_model_check();
    vec![CheckView {
        key: "orbit_model".to_owned(),
        label: label.label,
        label_en: Some(label.label_en),
        verdict: Verdict::Ok,
        message: message.label,
        message_en: Some(message.label_en),
        at,
        scope: None,
    }]
}

/// 出力の中の `checks` を、出てきた順に集める（`series` の中は見ない）。
/// 場所（JSON Pointer）と、`checks` を持つオブジェクトの `label`（無ければ `id`）を添える。
/// `span_fallback` があれば、その場所に期間不足の注意（key `span`、文は語彙の日本語と英語）を足す。
/// `mtq_field_gap` があれば、その場所に判定（key [`MTQ_FIELD_MODEL_CHECK`]、判定は [`MtqFieldGap::verdict`]、文は語彙）を足す。
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
                } else if key == MTQ_FIELD_GAP {
                    if let Ok(Some(g)) =
                        serde_json::from_value::<Option<MtqFieldGap>>(child.clone())
                    {
                        out.push(mtq_field_view(g, here, scope.clone()));
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
        // 1 回限りの軌道は、期間が足りないのではなく周回が無いだけ（説明であって注意ではない）
        verdict: if f == SpanFallback::OneOffSpan {
            Verdict::Ok
        } else {
            Verdict::Warning
        },
        message: message.label,
        message_en: Some(message.label_en),
        at,
        scope,
    }
}

/// MTQ を中心天体の磁場で評価できなかったことの判定。
fn mtq_field_view(g: MtqFieldGap, at: String, scope: Option<String>) -> CheckView {
    let (label, message) = mtq_field_check(g);
    CheckView {
        key: MTQ_FIELD_MODEL_CHECK.to_owned(),
        label: label.label,
        label_en: Some(label.label_en),
        verdict: g.verdict(),
        message: message.label,
        message_en: Some(message.label_en),
        at,
        scope,
    }
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

    /// 1 回限りの軌道（周回が無い）の注意は、期間不足の警告ではなく説明（ok）で、日本語と英語の文がある。
    #[test]
    fn one_off_span_is_an_ok_note_in_both_languages() {
        let output = json!({"span_fallback": "one_off_span", "checks": []});
        let checks = collect_checks(Some(&output));
        assert_eq!(checks.len(), 1);
        let c = &checks[0];
        assert_eq!((c.key.as_str(), c.at.as_str()), ("span", "/span_fallback"));
        assert_eq!(c.verdict, sat_analysis::spec::Verdict::Ok);
        assert!(c.message.contains("1 回限り"));
        assert!(
            c.message_en
                .as_deref()
                .is_some_and(|m| m.contains("one-off arc"))
        );
    }

    /// MTQ を評価できなかった理由は、解析の判定に依らず語彙の日本語と英語で判定にする。判定は解析が決める
    /// （磁場が無ければ違反、モデルが無ければ注意、RCS がすべての向きを捨てられれば OK で、文に RCS で行うと添える）。
    #[test]
    fn mtq_field_gap_becomes_a_check_in_both_languages() {
        use sat_analysis::spec::Verdict;
        let gap = |reason: &str, covered: bool| {
            let output = json!({"mtq_field_gap": {"reason": reason, "covered_by_rcs": covered}, "checks": []});
            let checks = collect_checks(Some(&output));
            assert_eq!(checks.len(), 1);
            checks[0].clone()
        };
        let c = gap("absent", false);
        assert_eq!(
            (c.key.as_str(), c.at.as_str()),
            ("mtq_field_model", "/mtq_field_gap")
        );
        assert_eq!(c.verdict, Verdict::Violation);
        assert!(c.message.contains("磁場が無い"));
        assert!(!c.message.contains("RCS"));
        assert!(
            c.message_en
                .as_deref()
                .is_some_and(|m| m.contains("no global magnetic field"))
        );
        assert_eq!(gap("not_modelled", false).verdict, Verdict::Warning);
        let covered = gap("absent", true);
        assert_eq!(covered.verdict, Verdict::Ok);
        assert!(covered.message.contains("RCS で行います"));
        assert!(
            covered
                .message_en
                .as_deref()
                .is_some_and(|m| m.contains("done by RCS"))
        );
        assert!(collect_checks(Some(&json!({"mtq_field_gap": null}))).is_empty());
    }
}
