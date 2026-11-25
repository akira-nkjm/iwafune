//! 和集合の帯の掃引の本体（`union_measure` と、向きによらない多角形を先に前処理する `StaticUnion`）。
//!
//! 切れ目の縦座標は、全頂点の縦座標と、異なる多角形の辺どうしの交点の縦座標。交点は「入力の並びで先の多角形の辺」を
//! 先に渡して求める（先後を替えると丸めが変わるので、並びは変えない）。`StaticUnion` は並びの後ろの多角形どうしの
//! 切れ目を 1 度だけ求め、前に付く多角形（向きで変わる影）との組だけを呼ぶたびに求める。結果は `union_measure` と同じ値になる。

use super::{
    CUT_MERGE_RELATIVE, ConvexPolygon, PARALLEL_RELATIVE, Point2, SEGMENT_SLACK, UnionMeasure,
    cross2,
};
use std::cmp::Ordering;

/// 多角形の辺 `a → b`。`r = b − a` と、その長さ `len` は交点の判定のたびに使うので、先に求めておく。
#[derive(Debug, Clone, Copy)]
struct Edge {
    a: Point2,
    b: Point2,
    r: Point2,
    len: f64,
}

/// 前処理した多角形（辺と縦座標の範囲）。面積が 0 の多角形は作らない（和集合に何も足さない）。
#[derive(Debug, Clone)]
struct Prepared {
    edges: Vec<Edge>,
    span: (f64, f64),
}

impl Prepared {
    /// 面積を持つ多角形だけ前処理する。
    fn of(p: &ConvexPolygon) -> Option<Self> {
        if p.area().partial_cmp(&0.0) != Some(Ordering::Greater) {
            return None;
        }
        let v = &p.vertices;
        let n = v.len();
        let edges = (0..n)
            .map(|i| {
                let (a, b) = (v[i], v[(i + 1) % n]);
                let r = [b[0] - a[0], b[1] - a[1]];
                Edge {
                    a,
                    b,
                    r,
                    len: r[0].hypot(r[1]),
                }
            })
            .collect();
        let span = v
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(p[1]), hi.max(p[1]))
            });
        Some(Self { edges, span })
    }

    /// 縦の範囲が重なる組だけ、辺どうしの交点の縦座標を足す（`self` が先の多角形）。
    fn add_crossings(&self, other: &Self, ys: &mut Ordinates) {
        if self.span.1 < other.span.0 || other.span.1 < self.span.0 {
            return;
        }
        for ea in &self.edges {
            for eb in &other.edges {
                if let Some(y) = crossing_ordinate(ea, eb) {
                    ys.insert(y);
                }
            }
        }
    }

    fn add_vertices(&self, ys: &mut Ordinates) {
        for e in &self.edges {
            ys.insert(e.a[1]);
        }
    }
}

/// 辺 `ea` と `eb` の交点の縦座標（平行なら `None`、線分の外なら `None`）。
fn crossing_ordinate(ea: &Edge, eb: &Edge) -> Option<f64> {
    let (a, r, c, s) = (ea.a, ea.r, eb.a, eb.r);
    let denom = cross2(r, s);
    let norms = ea.len * eb.len;
    if denom.abs() <= PARALLEL_RELATIVE * norms {
        return None;
    }
    let ca = [c[0] - a[0], c[1] - a[1]];
    let t = cross2(ca, s) / denom;
    let u = cross2(ca, r) / denom;
    let inside = |v: f64| (-SEGMENT_SLACK..=1.0 + SEGMENT_SLACK).contains(&v);
    (inside(t) && inside(u)).then(|| a[1] + t.clamp(0.0, 1.0) * r[1])
}

/// 昇順（`total_cmp`）で重複（同じ値）のない縦座標の列。同じ縦座標は多くの辺の交点で何度も現れるので、入れるときに重複を除く。
#[derive(Debug, Clone, Default)]
struct Ordinates(Vec<f64>);

impl Ordinates {
    fn insert(&mut self, y: f64) {
        if let Err(at) = self.0.binary_search_by(|v| v.total_cmp(&y)) {
            self.0.insert(at, y);
        }
    }
}

/// 昇順にした 2 つの縦座標の列を合わせて、近いものを 1 つにまとめる（先頭を残す）。同じ値の重複は結果に効かない
/// （残した値に等しい値は、近い値として捨てられる）ので、列はどちらも重複を除いてよい。
/// 近さの幅は、両方の列の絶対値の最大に掛ける。
fn merge_cuts(fixed: &[f64], extra: &[f64]) -> Vec<f64> {
    let scale = fixed
        .iter()
        .chain(extra)
        .fold(0.0_f64, |m, y| m.max(y.abs()))
        .max(f64::MIN_POSITIVE);
    let tol = CUT_MERGE_RELATIVE * scale;
    let mut merged: Vec<f64> = Vec::with_capacity(fixed.len() + extra.len());
    let (mut i, mut j) = (0, 0);
    while i < fixed.len() || j < extra.len() {
        let take_fixed =
            j >= extra.len() || (i < fixed.len() && fixed[i].total_cmp(&extra[j]).is_le());
        let y = if take_fixed {
            i += 1;
            fixed[i - 1]
        } else {
            j += 1;
            extra[j - 1]
        };
        if merged.last().is_none_or(|last| y - last > tol) {
            merged.push(y);
        }
    }
    merged
}

/// 並びの前後に固定する多角形（向きによらないもの）を前処理しておき、間に付く多角形との和集合を求める。
/// 並びは `[先頭, 前の固定, 間, 後ろの固定]`。
#[derive(Debug, Clone)]
pub struct StaticUnion {
    head: Vec<Prepared>,
    tail: Vec<Prepared>,
    /// 固定の多角形の頂点の縦座標と、固定の多角形どうしの交点の縦座標（昇順・重複なし）
    ys: Vec<f64>,
}

impl StaticUnion {
    /// 固定する多角形（それぞれ並びの順）から作る。
    pub fn new(head: &[ConvexPolygon], tail: &[ConvexPolygon]) -> Self {
        let prepare = |p: &[ConvexPolygon]| p.iter().filter_map(Prepared::of).collect::<Vec<_>>();
        let (head, tail) = (prepare(head), prepare(tail));
        let mut ys = Ordinates::default();
        for s in head.iter().chain(&tail) {
            s.add_vertices(&mut ys);
        }
        let fixed: Vec<&Prepared> = head.iter().chain(&tail).collect();
        for (i, s) in fixed.iter().enumerate() {
            for other in &fixed[i + 1..] {
                s.add_crossings(other, &mut ys);
            }
        }
        Self {
            head,
            tail,
            ys: ys.0,
        }
    }

    /// `[leading, 前の固定, between, 後ろの固定]` の順に並べた `union_measure` と同じ値。
    pub fn measure(&self, leading: &[ConvexPolygon], between: &[ConvexPolygon]) -> UnionMeasure {
        let prepare = |p: &[ConvexPolygon]| p.iter().filter_map(Prepared::of).collect::<Vec<_>>();
        let (lead, mid) = (prepare(leading), prepare(between));
        let mut ys = Ordinates::default();
        for (i, s) in lead.iter().enumerate() {
            s.add_vertices(&mut ys);
            let later = lead[i + 1..]
                .iter()
                .chain(&self.head)
                .chain(&mid)
                .chain(&self.tail);
            for other in later {
                s.add_crossings(other, &mut ys);
            }
        }
        for s in &self.head {
            for other in &mid {
                s.add_crossings(other, &mut ys);
            }
        }
        for (i, s) in mid.iter().enumerate() {
            s.add_vertices(&mut ys);
            for other in mid[i + 1..].iter().chain(&self.tail) {
                s.add_crossings(other, &mut ys);
            }
        }
        let cuts = merge_cuts(&self.ys, &ys.0);
        let all: Vec<&Prepared> = lead
            .iter()
            .chain(&self.head)
            .chain(&mid)
            .chain(&self.tail)
            .collect();
        let total = sweep(&all, &cuts);
        UnionMeasure {
            area: total.area,
            centroid: (total.area > 0.0).then(|| [total.mx / total.area, total.my / total.area]),
        }
    }
}

/// 帯の中の、多角形 1 つの区間。左右の辺の直線を、帯の下端・中央・上端での x で持つ。
#[derive(Debug, Clone, Copy)]
struct Interval {
    left: [f64; 3],
    right: [f64; 3],
}

/// 帯ごとに、帯を縦に覆う多角形の区間（並びの順）を集めて和集合の面積と 1 次モーメントを足す。
/// 多角形ごとに、縦の範囲が帯の中央を含む帯だけを調べる（帯の中央は昇順なので二分探索で始まりを求める）。
fn sweep(polys: &[&Prepared], cuts: &[f64]) -> Moments {
    let slabs = cuts.len().saturating_sub(1);
    let centers: Vec<f64> = cuts.windows(2).map(|w| 0.5 * (w[0] + w[1])).collect();
    let mut buckets: Vec<Vec<Interval>> = vec![Vec::new(); slabs];
    for p in polys {
        let (lo, hi) = p.span;
        let first = centers.partition_point(|ym| lo.partial_cmp(ym) != Some(Ordering::Less));
        for k in first..slabs {
            let ym = centers[k];
            if ym.partial_cmp(&hi) != Some(Ordering::Less) {
                break;
            }
            let (y0, y1) = (cuts[k], cuts[k + 1]);
            if y1 - y0 <= 0.0 {
                continue;
            }
            buckets[k].extend(interval(p, y0, ym, y1));
        }
    }
    let mut total = Moments::default();
    for (k, bucket) in buckets.into_iter().enumerate() {
        for merged in merge(bucket) {
            total.add(&merged, cuts[k], cuts[k + 1]);
        }
    }
    total
}

/// 帯 `[y0, y1]`（中央 `ym`）での多角形の区間（中央の縦座標で左右の辺を選ぶ）。
fn interval(p: &Prepared, y0: f64, ym: f64, y1: f64) -> Option<Interval> {
    let mut left: Option<[f64; 3]> = None;
    let mut right: Option<[f64; 3]> = None;
    for Edge { a, b, .. } in &p.edges {
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
    Some(Interval {
        left: left?,
        right: right?,
    })
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
