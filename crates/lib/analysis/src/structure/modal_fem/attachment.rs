//! 集中質量の取り付け先の記録（パネルへの取り付け [`super::lumped`] とフレームの部材への取り付け [`super::frame_tie`] が共有する葉のモジュール）。

use super::input::ModalFemFace;

/// フットプリントの 1 辺あたりのサンプル点の数。
pub const FOOTPRINT_SAMPLES: usize = 5;

/// 集中質量の取り付け先。
#[derive(Debug, Clone)]
pub struct Attachment {
    pub id: String,
    /// 取り付けたパネルの ID（フレームの部材なら、フレームの部品の ID）
    pub panel: String,
    /// フレームの部材につないだときの辺
    pub edge: Option<[ModalFemFace; 2]>,
    /// 質量の位置からパネルの面（または辺）までの距離 [m]
    pub distance: f64,
    /// つなぎ先を自動で選んだか（`false` は名前で指定）
    pub automatic: bool,
    /// この部品の質量のうち、このつなぎ先に配った割合（名前で指定なら 1。最も近いパネルが複数なら等分）
    pub share: f64,
}
