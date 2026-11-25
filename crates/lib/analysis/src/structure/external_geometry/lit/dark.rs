//! 面の暗い範囲の多角形（影・接地面・覆い）から日照割合を出す。面全体も升の長方形も同じ関数（`FaceOccluders` が多角形を作り、ここが面積に直す）。

use super::super::cells::FaceCell;
use sat_astro::math::planar::{ConvexPolygon, union_measure};

// 日照割合を 0 か 1 に丸める相対の幅は親の `LIT_SNAP`
use super::LIT_SNAP;

/// 暗い範囲の多角形: 面に載る部品の影 `m`・接地面 `f`・そのほかの影と覆い `b`（それぞれ、軸に平行な外接の長方形つき）。
pub(super) struct DarkShapes {
    m: Vec<(ConvexPolygon, [f64; 4])>,
    f: Vec<(ConvexPolygon, [f64; 4])>,
    b: Vec<(ConvexPolygon, [f64; 4])>,
}

/// 多角形の軸に平行な外接の長方形 `[u 下限, v 下限, u 上限, v 上限]`（頂点が無ければ空の長方形）。
fn bounds_of(p: &ConvexPolygon) -> [f64; 4] {
    p.vertices().iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |b, v| {
            [
                b[0].min(v[0]),
                b[1].min(v[1]),
                b[2].max(v[0]),
                b[3].max(v[1]),
            ]
        },
    )
}

impl DarkShapes {
    pub(super) fn new(m: Vec<ConvexPolygon>, f: Vec<ConvexPolygon>, b: Vec<ConvexPolygon>) -> Self {
        let with_bounds = |v: Vec<ConvexPolygon>| -> Vec<(ConvexPolygon, [f64; 4])> {
            v.into_iter()
                .map(|p| {
                    let bounds = bounds_of(&p);
                    (p, bounds)
                })
                .collect()
        };
        Self {
            m: with_bounds(m),
            f: with_bounds(f),
            b: with_bounds(b),
        }
    }

    /// 日照割合（`region` が `Some` ならその升の長方形で切り、面積 `area` で割る。`None` なら面全体）。
    /// |(M \ F) ∪ B| = |M ∪ B ∪ F| − |F ∪ B| + |B|（B が空なら |M ∪ F| − |F|）。1e-12 以内は 0 か 1 に丸める。
    pub(super) fn lit(&self, region: Option<&FaceCell>, area: f64) -> f64 {
        let clip = region.map(FaceCell::polygon);
        let clip = clip.as_ref();
        // 升の長方形で切るのは多角形ごとに 1 度だけ（和集合 3 つで使い回す）。升と外接の長方形が重ならない多角形は、共通部分が空なので切らない
        let cell_bounds = clip.map(bounds_of);
        let cut = |polygons: &[(ConvexPolygon, [f64; 4])]| -> Vec<ConvexPolygon> {
            polygons
                .iter()
                .filter(|(_, b)| {
                    cell_bounds
                        .is_none_or(|c| b[0] < c[2] && c[0] < b[2] && b[1] < c[3] && c[1] < b[3])
                })
                .map(|(p, _)| clipped(p, clip))
                .collect()
        };
        let (m, f, b) = (cut(&self.m), cut(&self.f), cut(&self.b));
        let all: Vec<ConvexPolygon> = [&m, &f, &b].into_iter().flatten().cloned().collect();
        let fb: Vec<ConvexPolygon> = [&f, &b].into_iter().flatten().cloned().collect();
        // 升では、接地面が無ければ M \ F = M なので |M ∪ B| だけで足りる（面全体は引き算の丸めまで含めて同じ式をたどる）
        let covered = if region.is_some() && f.is_empty() {
            union_measure(&all).area
        } else {
            (union_measure(&all).area - union_measure(&fb).area + union_measure(&b).area).max(0.0)
        };
        let lit = 1.0 - covered / area;
        if lit <= LIT_SNAP {
            0.0
        } else if lit >= 1.0 - LIT_SNAP {
            1.0
        } else {
            lit
        }
    }
}

/// 多角形を升の長方形 `clip` で切る（`None` ならそのまま。面全体の計算は切らない）。
pub(super) fn clipped(polygon: &ConvexPolygon, clip: Option<&ConvexPolygon>) -> ConvexPolygon {
    match clip {
        Some(c) => polygon.intersection(c),
        None => polygon.clone(),
    }
}
