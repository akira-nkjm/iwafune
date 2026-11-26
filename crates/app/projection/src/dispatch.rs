//! 解析 ID → 射影（入力の JSON）。**射影の対応表はここだけ。** 解析の登録と実行は `sat_analysis::catalog`（service が呼ぶ）。
//! `every_catalog_entry_has_a_row` のテストが、カタログとこの表がずれていないことを確かめる。

use crate::configuration_policy::ConfigurationSource;
use crate::context::Context;
use crate::missing::MissingInputs;
use crate::{aocs, cdh, comm, power, propulsion, structure, thermal};
use sat_analysis::spec::Analysis;
use sat_analysis::{
    aocs as aaocs, cdh as acdh, comm as acomm, power as apower, structure as astructure,
    thermal as athermal,
};
use serde_json::Value;
use ts_rs::{Config, TS, TypeVisitor};

/// 射影のエラー。
#[derive(Debug, thiserror::Error)]
pub enum ProjectionError {
    #[error("Unknown analysis: {0}")]
    UnknownAnalysis(String),
    #[error("{0}")]
    Missing(MissingInputs),
    #[error(
        "Building the input of {0} from design data is not implemented (it can run with a JSON input)"
    )]
    NotImplemented(String),
    #[error("Cannot convert the input to JSON: {0}")]
    Json(String),
}

impl From<MissingInputs> for ProjectionError {
    fn from(m: MissingInputs) -> Self {
        ProjectionError::Missing(m)
    }
}

/// 表の 1 行の説明。
#[derive(Debug, Clone)]
pub struct Entry {
    pub id: &'static str,
    /// 入力の TS 型名
    pub input_type: String,
    /// 出力の TS 型名
    pub output_type: String,
    /// 設計データから入力を組み立てられるか
    pub projectable: bool,
}

type Projector<A> = Option<fn(&Context<'_>) -> Result<<A as Analysis>::Input, MissingInputs>>;

fn entry<A: Analysis>(p: Projector<A>) -> Entry
where
    A::Input: TS,
    A::Output: TS,
{
    let cfg = Config::new();
    Entry {
        id: A::ID,
        input_type: <A::Input as TS>::name(&cfg),
        output_type: <A::Output as TS>::name(&cfg),
        projectable: p.is_some(),
    }
}

fn project_with<A: Analysis>(
    ctx: &Context<'_>,
    p: Projector<A>,
) -> Result<A::Input, ProjectionError> {
    let project = p.ok_or_else(|| ProjectionError::NotImplemented(A::ID.to_owned()))?;
    ctx.require_configuration()?;
    Ok(project(ctx)?)
}

fn to_json<A: Analysis>(ctx: &Context<'_>, p: Projector<A>) -> Result<Value, ProjectionError> {
    let input = project_with::<A>(ctx, p)?;
    serde_json::to_value(input).map_err(|e| ProjectionError::Json(e.to_string()))
}

/// 表の行を持つ解析（型のまま入力を組み立てられる）。`table!` がすべての解析に実装する。
pub trait Projects: Analysis {
    /// 設計データから型付きの入力を組み立てる（`project_json` と同じ射影）。
    fn project(ctx: &Context<'_>) -> Result<Self::Input, ProjectionError>;
}

/// 解析の型から入力を組み立てる（型のまま解析を回す呼び出し側用。表の行は `project_json` と同じ）。
pub fn project_typed<A: Projects>(ctx: &Context<'_>) -> Result<A::Input, ProjectionError> {
    A::project(ctx)
}

macro_rules! table {
    ($($a:ty => $p:expr),+ $(,)?) => {
        $(impl Projects for $a {
            fn project(ctx: &Context<'_>) -> Result<<$a as Analysis>::Input, ProjectionError> {
                project_with::<$a>(&ctx.for_analysis(Self::ID, None), $p)
            }
        })+

        /// 表のすべての行。
        pub fn entries() -> Vec<Entry> {
            vec![$(entry::<$a>($p)),+]
        }

        /// 解析の入力を設計データから組み立てて JSON で返す。
        pub fn project_json(id: &str, ctx: &Context<'_>) -> Result<Value, ProjectionError> {
            project_json_with_configuration(id, ctx, None).0
        }

        /// 形態の指定を受けて射影し、使う形態と選択元も返す。
        pub fn project_json_with_configuration<'a>(
            id: &str,
            ctx: &Context<'a>,
            explicit: Option<&'a str>,
        ) -> (Result<Value, ProjectionError>, Option<(&'a str, ConfigurationSource)>) {
            $(if id == <$a as Analysis>::ID {
                let ctx = ctx.for_analysis(id, explicit);
                return (to_json::<$a>(&ctx, $p), ctx.configuration());
            })+
            (Err(ProjectionError::UnknownAnalysis(id.to_owned())), None)
        }

        /// すべての解析の入出力の TS 型を書き出す（service の TS の書き出しから呼ぶ）。
        pub fn export_ts(cfg: &Config) -> Result<(), ts_rs::ExportError> {
            $(
                <<$a as Analysis>::Input as TS>::export_all(cfg)?;
                <<$a as Analysis>::Output as TS>::export_all(cfg)?;
            )+
            Ok(())
        }

        /// すべての解析の入出力の型をたどる（TS の型名の重複の検査用）。
        pub fn visit_ts<V: TypeVisitor>(v: &mut V) {
            $(
                v.visit::<<$a as Analysis>::Input>();
                v.visit::<<$a as Analysis>::Output>();
            )+
        }

        /// 出力の JSON を解析の出力の型に戻して `v` に渡す（service が量の次元を集めるのに使う）。
        pub fn visit_output<V: OutputVisitor>(id: &str, output: &Value, v: &mut V) -> Result<(), ProjectionError> {
            $(if id == <$a as Analysis>::ID {
                let typed: <$a as Analysis>::Output = serde_json::from_value(output.clone())
                    .map_err(|e| ProjectionError::Json(e.to_string()))?;
                v.visit(&typed);
                return Ok(());
            })+
            Err(ProjectionError::UnknownAnalysis(id.to_owned()))
        }
    };
}

/// 型の付いた解析の出力を受け取る（`visit_output`）。
pub trait OutputVisitor {
    fn visit<T: serde::Serialize>(&mut self, value: &T);
}

table!(
    apower::ModeLoad => Some(power::mode_load),
    apower::SolarSunlit => Some(power::solar_sunlit),
    apower::OrbitGeneration => Some(power::orbit_generation),
    apower::EnergyBalance => Some(power::energy_balance),
    apower::Sizing => Some(power::sizing),
    aaocs::RwUnloading => Some(aocs::rw_unloading),
    aaocs::UnloadTime => Some(aocs::unload_time),
    aaocs::SensorFov => Some(aocs::sensor_fov),
    aaocs::SlewTime => Some(aocs::slew_time),
    acdh::DataBudget => Some(cdh::data_budget),
    acomm::LinkBudget => Some(comm::link_budget),
    acomm::Visibility => Some(comm::visibility),
    acomm::DataThroughput => Some(comm::data_throughput),
    athermal::ThermalSteady => Some(thermal::steady),
    athermal::ThermalTransient => Some(thermal::transient),
    astructure::MassProperties => Some(structure::mass_properties),
    astructure::StaticLoad => Some(structure::static_load),
    astructure::ModalEstimate => Some(structure::modal_estimate),
    astructure::ModalFem => Some(structure::modal_fem),
    astructure::LaunchFit => Some(crate::launch_fit::launch_fit),
    astructure::Interference => Some(structure::interference),
    astructure::ConeClearance => Some(structure::cone_clearance),
    astructure::ExternalGeometryReport => Some(structure::external_geometry),
    sat_analysis::orbit::EclipseSeason => Some(crate::orbit::eclipse_season),
    sat_analysis::orbit::OrbitDecay => Some(crate::orbit::decay),
    sat_analysis::orbit::Geometry => Some(crate::orbit::geometry),
    sat_analysis::orbit::TotalDose => Some(crate::orbit::total_dose),
    sat_analysis::orbit::TrappedFlux => Some(crate::orbit::trapped_flux),
    sat_analysis::orbit::TrappedDose => Some(crate::orbit::trapped_dose),
    sat_analysis::propulsion::Maneuvers => Some(propulsion::maneuvers),
    sat_analysis::propulsion::DvBudget => Some(propulsion::dv_budget),
    sat_analysis::propulsion::Propellant => Some(propulsion::maneuvers),
    sat_analysis::propulsion::ThrusterUsage => Some(propulsion::maneuvers),
    sat_analysis::propulsion::OrbitHistoryAnalysis => Some(propulsion::maneuvers),
    sat_analysis::propulsion::TransferCheck => Some(propulsion::maneuvers),
    sat_analysis::propulsion::LaunchWindow => Some(propulsion::launch_window),
    sat_analysis::propulsion::PropellantMassProperties => Some(propulsion::mass_properties),
    apower::ThrustBalance => Some(power::thrust_balance),
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_entry_has_a_row() {
        let rows: Vec<&str> = entries().iter().map(|e| e.id).collect();
        for info in sat_analysis::catalog::catalog() {
            assert!(
                rows.contains(&info.id.as_str()),
                "{} has no projection row",
                info.id
            );
        }
        assert_eq!(rows.len(), sat_analysis::catalog::catalog().len());
    }
}
