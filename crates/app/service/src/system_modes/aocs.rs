//! モードの姿勢則を明示した解析を実行し、カード用の表示値にする。
use crate::dto::{AnalysisRunStatus, AocsAnalysisState, AocsModeView};
use crate::error::Result;
use crate::workspace::Snapshot;
use fune_projection::{Context, MissingInputs, aocs};
use sat_analysis::aocs::{RwUnloading, SensorFov, UnloadTime};
use sat_analysis::spec::{Analysis, Verdict};
use sat_units::typed::{AngularMomentum, Dimensionless, NamedKind, Time};

pub(super) fn views(s: &Snapshot) -> Result<Vec<crate::dto::SystemModeView>> {
    s.design
        .ordered_modes()
        .into_iter()
        .map(|mode| {
            Ok(crate::dto::SystemModeView {
                id: mode.id.to_string(),
                sunlit: Default::default(),
                eclipse: Default::default(),
                aocs: Some(mode_view(s, mode.id.as_str())?),
            })
        })
        .collect()
}

pub(super) fn mode_view(s: &Snapshot, mode: &str) -> Result<AocsModeView> {
    // 既定ケースの軌道条件を保ち、姿勢則だけ各モードから選ぶ。
    let (case, _) =
        fune_projection::case_policy::effective_case(Some(&s.design), RwUnloading::ID, None);
    let ctx = Context::new(&s.design, &s.resolved, case);
    let empty = Context::bare(&s.design, &s.resolved);
    let context = ctx.as_ref().unwrap_or(&empty);
    let mode_def = context
        .modes()
        .into_iter()
        .find(|m| m.id == mode)
        .ok_or_else(|| crate::error::ServiceError::not_found("モード", mode))?;
    let (rw_out, rw) = evaluate::<RwUnloading>(
        ctx.as_ref()
            .map_err(Clone::clone)
            .and_then(|c| aocs::rw_unloading_for_mode(c, Some(mode))),
    );
    let (_, sensor_fov) = evaluate::<SensorFov>(
        ctx.as_ref()
            .map_err(Clone::clone)
            .and_then(|c| aocs::sensor_fov_for_mode(c, Some(mode))),
    );
    let (unload_out, unloading) = evaluate::<UnloadTime>(
        ctx.as_ref()
            .map_err(Clone::clone)
            .and_then(|c| aocs::unload_time(c)),
    );
    Ok(AocsModeView {
        attitude: super::attitude::view(context, mode),
        devices: super::devices::groups(context, &mode_def)?,
        eclipse_devices: super::devices::groups(context, &mode_def.in_eclipse())?,
        momentum_per_orbit: rw_out.as_ref().map(|o| {
            crate::units::quantity_auto(o.secular_per_orbit.value(), AngularMomentum::name())
        }),
        unloads_per_day: rw_out
            .as_ref()
            .and_then(|o| o.unloads_per_day)
            .map(|n| crate::units::quantity_auto(n, Dimensionless::name())),
        mtq_unload_time: unload_out
            .and_then(|o| o.mtq)
            .map(|m| crate::units::quantity_auto(m.time.value(), Time::name())),
        rw,
        unloading,
        sensor_fov,
    })
}

fn evaluate<A: Analysis>(
    input: std::result::Result<A::Input, MissingInputs>,
) -> (Option<A::Output>, AocsAnalysisState) {
    let mut state = AocsAnalysisState {
        status: AnalysisRunStatus::Ok,
        verdict: None,
        missing: Vec::new(),
        error: None,
    };
    let input = match input {
        Ok(input) => input,
        Err(m) => {
            state.status = AnalysisRunStatus::MissingInputs;
            state.missing = m.into_vec();
            return (None, state);
        }
    };
    match sat_analysis::catalog::run_typed::<A>(&input) {
        Ok(out) => {
            let value = serde_json::to_value(&out).ok();
            let checks = crate::analysis::checks::collect_checks(value.as_ref());
            state.verdict = Verdict::worst(checks.iter().map(|c| c.verdict));
            (Some(out), state)
        }
        Err(e) => {
            state.status = AnalysisRunStatus::Failed;
            state.error = Some(e.to_string());
            (None, state)
        }
    }
}
