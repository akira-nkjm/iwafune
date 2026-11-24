//! 箱の中の空洞の面どうしの形態係数（遮蔽つき）。構造の箱が部品を囲むとき、その内側の面と部品の面が互いを見る割合を出す。
//! **箱の面どうしの形態係数（遮蔽つき）の計算はここだけ**（熱の内部の放射 `thermal::network::view_factors` が使う。一般の長方形の積分は `sat_astro::view_factor::rect`）。
//!
//! 規則（箱は `structure.external_geometry` と同じ `InterferenceBox`。**入力は箱だけ**）:
//! - **空洞**: ほかの 1 つの箱に丸ごと含まれる部品（内部の部品。判定は [`ExternalGeometry`] の `container_of`。含む箱が複数なら番号が最小のもの）を
//!   収める箱が 1 つの空洞。含む箱（容器）は **中空の殻** とみなし、その 6 面の **内側**（法線を内向きに反転した面）が壁。壁の厚みは見ない。
//!   空洞の中の面は、壁 6 面と、内部の部品の箱の 6 面（外向き）。**空洞に入らない部品**（外に出ている部品・容器でも内部でもない箱。
//!   パネル・展開物・外面の部品）は面を持たず、視線も遮らない（容器の内側の見通しの外にある）。
//! - **遮蔽**: 空洞に属する内部の部品の箱（面の両端の部品を除く）が、点から点への視線を遮る。箱は各辺から [`SHRINK`] だけ縮めて判定する
//!   （面にかすめるだけの視線は遮らない）。重なり合う箱の中に始点があれば遮られる。
//! - **面の組**: 同じ空洞の 2 つの面。互いに相手の面の表側に頂点が 1 つでもあるものだけを積分する（背中合わせ・同じ向きは 0）。
//!   **接している面の組は除く**（`BoxFace::contact_area` が正。箱どうしが接している面は、放射でなく伝導で結ばれる。向き合う距離 0 の核は発散する）。
//!   除いた組の数は `contact_pairs`。隙間があるだけの近接した面は積分する（`rect_view_factor` は近接でも厳密）。
//! - 面積 × 形態係数 `A_i F_ij` は対称で、組を 1 回だけ計算する（`i < j`）。
//! - **品質の指標**: 面ごとの `Σ_j F_ij`（`row_sums`）。閉じた空洞なら 1。1 から外れる分は、除いた接触面の組・重なった箱の中に入った視線・
//!   求積の誤差（遮蔽つきは数 %）の合計で、**この空洞の面の見え方のうち数えられなかった割合** の目安。
//!
//! 計算量: 面の組は面の数の 2 乗。向き合わない組は積分の前に除き、遮蔽の候補は 2 つの面の外接の範囲に入る箱だけに絞る。

use super::external_geometry::geometry::SURFACE_TOLERANCE;
use super::external_geometry::{ExternalGeometry, PartBox};
use super::interference::obb::{BoxFace, SegmentBox};
use super::interference::{DEFAULT_TOLERANCE, InterferenceBox};
use sat_astro::math::Vec3;
use sat_astro::view_factor::{Quadrature, rect_view_factor};
use std::collections::BTreeMap;

/// 視線の遮蔽の判定で、箱を各辺から縮める長さ [m]。内部の判定の許容（`SURFACE_TOLERANCE`）と同じ。
pub const SHRINK: f64 = SURFACE_TOLERANCE;

/// 空洞の面 1 つ。
#[derive(Debug, Clone, PartialEq)]
pub struct CavitySurface {
    /// 面を持つ部品（壁なら容器の部品）
    pub part: String,
    /// 面を持つ箱の番号（箱の並び = `ExternalGeometry::parts` の添字。同じ部品の面どうしの判定に使う）
    pub owner: usize,
    /// 容器の内側の壁か（偽なら内部の部品の外向きの面）
    pub wall: bool,
    /// 空洞の番号（`CavityViews::containers` の添字）
    pub cavity: usize,
    /// 空洞の中を向く面（壁は内向きに反転済み）
    pub face: BoxFace,
}

/// 互いを見る面の組（`a < b`）の `A_a F_ab`（対称）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FacePair {
    pub a: usize,
    pub b: usize,
    /// `A_a F_ab = A_b F_ba` [m²]
    pub exchange: f64,
    /// 向き合っているのに、視線がすべて遮られた（遮蔽がなければ正の値）
    pub blocked: bool,
}

/// 空洞の面どうしの形態係数。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CavityViews {
    /// 空洞を作る容器の部品（番号の順）
    pub containers: Vec<String>,
    pub surfaces: Vec<CavitySurface>,
    /// 向き合う面の組（接触の組を除く。`exchange > 0` か `blocked`）
    pub pairs: Vec<FacePair>,
    /// 面ごとの `Σ_j F_ij`（`surfaces` と同じ順。接触の組と数えなかった分は含まれない）
    pub row_sums: Vec<f64>,
    /// 接していて、放射でなく伝導とする面の組の数
    pub contact_pairs: usize,
    /// 空洞に入らない部品（面も遮蔽もない。ID の昇順）
    pub outside: Vec<String>,
}

/// 箱から空洞の面どうしの形態係数を出す。内部の部品が 1 つも無ければ（空洞が無ければ）`None`。
pub fn cavity_views(boxes: &[InterferenceBox]) -> Option<CavityViews> {
    let geometry = ExternalGeometry::from_boxes(boxes)?;
    let parts = geometry.parts();
    let mut cavities: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..parts.len() {
        if let Some(container) = geometry.container_of(i) {
            cavities.entry(container).or_default().push(i);
        }
    }
    if cavities.is_empty() {
        return None;
    }
    let mut views = CavityViews::default();
    let mut inside = vec![false; parts.len()];
    for (cavity, (&container, members)) in cavities.iter().enumerate() {
        inside[container] = true;
        views.containers.push(parts[container].id.clone());
        let start = views.surfaces.len();
        add_surfaces(&mut views.surfaces, cavity, container, members, parts);
        for &m in members {
            inside[m] = true;
        }
        let occluders: Vec<Occluder> = members
            .iter()
            .map(|&m| Occluder::new(&parts[m], m))
            .collect();
        integrate(&mut views, start, &occluders);
    }
    views.outside = (0..parts.len())
        .filter(|&i| !inside[i])
        .map(|i| parts[i].id.clone())
        .collect();
    Some(views)
}

/// 空洞の面を並べる（壁 6 面、続いて内部の部品ごとの 6 面。部品は並びの順）。
fn add_surfaces(
    out: &mut Vec<CavitySurface>,
    cavity: usize,
    container: usize,
    members: &[usize],
    parts: &[PartBox],
) {
    for face in parts[container].shape.faces() {
        out.push(CavitySurface {
            part: parts[container].id.clone(),
            owner: container,
            wall: true,
            cavity,
            face: face.flipped(),
        });
    }
    for &m in members {
        for face in parts[m].shape.faces() {
            out.push(CavitySurface {
                part: parts[m].id.clone(),
                owner: m,
                wall: false,
                cavity,
                face,
            });
        }
    }
}

/// 遮蔽になる箱（空洞の内部の部品）。
struct Occluder {
    index: usize,
    slab: SegmentBox,
    lo: Vec3,
    hi: Vec3,
}

impl Occluder {
    fn new(part: &PartBox, index: usize) -> Self {
        let (lo, hi) = bounds(&part.shape.corners());
        Self {
            index,
            slab: SegmentBox::new(&part.shape, SHRINK),
            lo,
            hi,
        }
    }
}

/// 点の集まりの軸に平行な外接の範囲。
fn bounds(points: &[Vec3]) -> (Vec3, Vec3) {
    let first = points.first().copied().unwrap_or(Vec3::ZERO);
    points.iter().fold((first, first), |(lo, hi), c| {
        (
            Vec3::new(lo.x.min(c.x), lo.y.min(c.y), lo.z.min(c.z)),
            Vec3::new(hi.x.max(c.x), hi.y.max(c.y), hi.z.max(c.z)),
        )
    })
}

/// 2 つの面が互いを見うるか（互いに、相手の頂点が自分の面の表側に 1 つでもある）。
fn may_face(a: &BoxFace, b: &BoxFace) -> bool {
    let above = |f: &BoxFace, g: &BoxFace| {
        g.corners()
            .iter()
            .any(|c| (*c - f.center).dot(f.normal) > SHRINK)
    };
    above(a, b) && above(b, a)
}

/// 空洞の面（`views.surfaces[start..]`）の全組を積分する。
fn integrate(views: &mut CavityViews, start: usize, occluders: &[Occluder]) {
    let end = views.surfaces.len();
    views.row_sums.resize(end, 0.0);
    for i in start..end {
        for j in (i + 1)..end {
            let (fa, fb) = (views.surfaces[i].face, views.surfaces[j].face);
            let (oi, oj) = (views.surfaces[i].owner, views.surfaces[j].owner);
            // 同じ部品の箱の外向きの面どうしは見えない（壁の内側の面どうしは見える）
            if oi == oj && !views.surfaces[i].wall {
                continue;
            }
            // 接している面の組は、向き合う判定（互いの面の表側に頂点がある）より先に調べる（接する面の頂点は面の平面の上にある）
            if fa.contact_area(&fb, DEFAULT_TOLERANCE.value()).is_some() {
                views.contact_pairs += 1;
                continue;
            }
            if !may_face(&fa, &fb) {
                continue;
            }
            let exchange = pair_exchange(&fa, &fb, occluders, [oi, oj]);
            if exchange.value > 0.0 {
                views.row_sums[i] += exchange.value / fa.area();
                views.row_sums[j] += exchange.value / fb.area();
            }
            if exchange.value > 0.0 || exchange.blocked {
                views.pairs.push(FacePair {
                    a: i,
                    b: j,
                    exchange: exchange.value,
                    blocked: exchange.blocked,
                });
            }
        }
    }
}

struct Exchange {
    value: f64,
    blocked: bool,
}

/// 面の組の `A_a F_ab`。2 つの面の外接の範囲にかかる箱だけを、視線の遮蔽に使う。
fn pair_exchange(fa: &BoxFace, fb: &BoxFace, occluders: &[Occluder], skip: [usize; 2]) -> Exchange {
    let (lo, hi) = bounds(&[fa.corners(), fb.corners()].concat());
    let near: Vec<&SegmentBox> = occluders
        .iter()
        .filter(|o| !skip.contains(&o.index))
        .filter(|o| {
            o.lo.x <= hi.x
                && o.hi.x >= lo.x
                && o.lo.y <= hi.y
                && o.hi.y >= lo.y
                && o.lo.z <= hi.z
                && o.hi.z >= lo.z
        })
        .map(|o| &o.slab)
        .collect();
    let (ra, rb) = (fa.rect(), fb.rect());
    let clear = near.is_empty();
    let visible = |p: Vec3, q: Vec3| !near.iter().any(|s| s.blocks(p, q));
    let f = rect_view_factor(
        &ra,
        &rb,
        if clear { None } else { Some(&visible) },
        Quadrature::DEFAULT,
    );
    let value = fa.area() * f;
    // 遮蔽がなければ正の値だったか（すべて遮られた組は壁越しに見えない組として数える）
    let blocked =
        !clear && value <= 0.0 && rect_view_factor(&ra, &rb, None, Quadrature::DEFAULT) > 0.0;
    Exchange { value, blocked }
}

#[cfg(test)]
mod tests;
