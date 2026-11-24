//! 機体の外形（取付を持たず `size` を持つ部品が 1 つならその箱）。空力・太陽輻射圧の基準の箱（`aocs::reference_box`）、
//! 高度低下の面積、打ち上げの収納外形、熱の筐体が共有する。「根 = 機体の外形」という決め方は #388 で直す。
//!
//! 外形の消費者は箱を「機体座標の原点を中心に、軸に沿って置いた `size`」として読む。根の `[pose]`（機体座標での位置・姿勢。
//! `pose`）が効くようになったので、箱の軸が機体の軸から 90° の倍数だけ回っていれば寸法を入れ替えて渡し（[`body_extents`]）、
//! 位置がずれている・軸に沿わない回転のときは消費者が読み違えることを `outline.root_pose` の警告で知らせる（[`check`]。
//! 数値は作り替えない）。選び方と寸法の入れ替えはここだけ。

use crate::pose::body_pose;
use fune_model::design::EntityKey;
use fune_model::resolve::{Resolved, ResolvedPart};
use fune_model::{Design, Diagnostic, Target};
use sat_astro::math::Vec3;

/// 軸に沿った回転とみなす許容差（回転行列の各成分が 0 か ±1 からこれ以内）。姿勢はファイルに 6 桁で書くので、
/// 90° でも成分に 1e-6 ほどの丸めが出る。
const AXIS_ALIGNED_TOLERANCE: f64 = 1e-4;
/// 位置のずれとみなさない上限 [m]（合成の丸め）。
const OFFSET_EPS_M: f64 = 1e-9;

/// 外形の候補（取付を持たず `size` を持つ部品。1 つだけなら機体の外形、複数なら決まらない）。**選ぶ規則はここだけ。**
pub(crate) fn candidates(resolved: &Resolved) -> Vec<&ResolvedPart> {
    resolved
        .parts
        .values()
        .filter(|p| p.mount.is_none() && p.vec3("size").is_some())
        .collect()
}

/// 部品座標の回転行列の各成分（行 = 機体の軸、列 = 部品の軸）。
fn rotation_rows(resolved: &Resolved, part: &str) -> Option<[[f64; 3]; 3]> {
    let (_, q) = body_pose(resolved, part)?;
    let cols = [Vec3::X, Vec3::Y, Vec3::Z].map(|e| q.rotate(e));
    Some([0, 1, 2].map(|i| cols.map(|c| [c.x, c.y, c.z][i])))
}

/// 回転行列が軸の入れ替え（各成分が 0 か ±1）なら、成分を丸めて返す。
fn axis_aligned(rows: [[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let mut out = [[0.0; 3]; 3];
    for (i, row) in rows.iter().enumerate() {
        for (j, v) in row.iter().enumerate() {
            let r = v.round();
            if (v - r).abs() > AXIS_ALIGNED_TOLERANCE {
                return None;
            }
            out[i][j] = r.abs();
        }
    }
    Some(out)
}

/// 部品の箱の、機体の軸に沿った寸法 [m]。部品の姿勢が軸の入れ替え（90° の倍数）のときだけ `Some`
/// （`size` を入れ替えて返す）。姿勢が無いか斜めなら `None`。**箱の軸が機体の軸に沿うかの判定と寸法の入れ替えはここだけ**
/// （外形の根・粗い有限要素のパネルと搭載部品の外形が共有する）。
pub fn axis_aligned_extents(resolved: &Resolved, part: &str, size: [f64; 3]) -> Option<[f64; 3]> {
    let m = rotation_rows(resolved, part).and_then(axis_aligned)?;
    Some(m.map(|row| row.iter().zip(size).map(|(a, s)| a * s).sum()))
}

/// 外形の根の箱の、機体の軸に沿った寸法 [m]。根の姿勢が軸の入れ替え（90° の倍数）なら `size` を入れ替えて返す。
/// そうでなければ `size` のまま（警告は [`check`]）。姿勢が無ければ `size` のまま。
pub fn body_extents(resolved: &Resolved, root: &str, size: [f64; 3]) -> [f64; 3] {
    axis_aligned_extents(resolved, root, size).unwrap_or(size)
}

/// 外形の根の `[pose]` を消費者が読み違えるときの警告（`outline.root_pose`）。機体座標の位置がずれているか、
/// 回転が 90° の倍数でない根だけ。基準と、根の姿勢を上書きしている形態のそれぞれで見る。
pub fn check(design: &Design, resolved: &Resolved) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    warn_in(design, resolved, None, &mut out);
    // 形態ごとに解決した設計（選ぶのではなく全部を見る）
    for (id, configured) in &resolved.configurations {
        warn_in(design, configured, Some(id), &mut out);
    }
    out
}

fn warn_in(
    design: &Design,
    resolved: &Resolved,
    configuration: Option<&str>,
    out: &mut Vec<Diagnostic>,
) {
    let [root] = candidates(resolved)[..] else {
        return;
    };
    let id = root.id.as_str();
    let overridden = configuration.is_some_and(|c| {
        design
            .configurations
            .get(c)
            .is_some_and(|c| c.pose.contains_key(id))
    });
    // 形態が根の姿勢を上書きしていなければ基準と同じ（基準で警告済み）
    if configuration.is_some() && !overridden {
        return;
    }
    let Some((position, _)) = body_pose(resolved, id) else {
        return;
    };
    let offset = position.norm() > OFFSET_EPS_M;
    let turned = rotation_rows(resolved, id).and_then(axis_aligned).is_none();
    if !offset && !turned {
        return;
    }
    let what = match (offset, turned) {
        (true, true) => {
            "an offset and a rotation that is not a multiple of 90 degrees about the body axes"
        }
        (true, false) => "an offset from the body origin",
        _ => "a rotation that is not a multiple of 90 degrees about the body axes",
    };
    let message = format!(
        "Part `{id}` is the vehicle outline (the only part without a mount that has a size) and its pose has {what}. \
         The aerodynamic and solar-pressure reference box, the decay area, the launch-fit envelope and the thermal enclosure \
         treat the outline as a box of its size centred on the body origin and aligned to the body axes. \
         Mass properties, interference and the 3-D view do use the pose"
    );
    let (entity, keys, target) = match configuration {
        Some(c) => (
            EntityKey::Configuration(c.to_owned()),
            vec!["pose", id],
            Target::configuration(c, Some(&format!("pose.{id}"))),
        ),
        None => (
            EntityKey::Part(id.to_owned()),
            vec!["pose"],
            Target::pose(id, None),
        ),
    };
    out.push(
        Diagnostic::warning("outline.root_pose", message)
            .with_target(target)
            .at(design.source_map.locate(&entity, &keys)),
    );
}
