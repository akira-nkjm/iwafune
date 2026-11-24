//! フレームの部材の位置。フレームの箱（軸に平行）の 12 本の辺それぞれについて、辺の上にある既存の節点
//! （パネルのメッシュの節点。新しい節点は作らない）を辺に沿って並べ、隣り合う節点を部材（梁要素）でつなぐ。
//! 辺の上に節点が 2 つ未満の辺（パネルが辺に沿っていない）は部材にならず、出力に出す。

use super::MemberEdge;
use crate::structure::modal_fem::input::{ModalFemFace, ModalFemFrame};

/// 軸 `axis` の最大側（`max`）か最小側の面。
fn face(axis: usize, max: bool) -> ModalFemFace {
    ModalFemFace::ALL[2 * axis + usize::from(!max)]
}

/// 箱の 12 本の辺（軸 `a` に平行な 4 本ずつ）。辺の位置は他の 2 軸 (b, c) = (a+1, a+2) の最小・最大。
fn box_edges(lo: [f64; 3], hi: [f64; 3]) -> Vec<([ModalFemFace; 2], usize, [f64; 2])> {
    let mut out = Vec::new();
    for a in 0..3 {
        let (b, c) = ((a + 1) % 3, (a + 2) % 3);
        for max_b in [false, true] {
            for max_c in [false, true] {
                out.push((
                    [face(b, max_b), face(c, max_c)],
                    a,
                    [
                        if max_b { hi[b] } else { lo[b] },
                        if max_c { hi[c] } else { lo[c] },
                    ],
                ));
            }
        }
    }
    out
}

/// フレームの部材の辺（節点が 2 つ以上ある辺）と、部材にならなかった辺。
pub(super) fn edges(
    frame: &ModalFemFrame,
    coords: &[[f64; 3]],
    tol: f64,
) -> (Vec<MemberEdge>, Vec<[ModalFemFace; 2]>) {
    let half: [f64; 3] = std::array::from_fn(|k| frame.size[k].value() / 2.0);
    let center = frame.center.map(|c| c.value());
    let lo: [f64; 3] = std::array::from_fn(|k| center[k] - half[k]);
    let hi: [f64; 3] = std::array::from_fn(|k| center[k] + half[k]);
    let (mut modelled, mut missing) = (Vec::new(), Vec::new());
    for (faces, axis, fixed) in box_edges(lo, hi) {
        let (b, c) = ((axis + 1) % 3, (axis + 2) % 3);
        let mut nodes: Vec<(f64, usize)> = coords
            .iter()
            .enumerate()
            .filter(|(_, x)| {
                (x[b] - fixed[0]).abs() <= tol
                    && (x[c] - fixed[1]).abs() <= tol
                    && x[axis] >= lo[axis] - tol
                    && x[axis] <= hi[axis] + tol
            })
            .map(|(i, x)| (x[axis], i))
            .collect();
        nodes.sort_by(|p, q| p.0.total_cmp(&q.0));
        nodes.dedup_by(|p, q| (p.0 - q.0).abs() < 1.0e-12);
        if nodes.len() < 2 {
            missing.push(faces);
        } else {
            modelled.push(MemberEdge {
                faces,
                axis,
                fixed,
                nodes,
            });
        }
    }
    (modelled, missing)
}
