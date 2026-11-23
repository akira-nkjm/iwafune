//! `structure.modal_fem` のテスト。閉じた式・導ける値・不変性で確かめる（許容差は各テストに理由を書く）。
//! 参照値の出典は各テストの doc に「確認した」か「記憶から」かを書く。

use super::*;
use sat_units::typed::{Frequency, Length, Mass};

mod box_model;
mod invariance;
mod plates;
mod strips;

/// アルミ合金の代表値（E = 70 GPa、ρ = 2700 kg/m³）。
pub const E_AL: f64 = 70.0e9;
pub const RHO_AL: f64 = 2700.0;

pub fn len(x: f64) -> Qty<Length> {
    Qty::from_raw(x)
}

pub fn pos(p: [f64; 3]) -> [Qty<Length>; 3] {
    p.map(len)
}

/// アルミのパネル（`size` は法線以外の 2 軸の昇順）。
pub fn panel(
    id: &str,
    normal: ModalFemAxis,
    center: [f64; 3],
    size: [f64; 2],
    t: f64,
    nu: f64,
) -> ModalFemPanel {
    ModalFemPanel {
        id: id.to_owned(),
        center: pos(center),
        normal,
        size: size.map(len),
        thickness: len(t),
        youngs_modulus: Qty::from_raw(E_AL),
        poisson_ratio: nu,
        density: Qty::from_raw(RHO_AL),
        mass: None,
    }
}

pub fn input(panels: Vec<ModalFemPanel>, support: ModalFemSupport) -> ModalFemInput {
    ModalFemInput {
        panels,
        lumped: vec![],
        distributed_mass: None,
        launch_axis: ModalFemAxis::Z,
        support,
        mesh_size: None,
        mode_count: None,
        merge_tolerance: None,
        drilling_factor: None,
    }
}

pub fn lumped(id: &str, p: [f64; 3], kg: f64) -> ModalFemLumped {
    ModalFemLumped {
        id: id.to_owned(),
        position: pos(p),
        mass: Qty::<Mass>::from_raw(kg),
        footprint: None,
        panel: None,
    }
}

pub fn solve(i: &ModalFemInput) -> ModalFemOutput {
    ModalFem::run(i).expect("the model solves")
}

pub fn hz(m: &ModalFemMode) -> f64 {
    let f: Qty<Frequency> = m.frequency;
    f.value()
}

/// 相対誤差。
pub fn rel(got: f64, reference: f64) -> f64 {
    (got - reference).abs() / reference.abs()
}

/// 4 辺（面 ±x、±y）の節点を固定する。`clamp = false` は並進だけ（単純支持。板の面内の動きも止めるが曲げには関係しない）、
/// `true` は回転も固定（四辺固定）。
pub fn four_edges(clamp: bool) -> ModalFemSupport {
    use ModalFemFace::*;
    ModalFemSupport::Fixed {
        items: [PlusX, MinusX, PlusY, MinusY]
            .into_iter()
            .map(|face| ModalFemFixedItem {
                target: ModalFemTarget::Face { face },
                clamp,
            })
            .collect(),
    }
}

/// 1 つの面を固定（クランプ）。
pub fn clamp_face(face: ModalFemFace) -> ModalFemSupport {
    ModalFemSupport::BaseFixed { face }
}

/// 中心が原点の閉じた箱（6 枚のパネル、中央面が箱の面）。
pub fn box_panels(sx: f64, sy: f64, sz: f64, t: f64) -> Vec<ModalFemPanel> {
    use ModalFemAxis::*;
    let mut v = Vec::new();
    for (name, sign) in [("P", 1.0), ("N", -1.0)] {
        v.push(panel(
            &format!("{name}X"),
            X,
            [sign * sx / 2.0, 0.0, 0.0],
            [sy, sz],
            t,
            0.33,
        ));
        v.push(panel(
            &format!("{name}Y"),
            Y,
            [0.0, sign * sy / 2.0, 0.0],
            [sx, sz],
            t,
            0.33,
        ));
        v.push(panel(
            &format!("{name}Z"),
            Z,
            [0.0, 0.0, sign * sz / 2.0],
            [sx, sy],
            t,
            0.33,
        ));
    }
    v
}

/// 全体を座標変換した入力。新しい座標 k = sign[k] · 古い座標 perm[k] + shift[k]（軸の入れ替え・反転・平行移動）。
/// 支持は `base_fixed`・`rails`・`free` だけに対応する。
pub fn remap(
    i: &ModalFemInput,
    perm: [usize; 3],
    sign: [f64; 3],
    shift: [f64; 3],
) -> ModalFemInput {
    let axis = |old: usize| (0..3).find(|k| perm[*k] == old).expect("a permutation");
    let point = |p: &[Qty<Length>; 3]| -> [Qty<Length>; 3] {
        std::array::from_fn(|k| len(sign[k] * p[perm[k]].value() + shift[k]))
    };
    let mut out = i.clone();
    for p in &mut out.panels {
        let old_n = p.normal.index();
        let old_axes: Vec<usize> = (0..3).filter(|a| *a != old_n).collect();
        let sizes: Vec<(usize, Qty<Length>)> = old_axes
            .iter()
            .copied()
            .zip(p.size.iter().copied())
            .collect();
        p.normal = ModalFemAxis::from_index(axis(old_n)).expect("axis");
        p.center = point(&p.center);
        let new_axes: Vec<usize> = (0..3).filter(|a| *a != p.normal.index()).collect();
        for (slot, na) in new_axes.iter().enumerate() {
            let size = sizes
                .iter()
                .find(|(old, _)| axis(*old) == *na)
                .expect("size")
                .1;
            p.size[slot] = size;
        }
    }
    for l in &mut out.lumped {
        l.position = point(&l.position);
        l.footprint = l.footprint.map(|f| std::array::from_fn(|k| f[perm[k]]));
    }
    out.launch_axis = ModalFemAxis::from_index(axis(i.launch_axis.index())).expect("axis");
    if let ModalFemSupport::BaseFixed { face } = i.support {
        let k = axis(face.axis().index());
        let max = face.is_max() == (sign[k] > 0.0);
        out.support = ModalFemSupport::BaseFixed {
            face: match (k, max) {
                (0, true) => ModalFemFace::PlusX,
                (0, false) => ModalFemFace::MinusX,
                (1, true) => ModalFemFace::PlusY,
                (1, false) => ModalFemFace::MinusY,
                (_, true) => ModalFemFace::PlusZ,
                (_, false) => ModalFemFace::MinusZ,
            },
        };
    }
    out
}

/// 登録した解析として JSON の入力で動く（`structure.modal_fem`）。タグ付きの列挙（支持・固定の対象）と面の綴り（`-x`）が読める。
#[test]
fn runs_from_json() {
    let json = serde_json::json!({
        "panels": [{
            "id": "P", "center": [0.0, 0.0, 0.0], "normal": "z", "size": [0.2, 0.2],
            "thickness": 0.001, "youngs_modulus": 70.0e9, "poisson_ratio": 0.3, "density": 2700.0
        }],
        "launch_axis": "z",
        "support": {"type": "base_fixed", "face": "-x"},
        "mode_count": 2
    });
    let out = crate::catalog::run_json("structure.modal_fem", json).expect("runs");
    let f = out["modes"][0]["frequency"].as_f64().expect("a number");
    assert!(f > 1.0 && f < 100.0, "{f}");
    assert_eq!(out["joints"].as_array().map(Vec::len), Some(0));
}
