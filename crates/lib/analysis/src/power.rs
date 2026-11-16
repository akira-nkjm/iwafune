//! 電源系の解析。各解析は 1 ファイル 1 つ（入出力の型と実装）。系の中で共有する計算はサブモジュールに置き、台帳に登録する。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | [`solar`] | 太陽電池の発電モデルと時系列（日照時・軌道上・収支で共有） |
//! | [`battery`] | 電池の充放電の積分（収支が使う） |
//! | [`mode_load`] | `power.mode_load` モード別消費電力（収支もモードの負荷をここで求める） |
//! | [`solar_sunlit`] | `power.solar_sunlit` 日照時の発電 |
//! | [`orbit_generation`] | `power.orbit_generation` 軌道上の発電 |
//! | [`energy_balance`] | `power.energy_balance` 電力収支と DoD（連続・最長継続時間・モードの並び） |
//! | [`sizing`] | `power.sizing` SAP 面積・電池容量の逆算 |
//! | [`thrust_balance`] | `power.thrust_balance` 数値伝搬のマヌーバの噴射のモードで回す電力収支（電力収支の計算をそのまま使う） |

pub mod battery;
pub mod energy_balance;
pub mod mode_load;
pub mod orbit_generation;
pub mod sizing;
pub mod solar;
pub mod solar_sunlit;
#[cfg(test)]
pub(crate) mod testkit;
pub mod thrust_balance;

pub use energy_balance::{EnergyBalance, EnergyBalanceInput, EnergyBalanceOutput};
pub use mode_load::{ModeLoad, ModeLoadInput, ModeLoadOutput};
pub use orbit_generation::{OrbitGeneration, OrbitGenerationInput, OrbitGenerationOutput};
pub use sizing::{Sizing, SizingInput, SizingOutput};
pub use solar_sunlit::{SolarSunlit, SolarSunlitInput, SolarSunlitOutput};
pub use thrust_balance::{ThrustBalance, ThrustBalanceInput, ThrustBalanceOutput};
