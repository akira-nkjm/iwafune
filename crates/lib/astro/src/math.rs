//! 3 次元ベクトル・3×3 行列・クォータニオン。座標系は呼び出し側が変数名で明示する（`r_eci` など）。
//!
//! | 型・関数 | ファイル |
//! |---|---|
//! | [`Vec3`] と演算子 | `math/vec3.rs` |
//! | [`Mat3`]（積・逆行列・外積行列・軸まわりの回転）と対称行列の固有値分解 [`Mat3::symmetric_eigen`] | `math/mat3.rs`・`math/eigen.rs` |
//! | [`Quat`] | `math/quat.rs` |
//! | 固定軸 X → Y → Z の回転角 ⇄ [`Quat`]（`Quat::{from_fixed_xyz, to_fixed_xyz}`） | `math/euler.rs` |
//! | 密行列の連立一次方程式 [`solve_dense`] | `math/dense.rs` |
//! | 対称行列のスカイライン格納と LDLᵀ 分解 [`Skyline`]・[`SkylineLdl`] | `math/skyline.rs` |
//! | 密対称行列の固有値分解・一般化固有値問題 [`symmetric_eigen`]・[`generalized_symmetric_eigen`] | `math/sym_eigen.rs` |
//! | 一般化固有値問題 `K x = λ M x` の低次モード（シフト付き部分空間反復と Sturm 列の判定）[`lowest_modes`] | `math/subspace.rs` |
//! | 平面の凸多角形と、その和集合の面積・図心 [`planar`]（帯の掃引で厳密。**凸多角形の和集合はここだけ**） | `math/planar.rs` |
//! | ガウス・ルジャンドル則の節点と重み [`quadrature::gauss_legendre`]（**求積則の節点はここだけ**） | `math/quadrature.rs` |
//! | 二分法 [`bisect`]・黄金分割 [`golden_section_max`] | `math/roots.rs` |

mod dense;
mod eigen;
mod euler;
mod mat3;
pub mod planar;
pub mod quadrature;
mod quat;
mod roots;
mod skyline;
mod subspace;
mod sym_eigen;
mod vec3;

pub use dense::solve_dense;
pub use mat3::Mat3;
pub use quat::Quat;
pub use roots::{bisect, golden_section_max, golden_section_min};
pub use skyline::{Skyline, SkylineError, SkylineLdl, ZERO_PIVOT_RELATIVE};
pub use subspace::{Mass, Modes, SubspaceError, SubspaceOptions, lowest_modes};
pub use sym_eigen::{SymEigen, SymEigenError, generalized_symmetric_eigen, symmetric_eigen};
pub use vec3::Vec3;
