//! 実行の記憶（[`crate::session::Session`]）を使う解析の実行。1 つの依頼で解析をいくつも回す呼び出し側（要求適合表・設計書の書き出し・
//! 電源のモードの表示・概要の隠す表）が使う。同じ解析・同じケースは 1 回だけ計算し、結果（JSON の結果は `Rc`、型付きの結果は型のまま）を使い回す。
//! 実行の中身（入力の組み立て・判定・KPI）は [`super::run_analysis`]・[`super::run_typed_in`] と同じ。

use super::{TypedRun, ensure_known, execute, run_typed_in};
use crate::dto::{AnalysisRunResult, RunAnalysisRequest};
use crate::error::Result;
use crate::session::Session;
use fune_projection::Projects;
use sat_analysis::spec::Analysis;
use std::rc::Rc;

/// 設計から組み立てた入力で解析を実行する（`case` を指定した場合も、省略して解析の既定に任せた場合も、依頼の通りに別々に記憶する。
/// 結果が選んだケースの出どころ（`case_source`）を持つため）。同じ解析・同じ依頼のケースは 1 回だけ計算する。失敗も記憶する。
pub(crate) fn run_cached(
    session: &Session,
    id: &str,
    case: Option<&str>,
) -> Result<Rc<AnalysisRunResult>> {
    ensure_known(id)?;
    session.cached_result(&(id, case), || {
        let req = RunAnalysisRequest {
            case: case.map(str::to_owned),
            ..RunAnalysisRequest::default()
        };
        execute(id, &req, Some(session))
    })
}

/// 解析を型付きで実行する（[`super::run_typed_in`]）。同じ解析・同じ**実効のケース**（`explicit_case` を省略して既定に任せたものと、
/// 既定のケースを明示したものは同じ）は 1 回だけ計算し、入力と出力を型のまま使い回す。
pub(crate) fn typed_run<A>(
    session: &Session,
    explicit_case: Option<&str>,
) -> Result<Rc<TypedRun<A>>>
where
    A: Analysis + Projects + 'static,
    A::Input: 'static,
    A::Output: 'static,
{
    let (case, ..) = super::effective_case(Some(session.snapshot()), A::ID, explicit_case);
    session.cached_result(&case, || {
        run_typed_in::<A>(session.snapshot(), explicit_case)
    })
}
