//! 空間の任意の位置・向きの長方形どうしの形態係数（Lambert 面）。遮蔽は呼び出し側が与える可視判定で数える。
//! **一般の長方形どうしの形態係数の数値積分はここだけ**（閉じた式は親の `view_factor`。両者は試験で一致を確かめる）。
//!
//! 面 A から面 B への形態係数 `F_AB = (1/A_A) ∬ cosθ_A cosθ_B / (π r²) V dA_A dA_B`（`V` は 2 点が互いに見えれば 1）を、
//! A の点 `p` ごとの `F(p→B)` の面積平均として求める:
//! - `F(p→B)`（遮蔽なし）は、`p` の接平面より上にある B の部分（半空間で切る）を多角形として、Nusselt の輪郭積分
//!   `F = (1/2π) Σ_k β_k (n·ĝ_k)` で **厳密に** 出す（`β_k` は辺 k が `p` に張る角、`ĝ_k` は `p` と辺の作る平面の単位法線）。
//!   面が近接していても（隙間が面の寸法より十分小さくても）積分の特異性が出ない。
//! - A の面内の平均は、A の `source × source` 点のガウス・ルジャンドル則（`F(p→B)` は A の中で滑らかで、辺の近くに折れ目があるだけ）。
//! - 遮蔽（`visible` を与えたとき）は、`p` から B の `target × target` 点（ガウス・ルジャンドル則）への線分の可視判定で、
//!   **核 `cosθ_A cosθ_B / (π r²)` で重みづけた見える割合** `v(p)` を出して `F(p→B)` に掛ける（`F = Σ w_p F(p→B) v(p)`）。
//!   遮蔽がなければ `v = 1` で、上の厳密な値に一致する。遮蔽があるときの誤差は、見える割合のサンプル化（1 つの `p` で `target²` 点の重みづけ）。
//!
//! 面の向き: `normal` が放射する側（外向き）。`u × v = normal`。向き合っていない（片方が相手の面の裏側にある）対は 0。

use crate::math::Vec3;
use crate::math::quadrature::gauss_legendre;
use std::f64::consts::PI;

/// 長方形（中心・外向き法線・面内の 2 本の単位ベクトル・半幅）。`u × v = normal`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub center: Vec3,
    /// 放射する側の単位法線
    pub normal: Vec3,
    pub u: Vec3,
    pub v: Vec3,
    /// `u`・`v` 方向の半幅 [m]
    pub half_u: f64,
    pub half_v: f64,
}

impl Rect {
    /// 面積 [m²]。
    pub fn area(&self) -> f64 {
        4.0 * self.half_u * self.half_v
    }

    /// 面の中の点（`s`・`t` は `[-1, 1]` の座標。0 が中心）。
    pub fn point(&self, s: f64, t: f64) -> Vec3 {
        self.center + self.u.scale(s * self.half_u) + self.v.scale(t * self.half_v)
    }

    /// 4 つの頂点。法線の側から見て反時計回り。
    pub fn corners(&self) -> [Vec3; 4] {
        [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(s, t)| self.point(s, t))
    }

    /// 同じ面の反対側（法線を反転。`u` はそのまま、`v` を反転して `u × v = normal` を保つ）。
    pub fn flipped(&self) -> Self {
        Self {
            normal: -self.normal,
            v: -self.v,
            ..*self
        }
    }
}

/// 求積の細かさ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quadrature {
    /// 面 A の 1 辺あたりの点数（`F(p→B)` の面内の平均）
    pub source: usize,
    /// 遮蔽の可視判定に使う、面 B の 1 辺あたりの点数
    pub target: usize,
}

impl Quadrature {
    /// 既定の細かさ。`source = 10`（遮蔽なしの値の誤差は試験に記録）、`target = 6`（見える割合のサンプル。1 つの `p` で 36 本の線分）。
    pub const DEFAULT: Self = Self {
        source: 10,
        target: 6,
    };
}

/// 微小面（点 `p`、法線 `n`）から長方形 `b` を見る形態係数（遮蔽なし、厳密）。`b` が `p` の側を向いていなければ 0。
pub fn point_to_rect(p: Vec3, n: Vec3, b: &Rect) -> f64 {
    // 面 B の平面の表側にいなければ B は p を見ない（B は平面なので全体が表か裏）
    if (p - b.center).dot(b.normal) <= 0.0 {
        return 0.0;
    }
    let above = clip_above(&b.corners(), p, n);
    if above.len() < 3 {
        return 0.0;
    }
    let w: Vec<Vec3> = above.iter().map(|c| *c - p).collect();
    let mut sum = 0.0;
    for k in 0..w.len() {
        let (a, c) = (w[k], w[(k + 1) % w.len()]);
        let cross = a.cross(c);
        let len = cross.norm();
        if len > 0.0 {
            sum += a.angle_to(c) * n.dot(cross.scale(1.0 / len));
        }
    }
    // 法線の側から見て反時計回りの頂点は、p から見ると p の法線 n に対して時計回りで、和が負になる
    (-sum / (2.0 * PI)).clamp(0.0, 1.0)
}

/// 多角形を、点 `p` を通り法線 `n` の平面の表側（`(x − p)·n > 0`）の部分に切る（Sutherland–Hodgman）。頂点の順は保つ。
fn clip_above(poly: &[Vec3], p: Vec3, n: Vec3) -> Vec<Vec3> {
    let height = |x: Vec3| (x - p).dot(n);
    let mut out = Vec::with_capacity(poly.len() + 1);
    for k in 0..poly.len() {
        let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
        let (ha, hb) = (height(a), height(b));
        if ha > 0.0 {
            out.push(a);
        }
        if (ha > 0.0) != (hb > 0.0) && ha != hb {
            out.push(a + (b - a).scale(ha / (ha - hb)));
        }
    }
    out
}

/// 面 A から面 B への形態係数。`visible(x, y)` は点 `x`（A の上）と点 `y`（B の上）が互いに見えるか（遮蔽のないときは `None`）。
pub fn rect_view_factor(
    a: &Rect,
    b: &Rect,
    visible: Option<&dyn Fn(Vec3, Vec3) -> bool>,
    quad: Quadrature,
) -> f64 {
    let nodes = gauss_legendre(quad.source);
    let targets = gauss_legendre(quad.target);
    let mut total = 0.0;
    for &(s, ws) in &nodes {
        for &(t, wt) in &nodes {
            let p = a.point(s, t);
            let f = point_to_rect(p, a.normal, b);
            if f <= 0.0 {
                continue;
            }
            let seen = visible.map_or(1.0, |v| visible_fraction(p, a, b, v, &targets));
            total += 0.25 * ws * wt * f * seen;
        }
    }
    total
}

/// 点 `p`（面 A の上）から面 B が見える割合（核 `cosθ_A cosθ_B / r²` で重みづけ。0〜1）。
fn visible_fraction(
    p: Vec3,
    a: &Rect,
    b: &Rect,
    visible: &dyn Fn(Vec3, Vec3) -> bool,
    targets: &[(f64, f64)],
) -> f64 {
    let (mut all, mut seen) = (0.0, 0.0);
    for &(s, ws) in targets {
        for &(t, wt) in targets {
            let q = b.point(s, t);
            let d = q - p;
            let r2 = d.norm_squared();
            let (ca, cb) = (a.normal.dot(d), -b.normal.dot(d));
            if r2 <= 0.0 || ca <= 0.0 || cb <= 0.0 {
                continue;
            }
            let weight = ws * wt * ca * cb / (r2 * r2);
            all += weight;
            if visible(p, q) {
                seen += weight;
            }
        }
    }
    if all > 0.0 { seen / all } else { 0.0 }
}

#[cfg(test)]
mod tests;
