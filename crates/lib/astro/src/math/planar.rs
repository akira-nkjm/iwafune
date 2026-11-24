//! 平面の凸多角形と、その和集合の面積・図心（厳密）。**凸多角形の和集合はここだけ**（箱の投影の重なり・面の被覆などが使う）。
//!
//! 和集合は **帯の掃引** で出す。平面を、全頂点の縦座標と、異なる多角形の辺どうしの交点の縦座標で水平な帯に切る。
//! 1 つの帯の中では、どの多角形も左右 2 本の辺に挟まれた台形で、辺どうしの前後も入れ替わらない。よって帯の中央で
//! 各多角形の区間 `[左, 右]` を左端の順に並べて区間の和集合を取れば、その帯のどこでも同じ区間の組になり、
//! 和集合の左右の端は 1 本ずつの辺（の直線）のままである。幅は縦座標の 1 次式なので、面積は台形の公式、
//! x の 1 次モーメント `∫(R² − L²)/2 dy` と y の 1 次モーメント `∫ y (R − L) dy` は 2 次式で、Simpson の公式が厳密に積分する。
//! 格子で数える方法と違い、解像度という調整値が無く、結果は決定的で、閉じた式と 1e-12 の相対誤差で一致する。
//!
//! 縮退（箱の組み立てでは普通）への扱い:
//! - 辺や面がぴったり重なる・共有する、同じ多角形、他の多角形の辺の上に頂点がある: 帯の端が同じ縦座標になるだけで、
//!   中央の区間の和集合が重なりを 1 度しか数えない。平行な辺どうしは交点を作らない（前後が入れ替わらない）。
//! - 面積が 0 の多角形（板を真横から見た線分・点）: 面積を持たないので何も足さない（縦座標も切らない）。
//! - ほぼ平行な辺: 交点が帯の外にずれても、取り違える量は辺の間隔（ほぼ 0）× 帯の高さで、面積の丸め誤差の程度にしかならない。
//! - 縦座標が 1e-13（相対）以内で近い切れ目は 1 つにまとめる（極端に薄い帯で中央の判定が不安定になるのを避ける）。
//!
//! 入力の順に結果は依存しない（和の順序による丸め誤差の違いだけ）。

use std::cmp::Ordering;

/// 平面の点 `[x, y]`。
pub type Point2 = [f64; 2];

/// 切れ目の縦座標を 1 つにまとめる相対の幅（座標の大きさと範囲の大きいほうに掛ける）。
const CUT_MERGE_RELATIVE: f64 = 1e-13;
/// 凸包で、3 点が一直線上にあるとみなす外積の大きさ（点群の対角線の 2 乗に対する相対値）。丸め誤差（~1e-16）より十分大きく、
/// 形の判別（板厚 mm 対 長さ m で ~1e-3）より十分小さい。
const COLLINEAR_RELATIVE: f64 = 1e-14;
/// 辺どうしを平行とみなす（交点を作らない）外積の相対値。
const PARALLEL_RELATIVE: f64 = 1e-14;
/// 辺どうしの交点の位置パラメータの余裕（端の上の交点を取りこぼさない）。
const SEGMENT_SLACK: f64 = 1e-12;

/// 凸多角形（頂点は反時計回り。頂点が 3 つに満たなければ面積 0 の退化した形）。
#[derive(Debug, Clone, PartialEq)]
pub struct ConvexPolygon {
    vertices: Vec<Point2>,
}

impl ConvexPolygon {
    /// 点群の凸包から作る（反時計回り。一直線上の点は落とす。重なる点は 1 つにする）。
    /// 全部が一直線上なら端の 2 点（線分）、1 点なら 1 点の退化した形になる。
    pub fn hull(points: &[Point2]) -> Self {
        Self {
            vertices: convex_hull(points),
        }
    }

    /// 軸に平行な長方形 `[x0, x1] × [y0, y1]`（`x0 <= x1`・`y0 <= y1`）。
    pub fn rectangle(min: Point2, max: Point2) -> Self {
        Self {
            vertices: vec![min, [max[0], min[1]], max, [min[0], max[1]]],
        }
    }

    /// 頂点（反時計回り）。
    pub fn vertices(&self) -> &[Point2] {
        &self.vertices
    }

    /// 面積（靴紐の公式。退化した形は 0）。
    pub fn area(&self) -> f64 {
        if self.vertices.len() < 3 {
            return 0.0;
        }
        let n = self.vertices.len();
        let twice: f64 = (0..n)
            .map(|i| cross2(self.vertices[i], self.vertices[(i + 1) % n]))
            .sum();
        (twice * 0.5).max(0.0)
    }

    /// 別の凸多角形との共通部分（Sutherland–Hodgman。凸どうしなので結果も凸）。共通部分が無ければ空（頂点 0 個）。
    pub fn intersection(&self, clip: &Self) -> Self {
        if self.vertices.len() < 3 || clip.vertices.len() < 3 {
            return Self {
                vertices: Vec::new(),
            };
        }
        let mut out = self.vertices.clone();
        let n = clip.vertices.len();
        for i in 0..n {
            let (a, b) = (clip.vertices[i], clip.vertices[(i + 1) % n]);
            out = clip_by_edge(&out, a, b);
            if out.is_empty() {
                break;
            }
        }
        Self { vertices: out }
    }
}

/// 和集合の面積と図心。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnionMeasure {
    /// 面積
    pub area: f64,
    /// 図心 `[x, y]`。面積が 0 なら `None`
    pub centroid: Option<Point2>,
}

/// 凸多角形の和集合の面積と図心（帯の掃引。上のモジュールの説明を参照）。
pub fn union_measure(polygons: &[ConvexPolygon]) -> UnionMeasure {
    let live: Vec<&ConvexPolygon> = polygons.iter().filter(|p| p.area() > 0.0).collect();
    let cuts = cut_ordinates(&live);
    let mut total = Moments::default();
    for slab in cuts.windows(2) {
        let (y0, y1) = (slab[0], slab[1]);
        let intervals = slab_intervals(&live, y0, y1);
        for merged in merge(intervals) {
            total.add(&merged, y0, y1);
        }
    }
    UnionMeasure {
        area: total.area,
        centroid: (total.area > 0.0).then(|| [total.mx / total.area, total.my / total.area]),
    }
}

/// 外積 `a × b`（z 成分）。
fn cross2(a: Point2, b: Point2) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

/// 点群の凸包（Andrew の単調連鎖。反時計回り）。
fn convex_hull(points: &[Point2]) -> Vec<Point2> {
    let mut pts: Vec<Point2> = points
        .iter()
        .copied()
        .filter(|p| p[0].is_finite() && p[1].is_finite())
        .collect();
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    pts.dedup();
    if pts.len() < 3 {
        return pts;
    }
    let (min, max) = bounds(&pts);
    let diagonal2 = (max[0] - min[0]).powi(2) + (max[1] - min[1]).powi(2);
    let eps = COLLINEAR_RELATIVE * diagonal2;
    let mut lower = half_hull(pts.iter().copied(), eps);
    let mut upper = half_hull(pts.iter().rev().copied(), eps);
    lower.pop();
    upper.pop();
    lower.extend(upper);
    // 全部が一直線上なら、両方の連鎖が端の 2 点だけになって 2 点に戻る
    if lower.len() < 3 {
        return vec![pts[0], pts[pts.len() - 1]];
    }
    lower
}

/// 単調連鎖の半分（左へ曲がらない点を落とす）。
fn half_hull(points: impl Iterator<Item = Point2>, eps: f64) -> Vec<Point2> {
    let mut h: Vec<Point2> = Vec::new();
    for p in points {
        while h.len() >= 2 {
            let (a, b) = (h[h.len() - 2], h[h.len() - 1]);
            if cross2([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]) <= eps {
                h.pop();
            } else {
                break;
            }
        }
        h.push(p);
    }
    h
}

/// 点群の外接する軸平行な長方形（最小・最大）。
fn bounds(points: &[Point2]) -> (Point2, Point2) {
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for p in points {
        for k in 0..2 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }
    (min, max)
}

/// 凸多角形を、有向の辺 `a → b` の左側（辺の上を含む）で切る。
fn clip_by_edge(subject: &[Point2], a: Point2, b: Point2) -> Vec<Point2> {
    let side = |p: Point2| cross2([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
    let mut out = Vec::with_capacity(subject.len() + 2);
    let n = subject.len();
    for i in 0..n {
        let (p, q) = (subject[i], subject[(i + 1) % n]);
        let (sp, sq) = (side(p), side(q));
        if sp >= 0.0 {
            out.push(p);
        }
        if (sp > 0.0 && sq < 0.0) || (sp < 0.0 && sq > 0.0) {
            let t = sp / (sp - sq);
            out.push([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]);
        }
    }
    out
}

/// 全多角形の頂点の縦座標と、異なる多角形の辺どうしの交点の縦座標（昇順・近いものは 1 つ）。
fn cut_ordinates(polys: &[&ConvexPolygon]) -> Vec<f64> {
    let mut ys: Vec<f64> = polys
        .iter()
        .flat_map(|p| p.vertices.iter().map(|v| v[1]))
        .collect();
    let spans: Vec<(f64, f64)> = polys.iter().map(|p| y_span(p)).collect();
    for i in 0..polys.len() {
        for j in (i + 1)..polys.len() {
            // 縦の範囲が重ならない組は辺が交わらない
            if spans[i].1 < spans[j].0 || spans[j].1 < spans[i].0 {
                continue;
            }
            for (a, b) in edges(polys[i]) {
                for (c, d) in edges(polys[j]) {
                    if let Some(y) = crossing_ordinate(a, b, c, d) {
                        ys.push(y);
                    }
                }
            }
        }
    }
    ys.sort_by(f64::total_cmp);
    let scale = ys
        .iter()
        .fold(0.0_f64, |m, y| m.max(y.abs()))
        .max(f64::MIN_POSITIVE);
    let tol = CUT_MERGE_RELATIVE * scale;
    let mut merged: Vec<f64> = Vec::with_capacity(ys.len());
    for y in ys {
        if merged.last().is_none_or(|last| y - last > tol) {
            merged.push(y);
        }
    }
    merged
}

/// 多角形の縦座標の範囲。
fn y_span(p: &ConvexPolygon) -> (f64, f64) {
    p.vertices
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v[1]), hi.max(v[1]))
        })
}

/// 多角形の辺（頂点の組、反時計回り）。
fn edges(p: &ConvexPolygon) -> impl Iterator<Item = (Point2, Point2)> + '_ {
    let n = p.vertices.len();
    (0..n).map(move |i| (p.vertices[i], p.vertices[(i + 1) % n]))
}

/// 線分 `a–b` と `c–d` の交点の縦座標（平行なら `None`、線分の外なら `None`）。
fn crossing_ordinate(a: Point2, b: Point2, c: Point2, d: Point2) -> Option<f64> {
    let r = [b[0] - a[0], b[1] - a[1]];
    let s = [d[0] - c[0], d[1] - c[1]];
    let denom = cross2(r, s);
    let norms = (r[0].hypot(r[1])) * (s[0].hypot(s[1]));
    if denom.abs() <= PARALLEL_RELATIVE * norms {
        return None;
    }
    let ca = [c[0] - a[0], c[1] - a[1]];
    let t = cross2(ca, s) / denom;
    let u = cross2(ca, r) / denom;
    let inside = |v: f64| (-SEGMENT_SLACK..=1.0 + SEGMENT_SLACK).contains(&v);
    (inside(t) && inside(u)).then(|| a[1] + t.clamp(0.0, 1.0) * r[1])
}

/// 帯の中の、多角形 1 つの区間。左右の辺の直線を、帯の下端・中央・上端での x で持つ。
#[derive(Debug, Clone, Copy)]
struct Interval {
    left: [f64; 3],
    right: [f64; 3],
}

/// 帯 `[y0, y1]` を縦に覆う多角形の区間（中央の縦座標で左右の辺を選ぶ）。
fn slab_intervals(polys: &[&ConvexPolygon], y0: f64, y1: f64) -> Vec<Interval> {
    let ym = 0.5 * (y0 + y1);
    if y1 - y0 <= 0.0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for p in polys {
        let (lo, hi) = y_span(p);
        if !(lo < ym && ym < hi) {
            continue;
        }
        let mut left: Option<[f64; 3]> = None;
        let mut right: Option<[f64; 3]> = None;
        for (a, b) in edges(p) {
            if (a[1] < ym) == (b[1] < ym) || a[1] == b[1] {
                continue;
            }
            let at = |y: f64| a[0] + (y - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
            let line = [at(y0), at(ym), at(y1)];
            if left.is_none_or(|l| line[1] < l[1]) {
                left = Some(line);
            }
            if right.is_none_or(|r| line[1] > r[1]) {
                right = Some(line);
            }
        }
        if let (Some(left), Some(right)) = (left, right) {
            out.push(Interval { left, right });
        }
    }
    out
}

/// 区間の和集合（中央での左端の順。重なる・接する区間は 1 つにし、右端は大きいほうの辺）。
fn merge(mut intervals: Vec<Interval>) -> Vec<Interval> {
    intervals.sort_by(|a, b| {
        a.left[1]
            .total_cmp(&b.left[1])
            .then_with(|| a.right[1].total_cmp(&b.right[1]))
    });
    let mut out: Vec<Interval> = Vec::new();
    for next in intervals {
        match out.last_mut() {
            Some(cur) if next.left[1].partial_cmp(&cur.right[1]) != Some(Ordering::Greater) => {
                if next.right[1] > cur.right[1] {
                    cur.right = next.right;
                }
            }
            _ => out.push(next),
        }
    }
    out
}

/// 面積と 1 次モーメントの和。
#[derive(Debug, Default)]
struct Moments {
    area: f64,
    mx: f64,
    my: f64,
}

impl Moments {
    /// 帯 `[y0, y1]` の中の、左右が直線の区間を足す（台形の面積と、Simpson の公式による厳密な 1 次モーメント）。
    fn add(&mut self, iv: &Interval, y0: f64, y1: f64) {
        let h = y1 - y0;
        let ys = [y0, 0.5 * (y0 + y1), y1];
        let width = |k: usize| iv.right[k] - iv.left[k];
        let mx = |k: usize| 0.5 * (iv.right[k].powi(2) - iv.left[k].powi(2));
        let my = |k: usize| ys[k] * width(k);
        self.area += 0.5 * h * (width(0) + width(2));
        self.mx += h / 6.0 * (mx(0) + 4.0 * mx(1) + mx(2));
        self.my += h / 6.0 * (my(0) + 4.0 * my(1) + my(2));
    }
}

#[cfg(test)]
mod tests;
