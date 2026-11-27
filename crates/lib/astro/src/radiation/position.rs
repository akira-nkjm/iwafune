//! 位置の (L, B/B₀)。AP-8・AE-8 のマップを引くための座標を、マップが前提にする場（[`super::BeltField`]）から求める。

use crate::geomag::{FieldSnapshot, LOptions, MomentConvention, NoL};
use crate::math::Vec3;

/// L を求める磁力線の長さの上限（基準半径の倍数）。AE-8 のマップは L = 12 までで、その先に L = 15.6 の見かけの節点がある。
const MAX_RADIUS_REF: f64 = 16.0;

/// AP-8・AE-8 が B₀ = M/L³ に使う双極子モーメント `M = 0.311653 G·Rₑ³` を、場の基準半径（6371.2 km）の球面上の赤道強度にしたもの [T]。
pub const EQUATORIAL_FIELD_T: f64 = 0.311_653e-4;

/// (L, B/B₀) の定義。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BeltConvention {
    /// 文献の定義（既定）: L はマップの元期の場から、**固定の双極子モーメント** `M = 0.311653 G·Rₑ³` で求め、
    /// B₀ = M/L³（同じ L の双極子の赤道強度）。SPENVIS などの AP-8・AE-8 の説明にある定義。
    #[default]
    Literature,
    /// 比較用: L は場自身の次数 1 の双極子モーメントで求め、B₀ は点を通る磁力線に沿う |B| の最小値
    /// （IGRF の `SHELLG` が返す赤道面の強さに当たる）。
    ModelDipoleLineMinimum,
}

/// マップを引く座標。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeltPosition {
    /// McIlwain の L（基準半径 6371.2 km が単位）
    pub l: f64,
    /// B/B₀（B₀ の取り方は [`BeltConvention`]）
    pub b_over_b0: f64,
    /// 点の |B| [T]
    pub b_t: f64,
    /// B₀ [T]
    pub b0_t: f64,
}

/// ECEF の位置 `r_ecef_m` [m] の (L, B/B₀)（既定の [`BeltConvention::Literature`]）。`field` はマップが前提にする場を固定した時刻の値にしたもの。
/// 磁力線が閉じない・L が求まらないときは理由を返す（マップの外）。
pub fn belt_position(field: &FieldSnapshot, r_ecef_m: Vec3) -> Result<BeltPosition, NoL> {
    belt_position_with(field, r_ecef_m, BeltConvention::default())
}

/// [`belt_position`] の定義を選ぶ版。
pub fn belt_position_with(
    field: &FieldSnapshot,
    r_ecef_m: Vec3,
    convention: BeltConvention,
) -> Result<BeltPosition, NoL> {
    let moment = match convention {
        BeltConvention::Literature => MomentConvention::Fixed {
            b0_t: EQUATORIAL_FIELD_T,
        },
        BeltConvention::ModelDipoleLineMinimum => MomentConvention::ModelDipole,
    };
    let s = field.l_shell(
        r_ecef_m,
        &LOptions {
            moment,
            max_radius_ref: MAX_RADIUS_REF,
        },
    )?;
    let b0_t = match convention {
        BeltConvention::Literature => EQUATORIAL_FIELD_T / s.l.powi(3),
        BeltConvention::ModelDipoleLineMinimum => s.b0_t,
    };
    Ok(BeltPosition {
        l: s.l,
        b_over_b0: s.b_t / b0_t,
        b_t: s.b_t,
        b0_t,
    })
}
