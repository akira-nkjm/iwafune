//! 3 次元ベクトル・3×3 行列・クォータニオン。座標系は呼び出し側が変数名で明示する（`r_eci` など）。
//!
//! | 型・関数 | ファイル |
//! |---|---|
//! | [`Vec3`] と演算子 | `math/vec3.rs` |
//! | [`Mat3`]（積・逆行列・外積行列・軸まわりの回転）と対称行列の固有値分解 [`Mat3::symmetric_eigen`] | `math/mat3.rs`・`math/eigen.rs` |
//! | [`Quat`] | `math/quat.rs` |
//! | 固定軸 X → Y → Z の回転角 ⇄ [`Quat`]（`Quat::{from_fixed_xyz, to_fixed_xyz}`） | `math/euler.rs` |
//! | 密行列の連立一次方程式 [`solve_dense`] | `math/dense.rs` |
//! | 二分法 [`bisect`]・黄金分割 [`golden_section_max`] | `math/roots.rs` |

mod dense;
mod eigen;
mod euler;
mod mat3;
mod quat;
mod roots;
mod vec3;

pub use dense::solve_dense;
pub use mat3::Mat3;
pub use quat::Quat;
pub use roots::{bisect, golden_section_max};
pub use vec3::Vec3;
