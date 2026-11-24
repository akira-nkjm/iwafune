//! 向きつきの箱（OBB）どうしの距離と重なり。分離軸の判定（SAT）で、2 つの箱の **符号つきの分離量** を求める。
//!
//! 凸多面体どうしは、面の法線と辺どうしの外積の軸のどれかで射影が離れていれば離れている。箱 2 つなら軸は 15 本
//! （各箱の面の法線 3 + 3、辺の方向どうしの外積 3 × 3）。軸 `n` に射影した 2 つの箱の区間の隙間は
//! `|n·(c_b − c_a)| − (r_a + r_b)`、`r = Σ_i h_i |n·u_i|`（`h_i` は半幅、`u_i` は箱の軸）で、正なら離れている。
//! **全部の軸のうち隙間が最大のもの**が符号つきの分離量で、負なら重なりの深さ（その軸方向に動かせば最小の移動で離れる）。
//!
//! - 重なっているとき（分離量 < 0）は、15 本の最大が最小の侵入（最小の並進で離せる向きと量）に厳密に一致する（凸多面体の性質）。
//! - 離れているとき（分離量 > 0）は、最良の軸での隙間で、真の最短距離の **下限**（面どうしが向き合うなら一致し、
//!   角・辺どうしが斜めに向き合うときは真の距離より小さい）。隙間の「目安」であって、最短距離そのものではない。
//! - 箱の形（頂点・面・投影の輪郭・断面・包含・外接）は子の `shape`（`structure.external_geometry` が使う）。線分が箱を通るかは子の `segment`。
//! - 計算量: 1 組あたり定数（軸 15 本）。部品 n 個の全組は O(n²)。n が数百までなら数万組で済むので、広域の絞り込みはしない。

mod segment;
mod shape;

use sat_astro::math::{Quat, Vec3};
pub use segment::SegmentBox;
pub use shape::{BoxFace, PLANE_EPS, axis_aligned_extents, plane_basis};

/// 辺の外積の軸が縮退（2 辺がほぼ平行）とみなす外積の大きさ。平行な辺は面の軸が受け持つので、ここでは飛ばす。
const PARALLEL_EDGE_EPS: f64 = 1e-9;

/// 向きつきの箱（中心・姿勢・半幅）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientedBox {
    /// 中心 [m]（機体座標系）
    pub center: Vec3,
    /// 箱 → 機体座標系の回転
    pub orientation: Quat,
    /// 箱の座標系の各軸の半幅 [m]
    pub half_extent: Vec3,
}

impl OrientedBox {
    /// 辺の長さ `size` [m] から作る（中心は部品の原点。`size` は箱の座標系の辺）。
    pub fn from_size(center: Vec3, orientation: Quat, size: Vec3) -> Self {
        Self {
            center,
            orientation: orientation.normalized(),
            half_extent: size.scale(0.5),
        }
    }

    /// 箱の 3 軸（機体座標系の単位ベクトル）。
    pub fn axes(&self) -> [Vec3; 3] {
        [Vec3::X, Vec3::Y, Vec3::Z].map(|e| self.orientation.rotate(e))
    }

    /// 箱の座標系の各軸の半幅 [m]（`half_extent` の配列）。
    pub fn half(&self) -> [f64; 3] {
        [self.half_extent.x, self.half_extent.y, self.half_extent.z]
    }
}

/// 2 つの箱の符号つきの分離量と、その軸。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Separation {
    /// 負なら重なり（深さ = `-distance`）、0 なら接している、正なら離れている（最良の軸での隙間）[m]
    pub distance: f64,
    /// 機体座標系の単位ベクトル。`a` から `b` へ向く（`b` をこの向きに `|distance|` だけ動かせば、重なりは解ける）
    pub axis: Vec3,
}

impl Separation {
    /// 重なりの深さ [m]（重なっていなければ 0）。
    pub fn depth(&self) -> f64 {
        (-self.distance).max(0.0)
    }
}

/// 箱 `a`・`b` の符号つきの分離量（15 本の軸）。
pub fn separation(a: &OrientedBox, b: &OrientedBox) -> Separation {
    search(a, b, true)
}

/// 軸の候補（`with_edges` が偽なら面の法線の 6 本だけ。辺どうしの外積の軸が要ることを試験で示すために分けてある）。
fn search(a: &OrientedBox, b: &OrientedBox, with_edges: bool) -> Separation {
    let (ua, ub) = (a.axes(), b.axes());
    let (ha, hb) = (a.half(), b.half());
    let delta = b.center - a.center;
    let mut candidates: Vec<Vec3> = ua.iter().chain(ub.iter()).copied().collect();
    if with_edges {
        for ea in &ua {
            for eb in &ub {
                let n = ea.cross(*eb);
                if n.norm() > PARALLEL_EDGE_EPS {
                    candidates.push(n.scale(1.0 / n.norm()));
                }
            }
        }
    }
    let mut best: Option<Separation> = None;
    for n in candidates {
        let radius = |h: &[f64; 3], u: &[Vec3; 3]| -> f64 {
            h.iter().zip(u).map(|(hi, ui)| hi * n.dot(*ui).abs()).sum()
        };
        let centre = n.dot(delta);
        let gap = centre.abs() - radius(&ha, &ua) - radius(&hb, &ub);
        // 同じ値なら先の軸（面の軸）を残す（組の順を入れ替えても同じ軸を選ぶ）
        if best.is_none_or(|s| gap > s.distance + TIE_EPS) {
            let axis = if centre < 0.0 { n.scale(-1.0) } else { n };
            best = Some(Separation {
                distance: gap,
                axis,
            });
        }
    }
    // 候補は常に 6 本以上あるので `best` は埋まる。念のため接しているとみなす
    best.unwrap_or(Separation {
        distance: 0.0,
        axis: Vec3::X,
    })
}

/// 軸どうしの隙間が同じとみなす差 [m]（丸め誤差）。
const TIE_EPS: f64 = 1e-12;

#[cfg(test)]
mod tests;
