//! 解析ごとの既定ケースの対象宣言と解決。共通設定の既定ケースを読むのはここだけ。

use fune_model::Design;
use sat_analysis::aocs::rw_unloading::RwUnloading;
use sat_analysis::aocs::sensor_fov::SensorFov;
use sat_analysis::power::energy_balance::EnergyBalance;
use sat_analysis::power::orbit_generation::OrbitGeneration;
use sat_analysis::spec::Analysis;
use sat_analysis::thermal::transient::ThermalTransient;

/// ケースを選んだ理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseSource {
    /// 依頼で明示したケース。
    Explicit,
    /// 解析全体に適用する既定ケース。
    Default,
    /// 電力収支の上段の姿勢則だけに使う既定ケース。
    DefaultAttitude,
}

/// 既定ケースが必要な解析の宣言（姿勢則または熱条件を一つに選ぶ解析だけ）。
const DEFAULT_CASE_ANALYSES: &[&str] = &[
    OrbitGeneration::ID,
    RwUnloading::ID,
    SensorFov::ID,
    ThermalTransient::ID,
];

/// 明示 → 対象解析の既定 → なし。設計が無い given には既定を当てない。
/// `DefaultAttitude` の ID は姿勢則の参照専用で、解析全体のケースにしてはいけない。
pub fn effective_case<'a>(
    design: Option<&'a Design>,
    id: &str,
    explicit: Option<&'a str>,
) -> (Option<&'a str>, Option<CaseSource>) {
    if let Some(case) = explicit {
        return (Some(case), Some(CaseSource::Explicit));
    }
    let source = if DEFAULT_CASE_ANALYSES.contains(&id) {
        CaseSource::Default
    } else if id == EnergyBalance::ID {
        CaseSource::DefaultAttitude
    } else {
        return (None, None);
    };
    let case = design
        .and_then(|d| d.analysis.common.as_ref())
        .and_then(|c| c.default_case.as_deref());
    (case, case.map(|_| source))
}
