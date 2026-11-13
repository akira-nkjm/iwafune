//! 解析の一覧と ID による動的な実行。**解析の登録はここだけで行う。** 新しい解析を足したら下の一覧に 1 行足す。

use crate::spec::{Analysis, AnalysisError, AnalysisInfo};
use crate::{aocs, cdh, comm, power, structure, thermal};

macro_rules! registry {
    ($($t:path),+ $(,)?) => {
        /// 登録されているすべての解析の説明。
        pub fn catalog() -> Vec<AnalysisInfo> {
            vec![$(<$t as Analysis>::info()),+]
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
    power::ModeLoad,
    power::SolarSunlit,
    power::OrbitGeneration,
    power::EnergyBalance,
    aocs::RwUnloading,
    aocs::UnloadTime,
    aocs::SensorFov,
    cdh::DataBudget,
    comm::LinkBudget,
    comm::Visibility,
    comm::DataThroughput,
    thermal::ThermalSteady,
    thermal::ThermalTransient,
    structure::MassProperties,
    structure::StaticLoad,
    structure::ModalEstimate,
    crate::orbit::EclipseSeason,
);

/// 型付き入力で登録済み解析を実行する。動的実行と budget が共有する入口。
pub fn run_typed<A: Analysis>(input: &A::Input) -> Result<A::Output, AnalysisError> {
    if !catalog().iter().any(|entry| entry.id == A::ID) {
        return Err(AnalysisError::Unknown(A::ID.to_owned()));
    }
    A::run(input)
}
