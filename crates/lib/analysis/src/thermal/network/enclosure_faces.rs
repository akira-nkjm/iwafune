//! 外殻（箱）の 6 面を担う節点を、外面から決める。**外面がどの箱の面の上にあるかの判断はここだけ**（`couplings` が使う）。
//!
//! 面 `i`（`BOX_FACES` の順）の候補になる外面は、(1) 法線がその面を向く（`FACE_NORMAL_COS`）、(2) 面積が箱の面の面積に近い（`FACE_AREA_TOLERANCE`。員数を掛ける）もの。
//! 候補の節点が 1 つならそれに決める（従来どおり。部品の箱が面の上になくても、箱が無くてもよい）。
//! **候補の節点が複数あるときだけ、位置で絞る**: 外面の部品の箱の、法線が面を向く面（部品の外向きの面）が、外殻の箱の面の平面から [`FACE_PLANE_TOLERANCE`]
//! （箱の最も短い辺の割合）以内にあり、外殻の面と重なる面積が箱の面の面積の半分以上ある（`BoxFace::contact_area`）候補に絞る。
//! 面積と向きだけでは、機体の外に立つ板（展開したパドルの裏）が、同じ面積・向きの箱の面と取り違えられるので、位置で見分ける。
//! 部品の箱が分からない候補（寸法を持たない部品）は位置が決まらないので残し、部品の箱が 1 つも渡されない入力（`boxes` が空）は絞らない。
//! 6 面が 1 つずつの別の節点に決まらなければ、理由（英文）を返し、外殻の放射は使わない。

use super::ThermalSurface;
use super::mount_contact::oriented;
use crate::structure::interference::obb::{BoxFace, OrientedBox};
use crate::thermal::enclosure::{BoxExchange, ThermalEnclosure};
use sat_astro::math::{Quat, Vec3};
use sat_astro::view_factor::BOX_FACES;

/// 面の法線が軸と一致するとみなす余弦の下限（約 2.6°）。
const FACE_NORMAL_COS: f64 = 0.999;
/// 外面の面積を箱の面の面積とみなす相対許容差。
const FACE_AREA_TOLERANCE: f64 = 0.05;
/// 外面の面が箱の面の平面にあるとみなす距離（箱の最も短い辺に対する割合）。パネルの板厚と、その上に載る表面材（太陽電池）の厚さを許す。
/// 機体から離れて立つ板（展開したパドル、畳んで浮かせたパドル）はこれを超える。
const FACE_PLANE_TOLERANCE: f64 = 0.05;
/// 外面の面が外殻の面と重なる面積の、箱の面の面積に対する下限。
const FACE_OVERLAP_MIN: f64 = 0.5;

/// 箱の面の外向き法線（`BOX_FACES` の順）。
fn face_normal(i: usize) -> Vec3 {
    let mut v = [0.0; 3];
    v[i / 2] = if i.is_multiple_of(2) { 1.0 } else { -1.0 };
    Vec3::new(v[0], v[1], v[2])
}

/// 外殻の 6 面（`BOX_FACES` の順）と許容距離。外殻の箱の位置が決まらなければ `None`。
fn enclosure_faces(e: &ThermalEnclosure) -> Option<([BoxFace; 6], f64)> {
    let size = e.size.as_ref()?;
    let size = Vec3::new(size.x.value(), size.y.value(), size.z.value());
    let faces =
        OrientedBox::from_size(e.center.unwrap_or(Vec3::ZERO), Quat::IDENTITY, size).faces();
    let tolerance = FACE_PLANE_TOLERANCE * size.x.min(size.y).min(size.z);
    // `OrientedBox::faces` の順（軸ごとに負の側・正の側）から `BOX_FACES` の順（軸ごとに正の側・負の側）へ
    let ordered = std::array::from_fn(|i| faces[2 * (i / 2) + usize::from(i.is_multiple_of(2))]);
    Some((ordered, tolerance))
}

/// 外面 `s` が外殻の面 `face` の上にあるか。部品の箱が分からなければ決められないので真。
fn on_face(e: &ThermalEnclosure, s: &ThermalSurface, face: &BoxFace, tolerance: f64) -> bool {
    let Some(shape) = e.boxes.iter().find(|b| b.id == s.id).and_then(oriented) else {
        return true;
    };
    // 部品の箱の面のうち、法線が外殻の面を向く面（外向きに揃えるため `flipped`。`contact_area` は逆向きの面の組を接触とみなす）
    let Some(own) = shape
        .faces()
        .into_iter()
        .filter(|f| f.normal.dot(face.normal) >= FACE_NORMAL_COS)
        .max_by(|a, b| {
            a.normal
                .dot(face.normal)
                .total_cmp(&b.normal.dot(face.normal))
        })
    else {
        return false;
    };
    face.contact_area(&own.flipped(), tolerance)
        .is_some_and(|area| area >= FACE_OVERLAP_MIN * face.area())
}

/// 6 面を担う節点。面ごとに、法線がその面を向き面積が箱の面に近い外面（候補）を集め、候補の節点が 1 つならそれに決める（従来どおり。
/// 箱が面の上になくても、箱が無くてもよい）。節点が複数あるときだけ、箱の面の上にある候補に絞り、なお 1 つに決まらなければ理由を返す。
/// 6 面の節点がすべて異なるときだけ `Ok`。そうでなければ理由（英文）を返す。
pub(super) fn face_nodes(
    surfaces: &[ThermalSurface],
    exchange: &BoxExchange,
    e: &ThermalEnclosure,
) -> Result<Vec<String>, String> {
    let located = if e.boxes.is_empty() {
        None
    } else {
        enclosure_faces(e)
    };
    let mut nodes: Vec<String> = Vec::with_capacity(6);
    for i in 0..6 {
        let want = exchange.area_m2[i];
        let candidates: Vec<&ThermalSurface> = surfaces
            .iter()
            .filter(|s| {
                let area = s.area.value() * f64::from(s.count);
                s.normal
                    .normalized()
                    .is_some_and(|n| n.dot(face_normal(i)) >= FACE_NORMAL_COS)
                    && (area - want).abs() <= FACE_AREA_TOLERANCE * want
            })
            .collect();
        let distinct = |c: &[&ThermalSurface]| -> Vec<String> {
            let mut v: Vec<String> = c.iter().map(|s| s.node.clone()).collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        let mut found = distinct(&candidates);
        if found.len() > 1
            && let Some((faces, tol)) = &located
        {
            let on: Vec<&ThermalSurface> = candidates
                .iter()
                .copied()
                .filter(|s| on_face(e, s, &faces[i], *tol))
                .collect();
            found = distinct(&on);
        }
        match found.as_slice() {
            [node] => nodes.push(node.clone()),
            [] => {
                let hint = if located.is_some() && !candidates.is_empty() {
                    " (an outer surface must lie on the face)"
                } else {
                    ""
                };
                return Err(format!(
                    "No outer surface node was found for the {} face of the box{hint}",
                    BOX_FACES[i]
                ));
            }
            _ => {
                return Err(format!(
                    "The {} face of the box matches several nodes",
                    BOX_FACES[i]
                ));
            }
        }
    }
    let mut sorted = nodes.clone();
    sorted.sort_unstable();
    if sorted.windows(2).any(|w| w[0] == w[1]) {
        return Err("Two faces of the box belong to the same node".to_owned());
    }
    Ok(nodes)
}

#[cfg(test)]
mod tests;
