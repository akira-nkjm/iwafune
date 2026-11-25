//! 面を覆う部品（面の平面に接する・横切る箱）。**ほかの部品に覆われた面の被覆の幾何はここだけ**
//! （熱の外面が、覆われた面積だけ環境とのやり取りをやめるのに使う。日照割合 `lit` と同じ材料）。
//!
//! 面 `F`（部品 `P` の箱の面、外向き法線 `n`）を覆うのは、`P` 以外の箱のうち、
//! (1) `F` の平面より [`SURFACE_TOLERANCE`] を超えて手前（`n` の側）に出る（面の裏から接する親の箱や、面の下に埋まる箱は覆わない）、かつ
//! (2) `F` の平面に [`SURFACE_TOLERANCE`] 以内で接する・横切る（面に載る・面を貫く箱）ものの、平面での接触面・断面を面の長方形で切った凸多角形。
//! 面から離れて浮く箱は覆わない（取付の隙間は許容に頼らない。接していなければ覆いではなく、影だけを作る）。
//! 面の部品に取り付けられた部品（取付の連鎖に面の部品がある部品）は、覆いに数えない: 面の光学特性と面積は、面に載る部品（太陽電池など）を含めた表面を表す慣例で
//! （`Footprint::Credited` と同じ）、載る部品が自分の外面を持っていても、面の部品の面積はその部品の下を含めて書く。ここで引くと二重に引くことになる。

//! 接していないが面の前に平行に向き合う箱（[`facing_of`]）は、面から見る形態係数だけ環境を塞ぐ（接する箱は覆いの規則だけで、二重に数えない）。

use super::super::geometry::{ExternalGeometry, SURFACE_TOLERANCE};
use super::{FACE_MATCH_COS, above_plane};
use crate::structure::interference::obb::BoxFace;
use sat_astro::math::planar::ConvexPolygon;
use sat_astro::view_factor::{Quadrature, rect_view_factor};

/// 面を覆う箱 1 つ。
#[derive(Debug, Clone)]
pub(super) struct Cover {
    /// 箱の並びの中の番号
    pub part: usize,
    /// 覆っている範囲（面の座標。面の長方形で切る）
    pub region: ConvexPolygon,
}

/// 部品 `owner` の面 `face` を覆う箱と範囲。`exempt` は箱ごと（箱の並びと同じ順）の「覆いに数えない」印
/// （面の部品に取り付けられた部品）。覆う面積が 0 の箱（辺・点で接するだけ）は含めない。
pub(super) fn covers_of(
    geometry: &ExternalGeometry,
    owner: usize,
    face: &BoxFace,
    exempt: &[bool],
) -> Vec<Cover> {
    let rectangle = face.rectangle();
    let offset = face.normal.dot(face.center);
    geometry
        .parts()
        .iter()
        .enumerate()
        .filter(|(j, _)| *j != owner && !exempt.get(*j).copied().unwrap_or(false))
        .filter_map(|(j, other)| {
            above_plane(&other.shape, face)?;
            let section = other
                .shape
                .plane_section(face.normal, offset, SURFACE_TOLERANCE)?;
            let local: Vec<[f64; 2]> = section.iter().map(|p| face.local(*p)).collect();
            let region = ConvexPolygon::hull(&local).intersection(&rectangle);
            (region.area() > 0.0).then_some(Cover { part: j, region })
        })
        .collect()
}

/// 面の前に平行に向き合う箱の面 1 つ（接していない）と、面からその面への形態係数。
#[derive(Debug, Clone)]
pub(super) struct Facing {
    /// 箱の並びの中の番号
    pub part: usize,
    /// 面 → 向き合う面の形態係数 `F`（遮蔽なし。`sat_astro::view_factor::rect_view_factor`）
    pub factor: f64,
    /// 面の平面から向き合う面までの距離 [m]（近い順に並べるのに使う）
    pub gap: f64,
}

/// 部品 `owner` の面 `face` の前にあり、**平行に向き合う**（法線が逆向き、cos が [`FACE_MATCH_COS`] 以上）箱の面と、面からの形態係数。
/// 面の長方形に（面への正射影で）重なるものだけを数え、接して覆う箱（`covered`）、取り付けられた部品（`exempt`）、面と同じ部品、
/// 別の箱に丸ごと含まれる内部の部品は数えない。傾いた箱は数えない。
/// 向き合う面どうしの放射の交換はここでは数えない（面を覆うことで環境とのやり取りが減る分だけ）。
pub(super) fn facing_of(
    geometry: &ExternalGeometry,
    owner: usize,
    face: &BoxFace,
    exempt: &[bool],
    covered: &[usize],
) -> Vec<Facing> {
    let rect = face.rect();
    let rectangle = face.rectangle();
    geometry
        .parts()
        .iter()
        .enumerate()
        .filter(|(j, _)| {
            *j != owner && !exempt.get(*j).copied().unwrap_or(false) && !covered.contains(j)
        })
        .filter(|(j, _)| !geometry.is_internal(*j))
        .filter_map(|(j, other)| {
            let own = other.shape.faces().into_iter().find(|g| {
                g.normal.dot(face.normal) <= -FACE_MATCH_COS
                    && (g.center - face.center).dot(face.normal) > SURFACE_TOLERANCE
            })?;
            let local: Vec<[f64; 2]> = own.corners().iter().map(|c| face.local(*c)).collect();
            if ConvexPolygon::hull(&local).intersection(&rectangle).area() <= 0.0 {
                return None;
            }
            let factor = rect_view_factor(&rect, &own.rect(), None, Quadrature::DEFAULT);
            let gap = (own.center - face.center).dot(face.normal);
            (factor > 0.0).then_some(Facing {
                part: j,
                factor,
                gap,
            })
        })
        .collect()
}
