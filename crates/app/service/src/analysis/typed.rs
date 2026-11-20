//! 解析の型付きの実行。入力と出力を JSON に書かずに型のまま返す（系ごとの表示の組み立て・概要の隠す表が型を直接読めるように）。
//! 入力の組み立てとケースの解決は [`super::run_analysis`] と同じ（`effective_case`・`project_typed`）。

use super::effective_case;
use crate::error::{Result, ServiceError};
use fune_projection::{Context, ProjectionError};
use sat_analysis::spec::Analysis;

/// 型付きで回した解析の入力と出力。不足入力・失敗のときは `output`（不足入力では `input` も）が無い。
pub(crate) struct TypedRun<A: Analysis> {
    pub input: Option<A::Input>,
    pub output: Option<A::Output>,
}

impl<A: Analysis> TypedRun<A> {
    fn none() -> Self {
        Self {
            input: None,
            output: None,
        }
    }
}

/// 解析を型付きで実行する（設計データから組み立てる入力だけ）。`snapshot` は呼び出し側が決める（下書きの `ops` を当てたものなど）。
/// 不足入力と解析の失敗は結果を空にして返す（理由は [`super::run_analysis`] の結果が持つ）。
pub(crate) fn run_typed_in<A>(
    snapshot: &crate::workspace::Snapshot,
    explicit_case: Option<&str>,
) -> Result<TypedRun<A>>
where
    A: Analysis + fune_projection::Projects,
{
    let (case, ..) = effective_case(Some(snapshot), A::ID, explicit_case);
    let Ok(ctx) = Context::new(&snapshot.design, &snapshot.resolved, case.as_deref()) else {
        return Ok(TypedRun::none());
    };
    let input = match fune_projection::project_typed::<A>(&ctx) {
        Ok(input) => input,
        Err(ProjectionError::Missing(_) | ProjectionError::NotImplemented(_)) => {
            return Ok(TypedRun::none());
        }
        Err(e) => return Err(ServiceError::Analysis(e.to_string())),
    };
    let output = sat_analysis::catalog::run_typed::<A>(&input).ok();
    Ok(TypedRun {
        input: Some(input),
        output,
    })
}
