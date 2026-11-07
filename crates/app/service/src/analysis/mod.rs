//! 解析の一覧と実行。解析の登録は `sat_analysis::catalog`、入力の組み立ては `fune_projection` が行う。

use crate::dto::{
    AnalysisEntry, AnalysisRunResult, AnalysisRunStatus, CheckView, RunAnalysisRequest,
};
use crate::error::{Result, ServiceError};
use crate::workspace::Workspace;
use fune_projection::{Context, MissingInput, ProjectionError, project_json};
use serde_json::Value;
use std::collections::BTreeMap;

mod channels;
mod dims;

/// 解析の一覧（`GET /analyses`）。
pub fn analysis_catalog() -> Result<Vec<AnalysisEntry>> {
    let rows = fune_projection::entries();
    sat_analysis::catalog::catalog()
        .into_iter()
        .map(|info| {
            let row = rows.iter().find(|r| r.id == info.id).ok_or_else(|| {
                ServiceError::Internal(format!("{} の射影の行がありません", info.id))
            })?;
            Ok(AnalysisEntry {
                input_type: row.input_type.clone(),
                output_type: row.output_type.clone(),
                projectable: row.projectable,
                info,
            })
        })
        .collect()
}

/// 実行の結果を組み立てる途中の値。
struct Outcome {
    status: AnalysisRunStatus,
    dims: BTreeMap<String, String>,
    input: Option<Value>,
    output: Option<Value>,
    missing: Vec<MissingInput>,
    error: Option<String>,
}

impl Outcome {
    fn failed(error: String) -> Self {
        Self {
            status: AnalysisRunStatus::Failed,
            dims: BTreeMap::new(),
            input: None,
            output: None,
            missing: Vec::new(),
            error: Some(error),
        }
    }
}

/// 解析を実行する（`POST /analyses/{id}`）。足りない入力は `missing` に入れて返す（エラーにしない）。
pub fn run_analysis(
    ws: &Workspace,
    id: &str,
    req: &RunAnalysisRequest,
) -> Result<AnalysisRunResult> {
    if !sat_analysis::catalog::catalog().iter().any(|i| i.id == id) {
        return Err(ServiceError::not_found("解析", id));
    }
    let outcome = run_inner(ws, id, req)?;
    let checks = collect_checks(outcome.output.as_ref());
    let verdict = sat_analysis::spec::Verdict::worst(checks.iter().map(|c| c.verdict));
    Ok(AnalysisRunResult {
        checks,
        verdict,
        id: id.to_owned(),
        case: req.case.clone(),
        status: outcome.status,
        input: outcome.input,
        output: outcome.output,
        missing: outcome.missing,
        error: outcome.error,
        dims: outcome.dims,
    })
}

fn run_inner(ws: &Workspace, id: &str, req: &RunAnalysisRequest) -> Result<Outcome> {
    let input = match &req.input {
        Some(v) => v.clone(),
        None => match project(ws, id, req.case.as_deref(), &req.ops) {
            Ok(v) => match &req.input_patch {
                Some(patch) => merge_patch(v, patch),
                None => v,
            },
            Err(ProjectionError::Missing(m)) => {
                return Ok(Outcome {
                    status: AnalysisRunStatus::MissingInputs,
                    dims: BTreeMap::new(),
                    input: None,
                    output: None,
                    missing: m.into_vec(),
                    error: None,
                });
            }
            Err(e @ ProjectionError::NotImplemented(_)) => {
                return Ok(Outcome::failed(e.to_string()));
            }
            Err(ProjectionError::Analysis(e)) => return Err(ServiceError::Analysis(e)),
            Err(e) => return Err(ServiceError::Analysis(e.to_string())),
        },
    };
    Ok(match sat_analysis::catalog::run_json(id, input.clone()) {
        Ok(mut output) => {
            channels::fix_channels(&mut output);
            Outcome {
                status: AnalysisRunStatus::Ok,
                dims: output_dims(id, &output),
                input: Some(input),
                output: Some(output),
                missing: Vec::new(),
                error: None,
            }
        }
        Err(e) => Outcome {
            input: Some(input),
            ..Outcome::failed(e.to_string())
        },
    })
}

/// 出力の中の `checks` を、出てきた順に集める（`series` の中は見ない）。
/// 場所（JSON Pointer）と、`checks` を持つオブジェクトの `label`（無ければ `id`）を添える。
fn collect_checks(output: Option<&Value>) -> Vec<CheckView> {
    fn walk(value: &Value, at: &str, out: &mut Vec<CheckView>) {
        match value {
            Value::Object(map) => {
                let scope = ["label", "id"]
                    .iter()
                    .find_map(|k| map.get(*k).and_then(Value::as_str))
                    .map(str::to_owned);
                for (key, child) in map {
                    let here = format!("{at}/{}", pointer_token(key));
                    if key == "checks" {
                        if let Ok(checks) =
                            serde_json::from_value::<Vec<sat_analysis::spec::Check>>(child.clone())
                        {
                            out.extend(checks.into_iter().enumerate().map(|(i, c)| CheckView {
                                key: c.key,
                                label: c.label,
                                verdict: c.verdict,
                                message: c.message,
                                at: format!("{here}/{i}"),
                                scope: if at.is_empty() { None } else { scope.clone() },
                            }));
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
    let mut checks = Vec::new();
    if let Some(output) = output {
        walk(output, "", &mut checks);
    }
    checks
}

/// JSON Pointer（RFC 6901）のキーの書き方（`~` → `~0`、`/` → `~1`）。
fn pointer_token(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// 設計データから入力を組み立てる。
/// 設計（`ops` があれば書き込まずに当てたもの）から解析の入力を組み立てる。
fn project(
    ws: &Workspace,
    id: &str,
    case: Option<&str>,
    ops: &[fune_ops::Op],
) -> Result<Value, ProjectionError> {
    let snapshot = crate::preview::snapshot_with(ws, ops)
        .map_err(|e| ProjectionError::Analysis(e.to_string()))?;
    let ctx = Context::new(&snapshot.design, &snapshot.resolved, case)?;
    project_json(id, &ctx)
}

/// JSON Merge Patch（RFC 7396）。
fn merge_patch(mut target: Value, patch: &Value) -> Value {
    let Value::Object(p) = patch else {
        return patch.clone();
    };
    if !target.is_object() {
        target = Value::Object(Default::default());
    }
    if let Value::Object(t) = &mut target {
        for (k, v) in p {
            if v.is_null() {
                t.remove(k);
            } else {
                let cur = t.remove(k).unwrap_or(Value::Null);
                t.insert(k.clone(), merge_patch(cur, v));
            }
        }
    }
    target
}

/// 出力の量の葉ごとの次元（JSON Pointer → 次元名）。型付きの出力に戻して `dims::dims_of` で集める。
fn output_dims(id: &str, output: &Value) -> BTreeMap<String, String> {
    struct Collect(BTreeMap<String, String>);
    impl fune_projection::OutputVisitor for Collect {
        fn visit<T: serde::Serialize>(&mut self, value: &T) {
            self.0 = dims::dims_of(value);
        }
    }
    let mut c = Collect(BTreeMap::new());
    match fune_projection::visit_output(id, output, &mut c) {
        Ok(()) => c.0,
        Err(_) => BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::merge_patch;
    use serde_json::json;

    #[test]
    fn collects_only_checks_and_skips_series() {
        let check =
            |key, verdict| json!({"key": key, "label": key, "verdict": verdict, "message": ""});
        let output = json!({"cases": [{"checks": [check("first", "warning")]}, {"checks": [check("second", "violation")]}], "series": {"checks": [check("ignored", "ok")]}, "other": [check("not_checks", "ok")]});
        let checks = super::collect_checks(Some(&output));
        assert_eq!(
            checks.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(),
            ["first", "second"]
        );
        assert_eq!(
            sat_analysis::spec::Verdict::worst(checks.iter().map(|c| c.verdict)),
            Some(sat_analysis::spec::Verdict::Violation)
        );
        assert!(super::collect_checks(None).is_empty());
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
        let checks = super::collect_checks(Some(&output));
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

    #[test]
    fn merge_patch_rfc7396() {
        let t = json!({"a": 1, "b": {"c": 2, "d": 3}});
        let p = json!({"b": {"c": 5, "d": null}, "e": true});
        assert_eq!(
            merge_patch(t, &p),
            json!({"a": 1, "b": {"c": 5}, "e": true})
        );
    }
}
