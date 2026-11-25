//! 推進系の解析。マヌーバの列（実行の順）を軌道の遷移の式（`sat_astro::transfer`）とロケット方程式
//! （`sat_astro::rocket`）で順に計算し、4 つの解析が同じ入力 [`PropulsionInput`] から結果を出す。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | [`input`] | 共通の入力（軌道・乾燥質量・スラスタ・タンク・マヌーバ・マージン・アンロード）と列挙 |
//! | `plan` | マヌーバごとの前後の軌道と、方式（インパルス・有限燃焼・低推力）ごとの理想の ΔV（質量によらない部分） |
//! | `sequence` | 質量を減らしながらマヌーバを順に当てる（有限燃焼の損失・推薬・噴射時間）。初期質量は乾燥質量に合うまで繰り返す。数値伝搬のマヌーバは見積もりの列の質量で伝搬して置き換える |
//! | `flight` | 数値伝搬（`propagated`）のマヌーバを推力を入れて伝搬し、マヌーバの順に軌道の推移を作る |
//! | [`history`] | 軌道の推移（いつ・どの軌道にいるか）と時刻からの引き方 |
//! | [`dated`] | 日付つきの惑星間遷移（マヌーバの `transfer`）の結果（C3・超過速度・赤緯・飛行時間・暦の出所）と判定 |
//! | [`transfer_check`] | `propulsion.transfer_check` 日付つきの遷移を太陽と惑星の重力で数値伝搬し直した確認（到着のずれ・最接近・超過速度の差・軌跡）。伝搬は `sat_astro::transfer::check_cruise` |
//! | [`launch_window`] | `propulsion.launch_window` 天体間の遷移の出発の日 × 到着の日の格子（C3・超過速度・赤緯・ΔV・最小の点）。格子の点は `plan` の日付つきの経路で解く |
//! | [`maneuvers`] | `propulsion.maneuvers` マヌーバごとの ΔV・噴射時間・推薬・前後の軌道 |
//! | [`mass_properties`] | `propulsion.mass_properties` 推薬を積んだとき（寿命初期）と使ったあと（寿命末期）の質量と重心 |
//! | [`orbit_history`] | `propulsion.orbit_history` マヌーバの順の軌道の推移（時系列） |
//! | [`dv_budget`] | `propulsion.dv_budget` 区分ごとの ΔV の内訳・合計・マージン（RCS のアンロードの ΔV を含む） |
//! | [`unloading`] | RCS でのアンロードの推薬と ΔV（フェーズごとに `aocs.rw_unloading` を評価して期間を掛け、`dv_budget` の区分 `unloading` に足す） |
//! | [`propellant`] | `propulsion.propellant` 推薬の質量（ウェット・ドライ）とタンクの容積の余裕 |
//! | [`tank_load`] | `propulsion.propellant` のタンクごとの内訳（スラスタが名指したタンクと、名指さないスラスタの同じ推薬のタンクへの振り分け。RCS のアンロードの推薬を含む） |
//! | [`thruster_usage`] | `propulsion.thruster_usage` スラスタの総インパルス・累積噴射時間・噴射回数と定格の比較 |
//!
//! モデル化の既定値（ΔV のマージン・残る推薬の割合・余裕の警告の幅）はここ（[`input`]）が持つ。

pub mod dated;
pub mod dv_budget;
mod flight;
pub mod flyby;
pub mod history;
pub mod input;
pub mod launch_window;
pub mod maneuvers;
pub mod mass_properties;
pub mod orbit_history;
mod plan;
pub mod propellant;
mod sequence;
pub mod tank_load;
pub mod thruster_usage;
pub mod transfer_check;
pub mod unloading;

pub use dated::DatedTransferResult;
pub use dv_budget::{DvBudget, DvBudgetOutput};
pub use flyby::FlybyResult;
pub use history::{OrbitHistory, OrbitSegment};
pub use input::{
    DEFAULT_DV_MARGIN, DEFAULT_RESIDUAL_FRACTION, DatedTransferInput, DepartureProvider,
    DvCategory, FlybyInput, ManeuverInput, OrbitTarget, PropellantTank, PropulsionInput,
    PropulsionThruster, ThrustConstraints, TransferMethod, UnloadingInput, UnloadingLeg,
    UnloadingNote,
};
pub use launch_window::{
    LaunchWindow, LaunchWindowFlybySearch, LaunchWindowGrid, LaunchWindowInput, LaunchWindowNote,
    LaunchWindowOutput, LaunchWindowPoint, LaunchWindowPointKind, LaunchWindowQuantity,
    LaunchWindowResult, LaunchWindowSettings,
};
pub use maneuvers::{ManeuverNote, ManeuverPropagation, Maneuvers, ManeuversOutput};
pub use mass_properties::{PropellantMassInput, PropellantMassOutput, PropellantMassProperties};
pub use orbit_history::{OrbitHistoryAnalysis, OrbitHistoryOutput, orbit_history};
pub use propellant::{Propellant, PropellantOutput};
pub use tank_load::PropellantTankRow;
pub use thruster_usage::{ThrusterUsage, ThrusterUsageOutput};
pub use transfer_check::{
    TransferCheck, TransferCheckNote, TransferCheckOutput, TransferCheckResult,
};
pub use unloading::{DvUnloading, DvUnloadingLeg};

#[cfg(test)]
mod dated_tests;
#[cfg(test)]
mod propagated_tests;
#[cfg(test)]
mod swing_by_tests;
#[cfg(test)]
mod tank_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod transfer_check_tests;
#[cfg(test)]
mod unloading_tests;
#[cfg(test)]
mod written_tests;
