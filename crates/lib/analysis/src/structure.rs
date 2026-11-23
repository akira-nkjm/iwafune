//! 構造系の解析。各解析は 1 ファイル 1 つ（入出力の型と実装）。系の中で共有する計算はサブモジュールに置き、台帳に登録する。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | [`mass_properties`] | `structure.mass_properties` と、質量特性の関数 [`mass_properties::mass_properties`]（projection も呼ぶ） |
//! | [`static_load`] | `structure.static_load` |
//! | [`modal_estimate`] | `structure.modal_estimate` |
//! | [`modal_fem`] | `structure.modal_fem`（パネルをシェル要素に分けた全体の固有値解析。メッシュ・要素・拘束・モードの分類は子モジュール） |
//! | [`interference`] | `structure.interference`（寸法を持つ部品の箱どうしの重なり。警告で、エラーにしない。向きつきの箱の距離・重なりの幾何は子の `obb`） |
//! | [`launch_fit`] | `structure.launch_fit`（質量・収納外形・固有振動数が打ち上げ機・ディスペンサーの条件に収まるか。限度の候補から効く 1 つを選ぶ規則もここ） |

pub mod interference;
pub mod launch_fit;
pub mod mass_properties;
pub mod modal_estimate;
pub mod modal_fem;
pub mod static_load;

pub use interference::{Interference, InterferenceInput, InterferenceOutput};
pub use launch_fit::{LaunchFit, LaunchFitInput, LaunchFitOutput};
pub use mass_properties::{
    MassProperties, MassPropertiesInput, MassPropertiesItem, MassPropertiesOutput,
    MassPropertiesSummary, mass_properties,
};
pub use modal_estimate::{ModalEstimate, ModalEstimateInput, ModalEstimateOutput};
pub use modal_fem::{ModalFem, ModalFemInput, ModalFemOutput};
pub use static_load::{StaticLoad, StaticLoadInput, StaticLoadOutput};
