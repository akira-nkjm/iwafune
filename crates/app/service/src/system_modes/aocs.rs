//! モードの姿勢則を明示した解析を実行し、カード用の表示値にする。
use crate::dto::{
    AnalysisRunStatus, AocsAnalysisState, AocsModeView, AocsUnloadMeans, AocsUnloadTime,
    QuantityView,
};
use crate::error::Result;
use crate::workspace::Snapshot;
use fune_projection::{Context, MissingInputs, aocs};
use sat_analysis::aocs::{RwUnloading, SensorFov, UnloadTime, UnloadTimeOutput};
use sat_analysis::spec::{Analysis, Verdict};
use sat_units::typed::{AngularMomentum, Dimensionless, NamedKind, Time};

/// モードに依らない解析の結果（アンロード時間）。全モードで 1 回だけ求めて使い回す。
struct ModeIndependent {
    unload_times: Vec<AocsUnloadTime>,
    unloading: AocsAnalysisState,
}

pub(super) fn views(s: &Snapshot) -> Result<Vec<crate::dto::SystemModeView>> {
    let shared = mode_independent(s);
    s.design
        .ordered_modes()
        .into_iter()
        .map(|mode| {
            Ok(crate::dto::SystemModeView {
                id: mode.id.to_string(),
                sunlit: Default::default(),
                eclipse: Default::default(),
                aocs: Some(mode_view_with(s, mode.id.as_str(), &shared)?),
                power: None,
            })
        })
        .collect()
}

/// 1 モードのカード（アンロード時間も求める。複数モードを並べるときは `views` が 1 回だけ求める）。
#[cfg(test)]
pub(super) fn mode_view(s: &Snapshot, mode: &str) -> Result<AocsModeView> {
    mode_view_with(s, mode, &mode_independent(s))
}

/// 既定ケースの軌道条件を保つ文脈（姿勢則は各モードから選ぶので、ここでは選ばない）。
fn case_context(s: &Snapshot) -> std::result::Result<Context<'_>, MissingInputs> {
    let (case, _) =
        fune_projection::case_policy::effective_case(Some(&s.design), RwUnloading::ID, None);
    Context::new(&s.design, &s.resolved, case)
}

/// アンロード時間（`aocs.unload_time`）の入力はモードを取らないので、モードごとに回さない。
fn mode_independent(s: &Snapshot) -> ModeIndependent {
    let ctx = case_context(s);
    let (out, unloading) = evaluate::<UnloadTime>(
        ctx.as_ref()
            .map_err(Clone::clone)
            .and_then(aocs::unload_time),
    );
    ModeIndependent {
        unload_times: out.map(|o| unload_times(&o)).unwrap_or_default(),
        unloading,
    }
}

/// 解析が出した手段（MTQ・RCS）ごとのアンロード時間。RCS は最悪の向きで、捨てられない向きがあれば出さない。
pub(super) fn unload_times(o: &UnloadTimeOutput) -> Vec<AocsUnloadTime> {
    let time = |si: f64| crate::units::quantity_auto(si, Time::name());
    let mut out = Vec::new();
    if let Some(m) = &o.mtq {
        out.push(unload_time_of(AocsUnloadMeans::Mtq, time(m.time.value())));
    }
    if let Some(t) = o.rcs.as_ref().and_then(|r| r.worst.time) {
        out.push(unload_time_of(AocsUnloadMeans::Rcs, time(t.value())));
    }
    out
}

fn unload_time_of(means: AocsUnloadMeans, time: QuantityView) -> AocsUnloadTime {
    let l = crate::analysis::labels::unload_means(means);
    AocsUnloadTime {
        means,
        label: l.label,
        label_en: l.label_en,
        time,
    }
}

fn mode_view_with(s: &Snapshot, mode: &str, shared: &ModeIndependent) -> Result<AocsModeView> {
    let ctx = case_context(s);
    let empty = Context::bare(&s.design, &s.resolved);
    let context = ctx.as_ref().unwrap_or(&empty);
    let mode_def = context
        .modes()
        .into_iter()
        .find(|m| m.id == mode)
        .ok_or_else(|| crate::error::ServiceError::not_found("mode", mode))?;
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
        unload_times: shared.unload_times.clone(),
        rw,
        unloading: shared.unloading.clone(),
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
