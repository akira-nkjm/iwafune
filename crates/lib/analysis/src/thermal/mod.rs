//! 熱系の解析。各解析は 1 ファイル 1 つ（入出力の型と実装）。系の中で共有する計算はサブモジュールに置き、台帳に登録する。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | [`network`] | 集中定数の熱ネットワークの型と解法（定常の Newton 法・非定常の微分）。定常・非定常で共有 |
//! | [`orbit_heat`] | 熱環境と外面への入熱（太陽・アルベド・地球赤外）。入射の幾何は `sat_astro::illumination` |
//! | [`range`] | 節点の温度と動作範囲の比較（警告の幅と判定）。定常・非定常で共有 |
//! | [`steady`] | `thermal.steady` |
//! | [`transient`] | `thermal.transient` |
//!
//! 解析（`steady`・`transient`）どうしは互いに参照しない。共有する計算は上の共有モジュールに置く。

pub mod network;
pub mod orbit_heat;
pub mod range;
pub mod steady;
pub mod transient;

pub use network::{
    ThermalConductiveLink, ThermalLoad, ThermalNetwork, ThermalNode, ThermalRadiativeLink,
    ThermalSurface, ThermalTempRange,
};
pub use orbit_heat::{ThermalEnvironment, ThermalEnvironmentUsed, ThermalExposure};
pub use steady::{ThermalSteady, ThermalSteadyInput, ThermalSteadyOutput};
pub use transient::{ThermalTransient, ThermalTransientInput, ThermalTransientOutput};
