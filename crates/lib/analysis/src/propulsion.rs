//! 推進系の解析。マヌーバの列（実行の順）を軌道の遷移の式（`sat_astro::transfer`）とロケット方程式
//! （`sat_astro::rocket`）で順に計算し、4 つの解析が同じ入力 [`PropulsionInput`] から結果を出す。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | [`input`] | 共通の入力（軌道・乾燥質量・スラスタ・タンク・マヌーバ・マージン・アンロード）と列挙 |
//! | `plan` | マヌーバごとの前後の軌道と、方式（インパルス・有限燃焼・低推力）ごとの理想の ΔV（質量によらない部分） |
//! | `sequence` | 質量を減らしながらマヌーバを順に当てる（有限燃焼の損失・推薬・噴射時間）。初期質量は乾燥質量に合うまで繰り返す |
//! | [`maneuvers`] | `propulsion.maneuvers` マヌーバごとの ΔV・噴射時間・推薬・前後の軌道 |
//! | [`dv_budget`] | `propulsion.dv_budget` 区分ごとの ΔV の内訳・合計・マージン（RCS のアンロードの ΔV を含む） |
//! | [`propellant`] | `propulsion.propellant` 推薬の質量（ウェット・ドライ）とタンクの容積の余裕 |
//! | [`thruster_usage`] | `propulsion.thruster_usage` スラスタの総インパルス・累積噴射時間・噴射回数と定格の比較 |
//!
//! モデル化の既定値（ΔV のマージン・残る推薬の割合・余裕の警告の幅）はここ（[`input`]）が持つ。

pub mod dv_budget;
pub mod input;
pub mod maneuvers;
mod plan;
pub mod propellant;
mod sequence;
pub mod thruster_usage;

pub use dv_budget::{DvBudget, DvBudgetOutput};
pub use input::{
    DEFAULT_DV_MARGIN, DEFAULT_RESIDUAL_FRACTION, DvCategory, ManeuverInput, OrbitTarget,
    PropellantTank, PropulsionInput, PropulsionThruster, TransferMethod, UnloadingInput,
    UnloadingNote,
};
pub use maneuvers::{ManeuverNote, Maneuvers, ManeuversOutput};
pub use propellant::{Propellant, PropellantOutput};
pub use thruster_usage::{ThrusterUsage, ThrusterUsageOutput};

#[cfg(test)]
mod tests;
