//! `structure.modal_fem` の出力の型。

use super::input::{ModalFemAxis, ModalFemFace, ModalFemSkipped};
use sat_units::typed::{Fraction, Frequency, Length, Mass, Qty};
use serde::{Deserialize, Serialize};

/// モードの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ModalFemModeClass {
    /// 剛体（拘束のないモード、機構）。振動数がほぼ 0
    Rigid,
    /// 機軸方向の全体のモード
    Axial,
    /// 機軸に直角な方向の全体のモード
    Lateral,
    /// 局所のモード（どの軸の有効質量も小さい。パネルの面外の振動など）
    Local,
}

/// 固有モード 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemMode {
    /// 1 から数える番号（振動数の昇順）
    pub index: u32,
    pub frequency: Qty<Frequency>,
    /// x, y, z 方向の有効質量の割合（全質量に対する比）
    pub effective_mass_fraction: [Fraction; 3],
    pub class: ModalFemModeClass,
    /// 最も効く軸（全体のモードだけ）
    pub dominant_axis: Option<ModalFemAxis>,
}

/// 共有する節点でつないだパネルの組。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemJoint {
    pub panel_a: String,
    pub panel_b: String,
    /// 共有する節点の数
    pub shared_nodes: u32,
}

/// 集中質量の取り付け先（1 つの部品が複数のパネルに等分されれば、パネルごとに 1 行。フレームの部材につなげば、辺ごとに 1 行）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemAttachment {
    pub id: String,
    /// 取り付けたパネル（フレームの部材につないだときはフレームの部品の ID）
    pub panel: String,
    /// フレームの部材につないだときの辺（箱の 2 つの面。`+x +y` のように [`edge_name`] の綴り）。パネルにつないだときは無い
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub edge: Option<String>,
    /// 質量の位置からパネルの面（または辺）までの距離
    pub distance: Qty<Length>,
    /// つなぎ先を自動で選んだ（最も近いパネル、またはフレームの最も近い辺。`false` は名前で指定）
    pub automatic: bool,
    /// この部品の質量のうち、このつなぎ先に配った割合（名前で指定なら 1。最も近いパネルが複数なら等分、フレームなら辺ごとの割合）
    pub share: Fraction,
}

/// 箱の辺の名前: 辺を作る 2 つの面の綴り（`+x +y`）。**辺の綴りはここだけ**。
pub fn edge_name(faces: [ModalFemFace; 2]) -> String {
    format!("{} {}", faces[0].as_str(), faces[1].as_str())
}

/// 部材をモデルにしたフレーム 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemFrameModel {
    pub id: String,
    /// 梁要素の数
    pub members: u32,
    /// 部材にした辺の数（箱の 12 本のうち、辺の上に節点が 2 つ以上あるもの）
    pub edges: u32,
    /// 部材にならなかった辺（辺の上に節点が 2 つ未満。辺に沿ったパネルが無い。`+x +y` のように [`edge_name`] の綴り）。質量は部材にした辺にだけ配る
    pub unmodelled_edges: Vec<String>,
    /// 部材に配った質量（フレームの全質量。部材が 1 本も無ければ 0 で、質量は全体に配る）
    pub mass: Qty<Mass>,
}

/// 機体の軸に沿わないパネルのつながり（辺ごと。節点が別のパネルの節点と一致して共有されていればつながっている。
/// 一致しない辺は、つながらない（剛につながない）ので、パネルが宙に浮いて見えるときはここを見る）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemSkewedPanel {
    pub id: String,
    /// 4 辺のうち、節点がすべて別のパネルと共有されている辺の数
    pub joined_edges: u32,
    /// 節点の一部だけが別のパネルと共有されている辺の数
    pub partly_joined_edges: u32,
    /// 節点が別のパネルと共有されていない辺の数（自由な辺、または格子が合わずつながっていない辺）
    pub unjoined_edges: u32,
}

/// モデルの大きさ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemModelSize {
    pub nodes: u32,
    pub elements: u32,
    /// 全自由度の数（節点 × 6）
    pub dofs: u32,
    /// 拘束した自由度の数
    pub fixed_dofs: u32,
    /// 実際に使った要素の大きさ
    pub mesh_size: Qty<Length>,
}

/// `structure.modal_fem` の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemOutput {
    /// 低いほうから（剛体のモードを含む）
    pub modes: Vec<ModalFemMode>,
    /// 剛体のモード（拘束で止まらない動き）の数
    pub rigid_body_modes: u32,
    /// 最も低い全体の横のモード（求めたモードの中に無ければ `None`）
    pub lowest_lateral: Option<Qty<Frequency>>,
    /// 最も低い全体の機軸のモード
    pub lowest_axial: Option<Qty<Frequency>>,
    pub model: ModalFemModelSize,
    /// 節点を共有してつないだパネルの組
    pub joints: Vec<ModalFemJoint>,
    pub attachments: Vec<ModalFemAttachment>,
    /// 部材をモデルにしたフレーム（無ければフレームの剛性は入っていない）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ModalFemFrameModel>>"))]
    pub frames: Vec<ModalFemFrameModel>,
    /// 機体の軸に沿わないパネルのつながり
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ModalFemSkewedPanel>>"))]
    pub skewed_panels: Vec<ModalFemSkewedPanel>,
    /// モデルにしなかったパネルと理由（入力の `skipped_panels`）
    pub skipped_panels: Vec<ModalFemSkipped>,
    /// 反復の回数（診断）
    pub iterations: u32,
    /// 最後のモードが重根の途中で切れている（求める数を 1 つ増やすと、同じ値のモードがもう 1 つ出る）
    pub cluster_split: bool,
    /// モデルの全質量（パネル + 集中質量 + 分布質量）
    pub total_mass: Qty<Mass>,
}
