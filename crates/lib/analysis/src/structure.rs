//! 構造系の解析。各解析は 1 ファイル 1 つ（入出力の型と実装）。系の中で共有する計算はサブモジュールに置き、台帳に登録する。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | [`mass_properties`] | `structure.mass_properties` と、質量特性の関数 [`mass_properties::mass_properties`]（projection も呼ぶ） |
//! | [`static_load`] | `structure.static_load` |
//! | [`modal_estimate`] | `structure.modal_estimate` |
//! | [`modal_fem`] | `structure.modal_fem`（パネルをシェル要素に分けた全体の固有値解析。メッシュ・要素・拘束・モードの分類は子モジュール） |
//! | [`acknowledged`] | 承知済みの組（`interference` と `cone_clearance` に共通。照合の規則と判定文の頭。所見は残して判定だけ `ok`） |
//! | [`interference`] | `structure.interference`（寸法を持つ部品の箱どうしの重なり。警告で、エラーにしない。向きつきの箱の距離・重なりの幾何は子の `obb`） |
//! | [`cone_clearance`] | `structure.cone_clearance`（噴流・視野の円錐に、機体の部品の箱が入っていないか。警告で、エラーにしない。円錐と箱の厳密な交差は子の `cone`。箱・取付の連鎖の規則は `interference` と共有） |
//! | [`external_geometry`] | `structure.external_geometry`（部品の箱から出した外形。投影面積は箱の輪郭の和集合で厳密、外に出ている表面積は面の被覆。報告だけ。幾何は子の `geometry`・`exposed`・`directions`。箱の形は `interference::obb` と共有） |
//! | [`launch_fit`] | `structure.launch_fit`（質量・収納外形・固有振動数が打ち上げ機・ディスペンサーの条件に収まるか。限度の候補から効く 1 つを選ぶ規則もここ） |

pub mod acknowledged;
pub mod cone_clearance;
pub mod external_geometry;
pub mod interference;
pub mod launch_fit;
pub mod mass_properties;
pub mod modal_estimate;
pub mod modal_fem;
pub mod static_load;

pub use cone_clearance::{ConeClearance, ConeClearanceInput, ConeClearanceOutput};
pub use external_geometry::{
    ExternalGeometryInput, ExternalGeometryOutput, ExternalGeometryReport,
};
pub use interference::{Interference, InterferenceInput, InterferenceOutput};
pub use launch_fit::{LaunchFit, LaunchFitInput, LaunchFitOutput};
pub use mass_properties::{
    MassProperties, MassPropertiesInput, MassPropertiesItem, MassPropertiesOutput,
    MassPropertiesSummary, mass_properties,
};
pub use modal_estimate::{ModalEstimate, ModalEstimateInput, ModalEstimateOutput};
pub use modal_fem::{ModalFem, ModalFemInput, ModalFemOutput};
pub use static_load::{StaticLoad, StaticLoadInput, StaticLoadOutput};
