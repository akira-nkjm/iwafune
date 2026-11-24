//! 線分と向きつきの箱の交差（スラブ法）。点から点への見通し（`structure::cavity_views` が面どうしの視線の遮蔽に使う）。
//! **線分が箱の内部を通るかの判定はここだけ。**
//!
//! 箱の座標系（軸 3 本）で、線分 `p + t (q − p)`（`t ∈ [0, 1]`）が 3 つのスラブ `|x_k| ≤ h_k` の共通部分に入る `t` の区間を求め、
//! その区間が正の長さを持てば通る。箱の表面をかすめるだけの線分（区間の長さが 0）は通らないものとする。
//! `shrink` は箱の各半幅から引く長さ [m]: 面や辺に接する視線（隣り合う箱の面に沿う線分）を遮りにしない余裕。

use super::OrientedBox;
use sat_astro::math::Vec3;

/// 平行とみなす方向成分の下限（線分の長さに対する比ではなく、箱の軸への射影 [m]）。
const PARALLEL_EPS: f64 = 1e-15;

/// 線分の判定用に、箱の軸と半幅を 1 度だけ求めて持つ箱（たくさんの線分を同じ箱に当てるとき）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentBox {
    center: Vec3,
    axes: [Vec3; 3],
    half: [f64; 3],
}

impl SegmentBox {
    /// 箱から作る。各半幅から `shrink`（0 以上）を引く（0 未満にはしない）。
    pub fn new(b: &OrientedBox, shrink: f64) -> Self {
        Self {
            center: b.center,
            axes: b.axes(),
            half: b.half().map(|h| (h - shrink.max(0.0)).max(0.0)),
        }
    }

    /// 線分 `p → q` が箱の内部を通るか（始点・終点が内部にあっても通る）。
    pub fn blocks(&self, p: Vec3, q: Vec3) -> bool {
        let (o, d) = (p - self.center, q - p);
        let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
        for k in 0..3 {
            let (ok, dk, hk) = (o.dot(self.axes[k]), d.dot(self.axes[k]), self.half[k]);
            if dk.abs() < PARALLEL_EPS {
                if ok.abs() >= hk {
                    return false;
                }
                continue;
            }
            let (t1, t2) = ((-hk - ok) / dk, (hk - ok) / dk);
            lo = lo.max(t1.min(t2));
            hi = hi.min(t1.max(t2));
            if hi <= lo {
                return false;
            }
        }
        hi > lo
    }
}

impl OrientedBox {
    /// 線分 `p → q` が箱の内部を通るか。`shrink` は箱を各辺から縮める長さ [m]（面にかすめるだけの視線を通す余裕）。
    /// 同じ箱に何度も当てるなら [`SegmentBox`] を使う。
    pub fn blocks_segment(&self, p: Vec3, q: Vec3, shrink: f64) -> bool {
        SegmentBox::new(self, shrink).blocks(p, q)
    }
}

#[cfg(test)]
mod tests;
