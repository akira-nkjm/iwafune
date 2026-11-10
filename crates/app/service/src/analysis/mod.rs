//! 解析の一覧と実行。解析の登録は `sat_analysis::catalog`、入力の組み立ては `fune_projection` が行う。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `dims` | 出力の量の次元と列挙の値（JSON Pointer ごと） |
//! | `channels` | 時系列の表示名と表示単位 |
//! | [`labels`] | 解析の出力の語彙の表示名（日本語・英語） |
//! | `checks` | 出力の中の判定を集める（期間不足の注意は語彙から作る） |

use crate::dto::{
    AnalysisEntry, AnalysisInputSource, AnalysisLabel, AnalysisRunResult, AnalysisRunStatus,
    RunAnalysisRequest,
};
use crate::error::{Result, ServiceError};
use crate::workspace::Workspace;
use fune_projection::{Context, MissingInput, ProjectionError, project_json};
use serde_json::Value;
use std::collections::BTreeMap;

mod channels;
mod checks;
mod dims;
pub mod labels;

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
    labels: BTreeMap<String, AnalysisLabel>,
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
            labels: BTreeMap::new(),
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
    let checks = checks::collect_checks(outcome.output.as_ref());
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
        labels: outcome.labels,
    })
}

fn run_inner(ws: &Workspace, id: &str, req: &RunAnalysisRequest) -> Result<Outcome> {
    let input = match &req.source {
        AnalysisInputSource::Given { input } => input.clone(),
        source => {
            // 下書き（`ops`）が通らなければ、保存・バジェットと同じエラー（検証なら診断付き）を返す
            let snapshot = crate::preview::snapshot_with(ws, &req.ops)?;
            match project(&snapshot, id, req.case.as_deref()) {
                Ok(v) => match source {
                    AnalysisInputSource::Patched { patch } => merge_patch(v, patch),
                    _ => v,
                },
                Err(ProjectionError::Missing(m)) => {
                    return Ok(Outcome {
                        status: AnalysisRunStatus::MissingInputs,
                        missing: m.into_vec(),
                        error: None,
                        ..Outcome::failed(String::new())
                    });
                }
                Err(e @ ProjectionError::NotImplemented(_)) => {
                    return Ok(Outcome::failed(e.to_string()));
                }
                Err(e) => return Err(ServiceError::Analysis(e.to_string())),
            }
        }
    };
    Ok(match sat_analysis::catalog::run_json(id, input.clone()) {
        Ok(mut output) => {
            channels::fix_channels(&mut output);
            let (dims, labels) = output_meta(id, &output);
            Outcome {
                status: AnalysisRunStatus::Ok,
                dims,
                labels,
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

/// 設計（依頼の `ops` があれば書き込まずに当てたもの）から解析の入力を組み立てる。
fn project(
    snapshot: &crate::workspace::Snapshot,
    id: &str,
    case: Option<&str>,
) -> Result<Value, ProjectionError> {
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

/// 出力の量の葉ごとの次元（JSON Pointer → 次元名）と、語彙の値の表示名（JSON Pointer → 表示名）。
/// 型付きの出力に戻して `dims::meta_of` で集め、列挙の値は `labels::value_label` で引く。
fn output_meta(
    id: &str,
    output: &Value,
) -> (BTreeMap<String, String>, BTreeMap<String, AnalysisLabel>) {
    struct Collect(dims::OutputMeta);
    impl fune_projection::OutputVisitor for Collect {
        fn visit<T: serde::Serialize>(&mut self, value: &T) {
            self.0 = dims::meta_of(value);
        }
    }
    let mut c = Collect(dims::OutputMeta::default());
    if fune_projection::visit_output(id, output, &mut c).is_err() {
        return (BTreeMap::new(), BTreeMap::new());
    }
    let labels =
        c.0.variants
            .iter()
            .filter_map(|(at, (ty, value))| Some((at.clone(), labels::value_label(ty, value)?)))
            .collect();
    (c.0.dims, labels)
}

#[cfg(test)]
mod tests {
    use super::merge_patch;
    use serde_json::json;

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
