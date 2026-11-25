//! 取付の伝導を、両端の部品の箱の接触面積から出す（`contact_conductance_per_area` を書いたときだけ）。
//!
//! 取付 1 か所の伝導は `h × A`（`h` は単位面積あたりの接触熱コンダクタンス、`A` は 2 つの箱の接する面の面積）。
//! **箱どうしの接触面の面積の計算は幾何で、`OrientedBox::contact_area` にある（ここは使うだけ）。**
//! 接触面を決められない取付（箱を持たない部品・箱が接していない・面積が 0）は、従来どおりの 1 取付あたりの値
//! （`contact_conductance`）に戻し、その取付を出力の `fallbacks` に理由つきで残す（誤りにはしない）。
//! 許容差は `structure.interference` の既定（`DEFAULT_TOLERANCE`）と同じ（箱が「接している」の判断を 2 か所で変えない）。

use crate::structure::interference::obb::OrientedBox;
use crate::structure::interference::{DEFAULT_TOLERANCE, InterferenceBox, is_usable};
use sat_units::typed::{Area, HeatTransferCoefficient, Length, Qty};
use serde::{Deserialize, Serialize};

/// 取付の伝導を接触面積から出すための入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalContact {
    /// 単位面積あたりの接触熱コンダクタンス `h`
    pub conductance_per_area: Qty<HeatTransferCoefficient>,
    /// 取付ごとの両端の部品と箱（`ThermalMounts::mounts` と同じ順・同じ数）
    pub mounts: Vec<ThermalMountBoxes>,
}

/// 取付 1 か所の両端の部品の ID と箱（寸法を持たない部品の箱は `None`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalMountBoxes {
    /// 取り付ける部品と取付先の部品（`ThermalMounts::mounts` の節点と同じ向き）
    pub parts: [String; 2],
    pub boxes: [Option<InterferenceBox>; 2],
}

/// 接触面積から出せなかった理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ThermalContactFallbackReason {
    /// 片方の部品が寸法（箱）を持たない（正の有限値でない寸法を含む）
    NoBox,
    /// 箱が接していない（隙間が許容差より大きい・面が向き合わない・接触面の面積が 0）
    NotTouching,
}

/// 1 取付あたりの値に戻した取付。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalContactFallback {
    /// 取り付ける部品
    pub part: String,
    /// 取付先の部品
    pub mounted_on: String,
    pub reason: ThermalContactFallbackReason,
}

/// 接触面積から出した結合の使われ方（`contact_conductance_per_area` を書いたときだけ出力に入る）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalContactUsed {
    /// 使った `h`
    pub conductance_per_area: Qty<HeatTransferCoefficient>,
    /// 箱が接しているとみなした距離の許容差
    pub contact_tolerance: Qty<Length>,
    /// 接触面積から出した取付の数（節点内の取付を除く）
    pub geometric_mounts: u32,
    /// 1 取付あたりの値に戻した取付（取付する部品の ID の順）
    pub fallbacks: Vec<ThermalContactFallback>,
}

/// 節点対の伝導のうち、接触面積から出した分の内訳（出力の `ThermalConductanceUsed::contact`）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalContactPair {
    /// 接触面積から出した取付の接触面積の合計
    pub area: Qty<Area>,
    /// 接触面積から出した取付の数
    pub contact_mounts: u32,
    /// 同じ節点対で 1 取付あたりの値に戻した取付の数（混ざった対だけ 0 でない）
    pub fixed_mounts: u32,
}

/// 取付 1 か所の接触面積 [m²]。出せなければ理由。
pub(super) fn contact_area(mount: &ThermalMountBoxes) -> Result<f64, ThermalContactFallbackReason> {
    let [Some(a), Some(b)] = &mount.boxes else {
        return Err(ThermalContactFallbackReason::NoBox);
    };
    let (Some(a), Some(b)) = (oriented(a), oriented(b)) else {
        return Err(ThermalContactFallbackReason::NoBox);
    };
    a.contact_area(&b, DEFAULT_TOLERANCE.value())
        .ok_or(ThermalContactFallbackReason::NotTouching)
}

/// 使える箱（`is_usable`）だけを幾何の箱にする。
pub(crate) fn oriented(b: &InterferenceBox) -> Option<OrientedBox> {
    is_usable(b).then(|| OrientedBox::from_size(b.position, b.orientation, b.size))
}
