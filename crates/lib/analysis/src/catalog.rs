//! 解析の一覧と ID による動的な実行。**解析の登録はここだけで行う。** 新しい解析を足したら下の一覧に 1 行足す（系の綴り順。軌道は `orbit`）。

use crate::spec::{Analysis, AnalysisError, AnalysisInfo};
use crate::{aocs, cdh, comm, power, propulsion, structure, thermal};

macro_rules! registry {
    ($($t:path),+ $(,)?) => {
        /// 登録されているすべての解析の説明。
        pub fn catalog() -> Vec<AnalysisInfo> {
            vec![$(<$t as Analysis>::info()),+]
        }

        /// 解析が計算の中で結果を使うほかの解析の ID（`Analysis::USES`）。未知の ID は空。
        pub fn uses(id: &str) -> &'static [&'static str] {
            $(
                if id == <$t as Analysis>::ID {
                    return <$t as Analysis>::USES;
                }
            )+
            &[]
        }

        /// ID で解析を選び、JSON の入力で実行して JSON の出力を返す（CLI と HTTP が使う）。
        pub fn run_json(id: &str, input: serde_json::Value) -> Result<serde_json::Value, AnalysisError> {
            $(
                if id == <$t as Analysis>::ID {
                    let input: <$t as Analysis>::Input =
                        serde_json::from_value(input).map_err(|e| AnalysisError::Json(e.to_string()))?;
                    let output = run_typed::<$t>(&input)?;
                    return serde_json::to_value(output).map_err(|e| AnalysisError::Json(e.to_string()));
                }
            )+
            Err(AnalysisError::Unknown(id.to_owned()))
        }
    };
}

registry!(
    aocs::RwUnloading,
    aocs::UnloadTime,
    aocs::SensorFov,
    aocs::SlewTime,
    cdh::DataBudget,
    comm::LinkBudget,
    comm::Visibility,
    comm::DataThroughput,
    crate::orbit::EclipseSeason,
    crate::orbit::OrbitDecay,
    power::ModeLoad,
    power::SolarSunlit,
    power::OrbitGeneration,
    power::EnergyBalance,
    power::Sizing,
    power::ThrustBalance,
    propulsion::Maneuvers,
    propulsion::DvBudget,
    propulsion::Propellant,
    propulsion::ThrusterUsage,
    propulsion::OrbitHistoryAnalysis,
    propulsion::TransferCheck,
    propulsion::PropellantMassProperties,
    structure::MassProperties,
    structure::StaticLoad,
    structure::ModalEstimate,
    structure::LaunchFit,
    thermal::ThermalSteady,
    thermal::ThermalTransient,
);

/// 型付き入力で登録済み解析を実行する。動的実行と budget が共有する入口。
pub fn run_typed<A: Analysis>(input: &A::Input) -> Result<A::Output, AnalysisError> {
    if !catalog().iter().any(|entry| entry.id == A::ID) {
        return Err(AnalysisError::Unknown(A::ID.to_owned()));
    }
    A::run(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 使う解析（`Analysis::USES`）はどれも登録済みで、自分自身を含まず、循環しない。
    #[test]
    fn uses_are_registered_and_acyclic() {
        let ids: Vec<String> = catalog().into_iter().map(|i| i.id).collect();
        fn reaches(from: &str, to: &str, depth: usize) -> bool {
            depth > 0
                && uses(from)
                    .iter()
                    .any(|u| *u == to || reaches(u, to, depth - 1))
        }
        for id in &ids {
            for u in uses(id) {
                assert!(
                    ids.iter().any(|x| x == u),
                    "{id} uses {u}, which is not registered"
                );
            }
            assert!(
                !reaches(id, id, ids.len()),
                "the analyses used by {id} form a cycle"
            );
        }
        assert_eq!(
            uses("power.sizing"),
            &[
                "orbit.eclipse_season",
                "power.mode_load",
                "power.orbit_generation",
                "power.energy_balance"
            ]
        );
    }
}
