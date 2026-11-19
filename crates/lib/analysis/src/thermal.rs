//! 熱系の解析。各解析は 1 ファイル 1 つ（入出力の型と実装）。系の中で共有する計算はサブモジュールに置き、台帳に登録する。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | [`network`] | 集中定数の熱ネットワークの型と解法（定常の Newton 法・非定常の微分）。定常・非定常で共有 |
//! | [`environment`] | 熱環境の値と、中心天体に合わせた決め方（書いた値 → プリセット → 天体の既定値） |
//! | [`planet`] | 中心天体ごとのアルベド・赤外放射の既定値（月の赤外は直下点の地方時で変わる） |
//! | [`orbit_heat`] | 外面への入熱（太陽・中心天体のアルベドと赤外）。入射の幾何は `sat_astro::illumination` |
//! | [`preset`] | 最悪ケース（高温・低温・セーフモード）のプリセットの熱環境の値 |
//! | [`range`] | 3 種の温度範囲（保管・動作・性能）と解析マージンの方針、節点の温度との比較と判定。定常・非定常で共有 |
//! | [`steady`] | `thermal.steady` |
//! | [`transient`] | `thermal.transient` |
//!
//! 解析（`steady`・`transient`）どうしは互いに参照しない。共有する計算は上の共有モジュールに置く。

pub mod environment;
pub mod network;
pub mod orbit_heat;
pub mod planet;
pub mod preset;
pub mod range;
pub mod steady;
pub mod transient;

pub use network::{
    ThermalCoating, ThermalConductiveLink, ThermalLoad, ThermalNetwork, ThermalNode,
    ThermalRadiativeLink, ThermalSurface,
};
pub use orbit_heat::{ThermalEnvironment, ThermalEnvironmentUsed, ThermalExposure};
pub use preset::ThermalPreset;
pub use range::{
    ThermalMarginPolicy, ThermalMarginUsed, ThermalNodeRange, ThermalPowerState,
    ThermalRangeExclusion, ThermalRangeKind, ThermalRangeMargin, ThermalRangeSide, ThermalRanges,
    ThermalTempRange, range_item,
};
pub use steady::{ThermalSteady, ThermalSteadyInput, ThermalSteadyOutput};
pub use transient::{ThermalTransient, ThermalTransientInput, ThermalTransientOutput};
