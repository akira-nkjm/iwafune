//! 縁の接触（edge contact）の規則。箱どうしの重なりのうち、**縁に沿った薄い切れ端だけ**が食い込んでいるものを、
//! 干渉ではなく接触として分ける（`structure.interference` が使う。規則の定義はここだけ）。
//!
//! 典型は、隣り合うパネルの箱: それぞれ取付先の面に中心を置くので、共有する縁で板厚の半分ずつ重なる。実物は縁で突き合わせるだけで、
//! これは箱の置き方からくる見かけの重なり。部品の種類（役割）は見ない。形だけで決める。
//!
//! **規則**（重なっていて、深さが許容差を超える組だけに使う）。箱 `P` の軸 `u_k` ごとに、重なりの区間
//! （`P` の範囲と相手の射影の共通部分）の長さを `e_k`、その区間が `P` の範囲の端に接するか（端から許容差以内）を見る。
//! 板厚 `t` は、2 つの箱の最も短い辺のうち小さいほう。区間が最も長い軸を「縁の方向」、残る 2 軸を「断面の方向」と呼ぶ。
//! 1. **断面が薄く、縁に沿って長い**: 両方の箱で、断面の 2 方向がどちらも「`e_k ≤ t`（許容差の余裕つき）で、範囲の端に接する」
//!    （切れ端が、箱の縁・面の端にある）、かつ縁の方向の長さが断面の長いほうの [`MIN_ASPECT`] 倍以上（細長い）。
//! 2. **体積が小さい**: 重なりの体積の上限（各箱で `Π e_k`。重なりは区間の箱に収まるので上限になる。小さいほうを取る）が、
//!    小さいほうの箱の体積の [`MAX_VOLUME_FRACTION`] 以下。
//!
//! 面に食い込む部品を縁の接触にしないための形: 板の面に箱が食い込むと、板の厚さ方向は薄くても、面内の 2 方向は部品の寸法ぶん長い
//! （断面が薄くない）。立方体どうしが面で少し食い込むのは、薄い方向が 1 つだけで、残る 2 方向は同じ長さ（細長くない）。
//! 部品が小さくて面内も薄くなる場合は、重なりが部品の体積の大部分になる（2. が満たされない）。
//! 板の面の中ほどに縁を当てて食い込む場合も、板の面内の方向が範囲の端に接しないので縁の接触にならない（保守的）。
//! 区間は箱の軸への射影の共通部分で、軸が合っていれば重なりそのもの。傾いていれば重なりを含む区間（上限）なので、
//! 判定は食い込みを縁の接触と取り違える側には傾かない。

use super::obb::OrientedBox;
use sat_units::typed::{Length, Qty};
use serde::{Deserialize, Serialize};

/// 重なりの体積の上限が、小さいほうの箱の体積のこの割合以下なら、薄い切れ端とみなす（板厚 1.5 mm・幅 226 mm のパネルが
/// 縁で半厚だけ重なる体積は 0.2 %。立方体どうしが縁で 14 % ずつ食い込む体積が 2 %）。
pub const MAX_VOLUME_FRACTION: f64 = 0.02;

/// 縁の方向の長さが、断面の長いほうの何倍以上なら「縁に沿って長い」とみなすか（パネルの角は板の長さ/半厚 = 数百倍。立方体の縁は 7 倍で、
/// 体積の条件が先に外れる）。
pub const MIN_ASPECT: f64 = 4.0;

/// 区間が箱の範囲の端に接しているとみなす距離の下限 [m]（許容差が 0 でも、座標の丸め誤差で外れないように）。
const MIN_CONTACT_EPS: f64 = 1e-9;

/// 縁の接触と判定した組（重なりの警告にはしない。出力に残す）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EdgeContact {
    /// 組の 2 つの部品（ID の昇順）
    pub first: String,
    pub second: String,
    /// 箱どうしの重なりの深さ（最も浅く分離できる量。干渉と同じ測り方）
    pub depth: Qty<Length>,
    /// 規則の板厚 `t`（2 つの箱の最も短い辺のうち小さいほう）
    pub thickness: Qty<Length>,
}

/// 縁の接触と判定した組の、判定に使った量。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeContactBasis {
    /// 板厚の上限 `t`（2 つの箱の最も短い辺のうち小さいほう）[m]
    pub thickness: f64,
    /// 重なりの体積の上限 [m³]
    pub volume_bound: f64,
}

/// 箱 `a`・`b` の重なりが縁の接触なら、その根拠。`tolerance` は干渉の許容差 [m]（端に接する・薄いの余裕）。
/// 呼ぶのは、重なりの深さが許容差を超えた組だけ（重なっていなければ意味を持たない）。
pub fn classify(a: &OrientedBox, b: &OrientedBox, tolerance: f64) -> Option<EdgeContactBasis> {
    let eps = tolerance.max(MIN_CONTACT_EPS);
    let thickness = shortest_side(a).min(shortest_side(b));
    let (sa, sb) = (overlap_along_axes(a, b), overlap_along_axes(b, a));
    let sliver = |s: &[Interval; 3]| {
        // 区間が最も長い軸が縁の方向、残る 2 軸が断面
        let long = (0..3).fold(0, |m, k| if s[k].length > s[m].length { k } else { m });
        let cross: Vec<&Interval> = (0..3).filter(|k| *k != long).map(|k| &s[k]).collect();
        let widest = cross.iter().map(|i| i.length).fold(0.0, f64::max);
        cross
            .iter()
            .all(|i| i.length <= thickness + eps && i.touches_end(eps))
            && s[long].length >= MIN_ASPECT * widest
    };
    if !sliver(&sa) || !sliver(&sb) {
        return None;
    }
    let volume = |s: &[Interval; 3]| s.iter().map(|i| i.length).product::<f64>();
    let volume_bound = volume(&sa).min(volume(&sb));
    let smaller = box_volume(a).min(box_volume(b));
    (volume_bound <= MAX_VOLUME_FRACTION * smaller).then_some(EdgeContactBasis {
        thickness,
        volume_bound,
    })
}

/// 箱の軸 1 本の上の、重なりの区間。
struct Interval {
    /// 区間の長さ [m]（正）
    length: f64,
    /// 箱の範囲の下端・上端から区間の端までの距離 [m]
    from_lower: f64,
    from_upper: f64,
}

impl Interval {
    /// 区間の端が、箱の範囲の端のどちらかに接するか。
    fn touches_end(&self, eps: f64) -> bool {
        self.from_lower <= eps || self.from_upper <= eps
    }
}

/// 箱 `p` の 3 つの軸それぞれの上で、`p` の範囲と相手 `q` の射影の共通部分。
fn overlap_along_axes(p: &OrientedBox, q: &OrientedBox) -> [Interval; 3] {
    let (up, uq) = (p.axes(), q.axes());
    let (hp, hq) = (p.half(), q.half());
    let delta = q.center - p.center;
    std::array::from_fn(|k| {
        // `q` の射影の中心と半径（`Σ_i h_i |u_k · uq_i|`）
        let centre = delta.dot(up[k]);
        let radius: f64 = (0..3).map(|i| hq[i] * up[k].dot(uq[i]).abs()).sum();
        let lo = (centre - radius).max(-hp[k]);
        let hi = (centre + radius).min(hp[k]);
        Interval {
            length: (hi - lo).max(0.0),
            from_lower: lo + hp[k],
            from_upper: hp[k] - hi,
        }
    })
}

/// 最も短い辺の長さ [m]。
fn shortest_side(b: &OrientedBox) -> f64 {
    2.0 * b.half().into_iter().fold(f64::INFINITY, f64::min)
}

/// 箱の体積 [m³]。
fn box_volume(b: &OrientedBox) -> f64 {
    8.0 * b.half().into_iter().product::<f64>()
}

#[cfg(test)]
mod tests;
