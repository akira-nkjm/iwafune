//! 地球の放射線帯（AP-8 の捕捉陽子・AE-8 の捕捉電子）の積分フラックス。**束縛粒子の地図の読み取りと引き方はここだけ。**
//!
//! 地図は NASA の NSSDC データセット PT-11B（Sawyer & Vette 1976 の AP-8、Vette 1991 の AE-8）で、(L, B/B₀, エネルギー) の
//! 積分全方向フラックスの表。**地図が作られた場と組にして使う**: B・L は地図ごとに決まった古い地磁気モデルの
//! 元期の値で（年変化は足さない）求める。形式は [`map`]、引き方は [`flux`]、地図と場の組は [`model`]。
//!
//! | サブモジュール | 責務 |
//! |---|---|
//! | [`map`] | 地図の ASCII ファイルの読み取り（[`TrappedMap`]） |
//! | `flux` | 積分フラックス `> E` の補間（[`TrappedMap::integral_flux`]） |
//! | [`model`] | 4 つの地図（[`TrappedModel`]）と、組にする地磁気モデル（[`BeltField`]）・L の求め方の定数 |
//! | [`position`] | 地磁気モデルから位置の (L, B/B₀) を求める |
//! | [`shielding`] | 遮蔽したアルミの深さでの線量への換算（SHIELDOSE-2。スペクトル → 陽子・電子・制動放射の線量） |
//! | [`registry`] | 読み込んだ地図・場のプロセス全体の登録（DE440 の登録と同じ。解析はここから引く） |
//!
//! 制限: 静的なモデル（太陽活動極小・極大の 2 つの元期だけ。年変化・磁気嵐・太陽粒子は含まない）。

mod flux;
pub mod map;
pub mod model;
pub mod position;
pub mod registry;
pub mod shielding;

pub use map::{MapHeader, TrappedMap};
pub use model::{BeltField, SolarCondition, Species, TrappedModel};
pub use position::{
    BeltConvention, BeltPosition, EQUATORIAL_FIELD_T, belt_position, belt_position_with,
};
pub use registry::RadiationTables;

#[cfg(test)]
mod tests;
