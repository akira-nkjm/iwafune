//! 実行の記憶（[`crate::session::Session`]）を使う解析の実行。1 つの依頼で解析をいくつも回す呼び出し側（要求適合表・設計書の書き出し・
//! 電源のモードの表示・概要の隠す表）が使う。同じ解析・同じケースは 1 回だけ計算し、結果（JSON の結果は `Rc`、型付きの結果は型のまま）を使い回す。
//! **計算は実効のケースごとに 1 回だけ**: 解析の実行の中身（入力の組み立てと解析の実行。[`Outcome`]）を `outcome` が記憶し、JSON の結果（[`run_cached`]。
//! 依頼のケースごとに判定・KPI を組み立てる）と型付きの結果（[`typed_run`]。`Outcome` の入力・出力を型に戻すだけ）のどちらから先に頼んでも同じ計算を使う。

use super::{Outcome, TypedRun, ensure_known, execute, run_inner};
use crate::dto::{AnalysisRunResult, RunAnalysisRequest};
use crate::error::{Result, ServiceError};
use crate::session::Session;
use fune_projection::Projects;
use sat_analysis::spec::Analysis;
use serde::de::DeserializeOwned;
use std::rc::Rc;

/// 設計から組み立てるだけの依頼か（形態・入力の上書き・重い出力のどれも無い）。そうでない依頼は記憶を通さない。
pub(super) fn is_plain(req: &RunAnalysisRequest) -> bool {
    req.configuration.is_none()
        && !req.detail
        && req.source == crate::dto::AnalysisInputSource::Design
}

/// 解析の実行の中身（入力の組み立てと解析の実行）を、実効のケース `case` ごとに 1 回だけ計算して持つ。失敗も記憶する。
pub(super) fn outcome(session: &Session, id: &str, case: Option<&str>) -> Result<Rc<Outcome>> {
    session.cached_result(&("outcome", id, case), || {
        run_inner(
            id,
            &RunAnalysisRequest::default(),
            Some(session.snapshot()),
            case,
        )
    })
}

/// 設計から組み立てた入力で解析を実行する（`case` を指定した場合も、省略して解析の既定に任せた場合も、依頼の通りに別々に記憶する。
/// 結果が選んだケースの出どころ（`case_source`）を持つため。ただし計算そのものは実効のケースごとに 1 回）。失敗も記憶する。
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

/// 解析を型付きで実行する。同じ解析・同じ**実効のケース**（`explicit_case` を省略して既定に任せたものと、
/// 既定のケースを明示したものは同じ）は 1 回だけ型に戻し、入力と出力を型のまま使い回す。計算は [`outcome`] を [`run_cached`] と共有する。
/// 不足入力・解析の失敗は出力を空にして返す（理由は [`run_cached`] の結果が持つ）。出力が型に戻せないときはエラー。
pub(crate) fn typed_run<A>(
    session: &Session,
    explicit_case: Option<&str>,
) -> Result<Rc<TypedRun<A>>>
where
    A: Analysis + Projects + 'static,
    A::Input: DeserializeOwned + 'static,
    A::Output: DeserializeOwned + 'static,
{
    let (case, ..) = super::effective_case(Some(session.snapshot()), A::ID, explicit_case);
    session.cached_result(&case, || {
        let outcome = outcome(session, A::ID, case.as_deref())?;
        Ok(TypedRun {
            input: back::<A::Input>(A::ID, "input", outcome.input.as_ref())?,
            output: back::<A::Output>(A::ID, "output", outcome.output.as_ref())?,
        })
    })
}

/// JSON の入力・出力を型に戻す（無ければ None。型に戻せなければ、空にせずエラー）。
fn back<T: DeserializeOwned>(
    id: &str,
    what: &str,
    value: Option<&serde_json::Value>,
) -> Result<Option<T>> {
    value
        .map(|v| {
            serde_json::from_value(v.clone()).map_err(|e| {
                ServiceError::Internal(format!("The {what} of {id} does not fit its type: {e}"))
            })
        })
        .transpose()
}
