//! 円錐と向きつきの箱の交差。頂点・軸・半頂角（と、あれば頂点からの距離の上限）を持つ円錐のうちに、箱の点があるか。
//!
//! **判定は厳密**（円錐は丸い錐で、箱は凸の多面体）。近似も下限もなく、接する場合（誤差の範囲）は交差に数える側へ倒す。
//! 部品の箱が実物より大きいことによる過大側の誤差は、この幾何ではなく箱の近似に由来する（解析の判定文に書く）。
//!
//! 計算は、頂点を原点にした座標で行う。
//! - **最小の角度**（軸と、箱の点を結ぶ向きの角）: 軸の半直線が箱に当たれば 0。当たらなければ、最小は箱の境界にあり
//!   （面の上の角度は、軸が当たる点だけを極小に持つ準凸の関数なので、面が軸を含まなければ面の縁へ寄る）、**12 本の辺**で閉じた形に解く。
//!   辺 `p + s·d` の点の向きは、原点と辺を含む平面の大円の弧をなし、軸 `a` に最も近い向きは `a` をその平面へ射影した向き
//!   （射影が 0 なら軸が平面に垂直で、角度は一定の 90°）。辺の両端と、この向きを向く点の中の最小が辺の最小になる。
//! - **円錐の中の最短距離**（頂点から、箱のうち円錐の中にある部分への距離）: 凸集合 `箱 ∩ 円錐` の頂点に最も近い点は、
//!   頂点が箱の中なら 0、そうでなければ集合の境界にあり、次のどれかになる（すべて閉じた形）。
//!   (1) 箱の頂点、(2) 辺と円錐の面の交点・辺の上の頂点に最も近い点（円錐との交点は 2 次方程式）、
//!   (3) 面の内部で頂点に最も近い点（頂点から面の平面への垂線の足）、(4) 面と円錐の面の交線（円・楕円・放物線・双曲線）の上で最も近い点
//!   （面の法線に最も近い母線）。それぞれが箱と円錐に入っているものから最小を取る。
//! - 交差する ⟺ 最小の角度 ≤ 半頂角（±1e-12 rad）で、距離の上限があれば、さらに最短距離 ≤ 上限。
//!
//! 報告する最小の角度は、距離の上限を考えない箱全体での値（上限の内側の部分だけでの最小とは、上限が箱を切る場合に異なりうる）。

use crate::structure::interference::obb::OrientedBox;
use sat_astro::math::Vec3;
use std::f64::consts::FRAC_PI_2;

/// 接する場合を交差に数える角度の余裕 [rad]（丸め誤差）。
const ANGLE_EPS: f64 = 1e-12;
/// 点が箱・円錐に入っているとみなす長さの余裕（相対。`1 + |p|` を掛ける）。
const LENGTH_EPS: f64 = 1e-9;

/// 円錐（頂点・軸・半頂角・距離の上限）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cone {
    /// 頂点（機体座標系）[m]
    pub apex: Vec3,
    /// 軸の向き（単位ベクトル）
    pub axis: Vec3,
    /// 半頂角 [rad]（0 より大きく 90° より小さい）
    pub half_angle: f64,
    /// 頂点からの距離の上限 [m]（無ければ無限）
    pub range: Option<f64>,
}

impl Cone {
    /// 円錐を作る（`axis` は正規化する）。軸が零・非有限、半頂角が (0, 90°) の外、上限が負・非有限なら `None`。
    pub fn new(apex: Vec3, axis: Vec3, half_angle: f64, range: Option<f64>) -> Option<Self> {
        let axis = axis.normalized()?;
        let finite = |v: Vec3| v.to_array().iter().all(|x| x.is_finite());
        let angle_ok = half_angle.is_finite() && half_angle > 0.0 && half_angle < FRAC_PI_2;
        let range_ok = range.is_none_or(|r| r.is_finite() && r >= 0.0);
        (finite(apex) && finite(axis) && angle_ok && range_ok).then_some(Self {
            apex,
            axis,
            half_angle,
            range,
        })
    }
}

/// 円錐と箱が交わるときの、報告する量。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConeContact {
    /// 軸から箱の点までの最小の角度 [rad]（軸が箱に当たれば 0。距離の上限は考えない）
    pub min_angle: f64,
    /// 頂点から、箱のうち円錐（と距離の上限）の中にある部分までの最短距離 [m]（頂点が箱の中なら 0）
    pub min_distance: f64,
}

/// 円錐と箱の交差。交わらなければ `None`。
pub fn contact(cone: &Cone, b: &OrientedBox) -> Option<ConeContact> {
    let local = Local::new(cone, b);
    if local.contains(Vec3::ZERO) {
        return Some(ConeContact {
            min_angle: 0.0,
            min_distance: 0.0,
        });
    }
    let (min_angle, at) = min_angle(&local, cone.axis);
    if min_angle > cone.half_angle + ANGLE_EPS {
        return None;
    }
    // 円錐に入る点の最小。最小の角度の点（円錐に入る）も候補に入れ、交わるなら必ず距離が定まるようにする
    let min_distance = nearest_in_cone(&local, cone).min(at.norm());
    if cone.range.is_some_and(|r| min_distance > r) {
        return None;
    }
    Some(ConeContact {
        min_angle,
        min_distance,
    })
}

/// 頂点を原点にした座標の箱。
struct Local {
    /// 箱の中心
    c: Vec3,
    /// 箱の 3 軸
    u: [Vec3; 3],
    /// 半幅
    h: [f64; 3],
}

impl Local {
    fn new(cone: &Cone, b: &OrientedBox) -> Self {
        Self {
            c: b.center - cone.apex,
            u: b.axes(),
            h: b.half(),
        }
    }

    /// 長さの余裕（箱の大きさに対する相対）。
    fn tol(&self, p: Vec3) -> f64 {
        LENGTH_EPS * (1.0 + p.norm() + self.c.norm())
    }

    /// 点が箱（境界を含む）に入っているか。
    fn contains(&self, p: Vec3) -> bool {
        let t = self.tol(p);
        (0..3).all(|i| (p - self.c).dot(self.u[i]).abs() <= self.h[i] + t)
    }

    /// 頂点 8 つ（符号の組 `k` の下位 3 ビットが各軸の ±）。
    fn vertex(&self, k: usize) -> Vec3 {
        let s = |bit: usize| if k >> bit & 1 == 1 { 1.0 } else { -1.0 };
        self.c
            + self.u[0].scale(s(0) * self.h[0])
            + self.u[1].scale(s(1) * self.h[1])
            + self.u[2].scale(s(2) * self.h[2])
    }

    /// 辺 12 本 `(始点, 向きつきの長さ)`。軸 `i` に沿う辺は、他の 2 軸の符号の 4 通り。
    fn edges(&self) -> Vec<(Vec3, Vec3)> {
        let mut out = Vec::with_capacity(12);
        for i in 0..3 {
            for k in 0..8usize {
                if k >> i & 1 == 0 {
                    out.push((self.vertex(k), self.u[i].scale(2.0 * self.h[i])));
                }
            }
        }
        out
    }
}

/// 向き `p` と軸 `a` の間の角度 [rad]（`p` が零なら 0）。
fn angle_between(p: Vec3, a: Vec3) -> f64 {
    p.cross(a).norm().atan2(p.dot(a))
}

/// 軸の半直線 `t·a`（`t ≥ 0`）が箱に入る最小の `t`（当たらなければ `None`）。スラブ法。
fn axis_entry(l: &Local, a: Vec3) -> Option<f64> {
    let (mut t0, mut t1) = (0.0_f64, f64::INFINITY);
    for i in 0..3 {
        let origin = -l.c.dot(l.u[i]);
        let dir = a.dot(l.u[i]);
        if dir.abs() < 1e-15 {
            if origin.abs() > l.h[i] + l.tol(Vec3::ZERO) {
                return None;
            }
        } else {
            let (mut ta, mut tb) = ((-l.h[i] - origin) / dir, (l.h[i] - origin) / dir);
            if ta > tb {
                std::mem::swap(&mut ta, &mut tb);
            }
            t0 = t0.max(ta);
            t1 = t1.min(tb);
        }
    }
    (t0 <= t1 + l.tol(Vec3::ZERO)).then_some(t0)
}

/// 線分 `p + s·d`（`s ∈ [0, 1]`）の上の、軸 `a` からの角度が最小の点。
fn segment_min_angle_point(p: Vec3, d: Vec3, a: Vec3) -> Vec3 {
    let mut candidates = vec![p, p + d];
    let n = p.cross(d);
    // 原点を通る直線なら向きは一定（両端で足りる）
    if n.norm() > 1e-12 * p.norm() * d.norm() && n.norm() > 0.0 {
        let n = n.scale(1.0 / n.norm());
        let in_plane = a - n.scale(a.dot(n));
        if in_plane.norm() > 1e-12 {
            let u = in_plane.scale(1.0 / in_plane.norm());
            let w = n.cross(u);
            let den = d.dot(w);
            if den.abs() > 1e-15 {
                let s = (-p.dot(w) / den).clamp(0.0, 1.0);
                candidates.push(p + d.scale(s));
            }
        }
    }
    candidates
        .into_iter()
        .min_by(|x, y| angle_between(*x, a).total_cmp(&angle_between(*y, a)))
        .unwrap_or(p)
}

/// 箱の点の軸からの最小の角度と、その点。
fn min_angle(l: &Local, a: Vec3) -> (f64, Vec3) {
    if let Some(t) = axis_entry(l, a) {
        return (0.0, a.scale(t));
    }
    let mut best = (f64::INFINITY, l.c);
    for (p, d) in l.edges() {
        let q = segment_min_angle_point(p, d, a);
        let angle = angle_between(q, a);
        if angle < best.0 {
            best = (angle, q);
        }
    }
    best
}

/// 箱のうち円錐（上限は考えない）に入る点の、頂点からの最短距離。入る点が候補に無ければ無限。
fn nearest_in_cone(l: &Local, cone: &Cone) -> f64 {
    let a = cone.axis;
    let (cos, sin) = (cone.half_angle.cos(), cone.half_angle.sin());
    let mut best = f64::INFINITY;
    let mut take = |p: Vec3| {
        let along = p.dot(a);
        let t = l.tol(p);
        if l.contains(p) && along >= -t && along >= p.norm() * cos - t {
            best = best.min(p.norm());
        }
    };
    for k in 0..8 {
        take(l.vertex(k));
    }
    for (p, d) in l.edges() {
        for s in edge_parameters(p, d, a, cos) {
            take(p + d.scale(s));
        }
    }
    for i in 0..3 {
        for sign in [1.0, -1.0] {
            let n = l.u[i].scale(sign);
            // 面の平面は `n·p = dist`
            let dist = n.dot(l.c) + l.h[i];
            take(n.scale(dist));
            // 面と円錐の面の交線で最も近い点: 法線に最も近い母線（`n·g` の絶対値が最大）
            let perp = n - a.scale(n.dot(a));
            if perp.norm() > 1e-12 {
                let e = perp.scale(1.0 / perp.norm());
                for side in [1.0, -1.0] {
                    let g = a.scale(cos) + e.scale(side * sin);
                    let den = n.dot(g);
                    if den.abs() > 1e-12 && dist / den > 0.0 {
                        take(g.scale(dist / den));
                    }
                }
            }
        }
    }
    best
}

/// 辺 `p + s·d` の上の候補の `s`: 両端、頂点に最も近い点、円錐の面との交点（`(q·a)² = cos²θ |q|²` の 2 次方程式の根）。
/// すべて `[0, 1]` に収める。
fn edge_parameters(p: Vec3, d: Vec3, a: Vec3, cos: f64) -> Vec<f64> {
    let mut out = vec![0.0, 1.0];
    let dd = d.dot(d);
    if dd > 0.0 {
        out.push(-p.dot(d) / dd);
    }
    let c2 = cos * cos;
    let qa = 2.0 * ((p.dot(a)) * (d.dot(a)) - c2 * p.dot(d));
    let qq = d.dot(a).powi(2) - c2 * dd;
    let qc = p.dot(a).powi(2) - c2 * p.dot(p);
    let scale = dd.max(p.dot(p)).max(f64::MIN_POSITIVE);
    if qq.abs() < 1e-14 * scale {
        if qa.abs() > 1e-14 * scale {
            out.push(-qc / qa);
        }
    } else {
        let disc = qa * qa - 4.0 * qq * qc;
        if disc >= 0.0 {
            let root = disc.sqrt();
            out.push((-qa + root) / (2.0 * qq));
            out.push((-qa - root) / (2.0 * qq));
        }
    }
    out.into_iter()
        .filter(|s| s.is_finite())
        .map(|s| s.clamp(0.0, 1.0))
        .collect()
}

#[cfg(test)]
mod tests;
