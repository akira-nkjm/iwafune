//! 解析の一覧と実行。解析の登録は `sat_analysis::catalog`、入力の組み立ては `fune_projection` が行う。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `dims` | 出力の量の次元と列挙の値（JSON Pointer ごと） |
//! | `channels` | 時系列の表示名と表示単位 |
//! | [`labels`] | 解析の出力の語彙の表示名（日本語・英語） |
//! | `checks` | 出力の中の判定を集める（期間不足の注意は語彙から作る） |
//! | `kpi` | KPI（代表値の選び方・判定・塗りの比・判定の数え方） |
//! | `overview` | 系の概要の解析の表の行（主な結果の選び方・判定の文の持ち主ごとのまとまり・電源の棒グラフと隠す表） |
//! | `part_findings` | 幾何の所見（箱の重なり・円錐に入る部品）に関わる部品ごとの状態（承知でない所見が残るか・すべて承知済みか） |
//! | `part_heat` | 熱の定常解析の部品ごとの代表の温度（節点と部品の対応・代表の選び方） |
//! | `typed` | 型付きの実行（入力・出力を型のまま返す。電源のモードの見方・概要の隠す表が使う） |
//! | `weather` | 高度低下の入力に宇宙天気の表から日付で引いた列を足し、使った表の版を返す |

use crate::dto::{
    AnalysisCaseSource, AnalysisEntry, AnalysisInputSource, AnalysisLabel, AnalysisRunResult,
    AnalysisRunStatus, RunAnalysisRequest,
};
use crate::error::{Result, ServiceError};
use crate::workspace::Workspace;
use fune_model::design::UncertaintySide;
use fune_projection::configuration_policy::ConfigurationSource;
use fune_projection::{Context, MissingInput, ProjectionError, project_json_with_configuration};
use serde_json::Value;
use std::collections::BTreeMap;

pub use failure::fails;
pub use fune_projection::system::area_key;
pub(crate) use typed::{TypedRun, run_typed_in};

pub(crate) mod channels;
pub(crate) mod checks;
mod dims;
mod failure;
pub(crate) mod kpi;
pub mod labels;
pub(crate) mod overview;
mod part_findings;
mod part_heat;
mod typed;
mod weather;

/// 解析の一覧（`GET /analyses`）。
pub fn analysis_catalog() -> Result<Vec<AnalysisEntry>> {
    let rows = fune_projection::entries();
    sat_analysis::catalog::catalog()
        .into_iter()
        .map(|info| {
            let row = rows.iter().find(|r| r.id == info.id).ok_or_else(|| {
                ServiceError::Internal(format!("No projection row for {}", info.id))
            })?;
            Ok(AnalysisEntry {
                input_type: row.input_type.clone(),
                output_type: row.output_type.clone(),
                projectable: row.projectable,
                kpis: kpi::defs(&info.id),
                detail_type: sat_analysis::catalog::detail_type(&info.id).map(str::to_owned),
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
    /// 解析ケースのモード（KPI の電力のモードの選び方に使う。入力を丸ごと与えたときは None）
    case_mode: Option<String>,
    configuration: Option<(String, ConfigurationSource)>,
    /// 入力に足した宇宙天気の表の版
    space_weather: Option<crate::dto::SpaceWeatherVersion>,
    /// 重い任意の出力（頼まれて、解析が持つときだけ）
    detail: Option<Value>,
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
            case_mode: None,
            configuration: None,
            space_weather: None,
            detail: None,
        }
    }
}

/// 設計から組み立てた入力と、そのケースのモード・入力に足した宇宙天気の表の版。
struct Projected {
    input: Value,
    case_mode: Option<String>,
    space_weather: Option<crate::dto::SpaceWeatherVersion>,
}

/// 解析を実行する（`POST /analyses/{id}`）。足りない入力は `missing` に入れて返す（エラーにしない）。
pub fn run_analysis(
    ws: &Workspace,
    id: &str,
    req: &RunAnalysisRequest,
) -> Result<AnalysisRunResult> {
    if !sat_analysis::catalog::catalog().iter().any(|i| i.id == id) {
        return Err(ServiceError::not_found("analysis", id));
    }
    let snapshot = match &req.source {
        AnalysisInputSource::Given { .. } => {
            if req.configuration.is_some() {
                return Err(ServiceError::InvalidInput(
                    "A configuration requires input built from the design".to_owned(),
                ));
            }
            None
        }
        _ => {
            let snapshot = crate::preview::snapshot_with(ws, &req.ops)?;
            crate::query::pose::require_configuration(
                &snapshot.design,
                req.configuration.as_deref(),
            )?;
            Some(snapshot)
        }
    };
    let (case, case_source, side) = effective_case(snapshot.as_ref(), id, req.case.as_deref());
    let outcome = run_inner(id, req, snapshot.as_ref(), case.as_deref())?;
    let mut result = finish(id, (case, case_source, side), outcome);
    overview::add_system_parts(&mut result, snapshot.as_ref(), req.case.as_deref());
    Ok(result)
}

/// 実行の途中の値から結果を組み立てる（判定・KPI・表示用の補足）。JSON の実行と型付きの実行が同じ組み立てを通る。
fn finish(id: &str, (case, case_source, side): CaseInfo, outcome: Outcome) -> AnalysisRunResult {
    let checks = checks::collect_checks(outcome.output.as_ref());
    let verdict = sat_analysis::spec::Verdict::worst(checks.iter().map(|c| c.verdict));
    let kpis = kpi::kpis(
        id,
        outcome.output.as_ref(),
        &checks,
        kpi::KpiContext {
            case_mode: outcome.case_mode.as_deref(),
        },
    );
    let (configuration, configuration_source) = outcome.configuration.unzip();
    let mut result = AnalysisRunResult {
        checks,
        verdict,
        kpis,
        id: id.to_owned(),
        case,
        case_source,
        configuration,
        configuration_source,
        side,
        status: outcome.status,
        input: outcome.input,
        output: outcome.output,
        missing: outcome.missing,
        error: outcome.error,
        dims: outcome.dims,
        labels: outcome.labels,
        space_weather: outcome.space_weather,
        detail: outcome.detail,
        overview: crate::dto::AnalysisOverview::default(),
        part_heat: None,
        part_findings: None,
    };
    result.part_heat = part_heat::part_heat(&result);
    result.part_findings = part_findings::part_findings(&result);
    result.overview = overview::overview(&result, None, Vec::new());
    result
}

/// projection のケース選択を DTO に写す。上段の姿勢則専用の ID は全体に渡さない。
fn effective_case(
    snapshot: Option<&crate::workspace::Snapshot>,
    id: &str,
    explicit: Option<&str>,
) -> CaseInfo {
    use fune_projection::case_policy::{CaseSource, effective_case};
    let (case, source) = effective_case(snapshot.map(|s| &s.design), id, explicit);
    let source = source.map(|s| match s {
        CaseSource::Explicit => AnalysisCaseSource::Explicit,
        CaseSource::Default => AnalysisCaseSource::Default,
        CaseSource::DefaultAttitude => AnalysisCaseSource::DefaultAttitude,
    });
    let case = if source == Some(AnalysisCaseSource::DefaultAttitude) {
        None
    } else {
        case.map(str::to_owned)
    };
    // 実際に読む側（ケースがあれば `side`、書かれていなければ nominal。ケースが無ければ欄を出さない）。
    let side = case.as_deref().and_then(|id| {
        let design = &snapshot?.design;
        let c = design.cases.get(id)?;
        Some(c.side.unwrap_or(UncertaintySide::Nominal))
    });
    (case, source, side)
}

/// 結果に載せるケースの情報（ケース ID・選択元・読んだ側）。
pub(crate) type CaseInfo = (
    Option<String>,
    Option<AnalysisCaseSource>,
    Option<UncertaintySide>,
);

fn run_inner(
    id: &str,
    req: &RunAnalysisRequest,
    snapshot: Option<&crate::workspace::Snapshot>,
    case: Option<&str>,
) -> Result<Outcome> {
    let (projected, configuration) = match &req.source {
        AnalysisInputSource::Given { input } => (
            Projected {
                input: input.clone(),
                case_mode: None,
                space_weather: None,
            },
            None,
        ),
        source => {
            let snapshot = snapshot
                .ok_or_else(|| ServiceError::Internal("No design for the analysis".to_owned()))?;
            let (projected, configuration) =
                project(snapshot, id, case, req.configuration.as_deref());
            let configuration = configuration.map(|(id, source)| (id.to_owned(), source));
            match projected {
                Ok(mut p) => {
                    if let AnalysisInputSource::Patched { patch } = source {
                        p.input = merge_patch(p.input, patch);
                    }
                    (p, configuration)
                }
                Err(ProjectionError::Missing(m)) => {
                    return Ok(Outcome {
                        status: AnalysisRunStatus::MissingInputs,
                        missing: crate::fixes::complete(
                            &snapshot.design,
                            &snapshot.resolved,
                            m.into_vec(),
                        ),
                        error: None,
                        configuration,
                        ..Outcome::failed(String::new())
                    });
                }
                Err(e @ ProjectionError::NotImplemented(_)) => {
                    return Ok(Outcome {
                        configuration,
                        ..Outcome::failed(e.to_string())
                    });
                }
                Err(e) => return Err(ServiceError::Analysis(e.to_string())),
            }
        }
    };
    Ok(evaluate(id, projected, configuration, req.detail))
}

/// 組み立てた入力で解析を実行し、出力の表示情報も作る。
fn evaluate(
    id: &str,
    Projected {
        input,
        case_mode,
        space_weather,
    }: Projected,
    configuration: Option<(String, ConfigurationSource)>,
    detail: bool,
) -> Outcome {
    match sat_analysis::catalog::run_json_detailed(id, input.clone(), detail) {
        Ok((mut output, detail)) => {
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
                case_mode,
                configuration,
                space_weather,
                detail,
            }
        }
        Err(e) => Outcome {
            input: Some(input),
            configuration,
            space_weather,
            ..Outcome::failed(e.to_string())
        },
    }
}

/// 設計（依頼の `ops` があれば書き込まずに当てたもの）から解析の入力を組み立てる。ケースのモードも添える。
/// 外部データの要る解析には、ここでキャッシュの表から引いた値を足す（`weather::attach`）。
fn project<'a>(
    snapshot: &'a crate::workspace::Snapshot,
    id: &str,
    case: Option<&str>,
    explicit: Option<&'a str>,
) -> (
    Result<Projected, ProjectionError>,
    Option<(&'a str, ConfigurationSource)>,
) {
    let ctx = match Context::new(&snapshot.design, &snapshot.resolved, case) {
        Ok(ctx) => ctx,
        Err(missing) => return (Err(missing.into()), None),
    };
    let (input, configuration) = project_json_with_configuration(id, &ctx, explicit);
    let projected = input.map(|input| {
        let (input, space_weather) = weather::attach(id, input);
        Projected {
            input,
            case_mode: ctx.case_mode().map(|m| m.id.to_string()),
            space_weather,
        }
    });
    (projected, configuration)
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
/// 型付きの出力に戻して `dims::meta_of` で集める（[`meta_labels`]）。
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
    meta_labels(c.0)
}

/// 集めた次元と列挙の値から、列挙の値の表示名を引く（`labels::value_label`）。
fn meta_labels(
    meta: dims::OutputMeta,
) -> (BTreeMap<String, String>, BTreeMap<String, AnalysisLabel>) {
    let labels = meta
        .variants
        .iter()
        .filter_map(|(at, (ty, value))| Some((at.clone(), labels::value_label(ty, value)?)))
        .collect();
    (meta.dims, labels)
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
