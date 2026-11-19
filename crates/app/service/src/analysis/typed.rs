//! 解析の型付きの実行。結果（`AnalysisRunResult`）は [`super::run_analysis`] と同じ組み立て（`finish`）で、入力と出力を型のまま添える。
//! 表示の組み立て（電源のページ）が JSON に書いて読み直さずに型を直接読めるようにするため。

use super::{AnalysisRunStatus, Outcome, channels, dims, effective_case, finish, meta_labels};
use crate::dto::AnalysisRunResult;
use crate::error::{Result, ServiceError};
use fune_projection::{Context, ProjectionError};
use sat_analysis::spec::Analysis;

/// 型付きで回した解析: API に返す結果（JSON）と、入力・出力を型のまま。表示の組み立てはこの型を直接読む
/// （JSON に書いて読み直さない）。不足入力・失敗のときは `output` が無い。
pub(crate) struct TypedRun<A: Analysis> {
    pub result: AnalysisRunResult,
    pub input: Option<A::Input>,
    pub output: Option<A::Output>,
}

/// 解析を型付きで実行する（設計データから組み立てる入力だけ。結果は [`run_analysis`] と同じ）。
/// `snapshot` は呼び出し側が決める（下書きの `ops` を当てたものなど）。
pub(crate) fn run_typed_in<A>(
    snapshot: &crate::workspace::Snapshot,
    explicit_case: Option<&str>,
) -> Result<TypedRun<A>>
where
    A: Analysis + fune_projection::Projects,
{
    let info = effective_case(Some(snapshot), A::ID, explicit_case);
    let (outcome, input, output) = typed_outcome::<A>(snapshot, info.0.as_deref())?;
    Ok(TypedRun {
        result: finish(A::ID, info, outcome),
        input,
        output,
    })
}

/// 型付きの実行の途中の値（結果の素・型のままの入力・出力）。
type TypedOutcome<A> = (
    Outcome,
    Option<<A as Analysis>::Input>,
    Option<<A as Analysis>::Output>,
);

fn typed_outcome<A>(
    snapshot: &crate::workspace::Snapshot,
    case: Option<&str>,
) -> Result<TypedOutcome<A>>
where
    A: Analysis + fune_projection::Projects,
{
    let missing = |m: fune_projection::MissingInputs| Outcome {
        status: AnalysisRunStatus::MissingInputs,
        missing: m.into_vec(),
        error: None,
        ..Outcome::failed(String::new())
    };
    let ctx = match Context::new(&snapshot.design, &snapshot.resolved, case) {
        Ok(ctx) => ctx,
        Err(m) => return Ok((missing(m), None, None)),
    };
    let input = match fune_projection::project_typed::<A>(&ctx) {
        Ok(input) => input,
        Err(ProjectionError::Missing(m)) => return Ok((missing(m), None, None)),
        Err(e @ ProjectionError::NotImplemented(_)) => {
            return Ok((Outcome::failed(e.to_string()), None, None));
        }
        Err(e) => return Err(ServiceError::Analysis(e.to_string())),
    };
    let input_json = serde_json::to_value(&input)
        .map_err(|e| ServiceError::Internal(format!("Cannot write the input as JSON: {e}")))?;
    let case_mode = ctx.case_mode().map(|m| m.id.to_string());
    let output = match sat_analysis::catalog::run_typed::<A>(&input) {
        Ok(output) => output,
        Err(e) => {
            let failed = Outcome {
                input: Some(input_json),
                ..Outcome::failed(e.to_string())
            };
            return Ok((failed, Some(input), None));
        }
    };
    let mut json = serde_json::to_value(&output)
        .map_err(|e| ServiceError::Internal(format!("Cannot write the output as JSON: {e}")))?;
    channels::fix_channels(&mut json);
    let (dims, labels) = meta_labels(dims::meta_of(&output));
    let outcome = Outcome {
        status: AnalysisRunStatus::Ok,
        dims,
        labels,
        input: Some(input_json),
        output: Some(json),
        missing: Vec::new(),
        error: None,
        case_mode,
        space_weather: None,
    };
    Ok((outcome, Some(input), Some(output)))
}
