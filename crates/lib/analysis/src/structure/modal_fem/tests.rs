//! `structure.modal_fem` のテスト。閉じた式・導ける値・不変性で確かめる（許容差は各テストに理由を書く）。
//! 参照値の出典は各テストの doc に「確認した」か「記憶から」かを書く。

use super::*;
use sat_units::typed::{Frequency, Length, Mass};

mod beam_element;
mod box_model;
mod frame;
mod invariance;
mod plates;
mod shapes;
mod skewed;
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
        orientation: None,
    }
}

pub fn input(panels: Vec<ModalFemPanel>, support: ModalFemSupport) -> ModalFemInput {
    ModalFemInput {
        panels,
        lumped: vec![],
        frames: vec![],
        skipped_panels: vec![],
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
        frame: None,
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
    for f in &mut out.frames {
        f.center = point(&f.center);
        f.size = std::array::from_fn(|k| f.size[perm[k]]);
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

/// 面の綴りは往復でき、serde の綴りと同じ（`+x`・`-z`）。
#[test]
fn face_spellings_round_trip() {
    for face in ModalFemFace::ALL {
        assert_eq!(ModalFemFace::parse(face.as_str()), Some(face));
        let json = serde_json::to_value(face).expect("serializes");
        assert_eq!(json, serde_json::json!(face.as_str()));
    }
    assert_eq!(ModalFemFace::parse("z"), None);
}

/// 打ち上げ条件の拘束の綴りの意味: 面が要るものは面が無ければ拒み、`rails`・`free` は面を使わず、知らない綴りは拒む。
#[test]
fn launch_kinds_make_supports() {
    use ModalFemSupportError::{FaceRequired, UnknownKind};
    let face = Some(ModalFemFace::MinusZ);
    assert!(matches!(
        ModalFemSupport::from_launch_kind("base_fixed", face),
        Ok(ModalFemSupport::BaseFixed {
            face: ModalFemFace::MinusZ
        })
    ));
    assert!(matches!(
        ModalFemSupport::from_launch_kind("tabs", face),
        Ok(ModalFemSupport::Tabs { .. })
    ));
    assert!(matches!(
        ModalFemSupport::from_launch_kind("rails", None),
        Ok(ModalFemSupport::Rails)
    ));
    assert!(matches!(
        ModalFemSupport::from_launch_kind("free", face),
        Ok(ModalFemSupport::Free)
    ));
    for kind in ["base_fixed", "tabs", "fixed"] {
        assert_eq!(
            ModalFemSupport::from_launch_kind(kind, None).unwrap_err(),
            FaceRequired
        );
    }
    assert_eq!(
        ModalFemSupport::from_launch_kind("clamped", face).unwrap_err(),
        UnknownKind
    );
    // 語彙の全部が、面を添えれば拘束になる
    for kind in ModalFemSupport::LAUNCH_KINDS {
        assert!(
            ModalFemSupport::from_launch_kind(kind, face).is_ok(),
            "{kind}"
        );
    }
}

/// 入力の `skipped_panels` は、そのまま出力に出る。
#[test]
fn skipped_panels_pass_through() {
    let mut i = input(
        vec![panel(
            "P",
            ModalFemAxis::Z,
            [0.0; 3],
            [0.2, 0.2],
            0.002,
            0.3,
        )],
        ModalFemSupport::Free,
    );
    i.skipped_panels = vec![ModalFemSkipped {
        id: "TILTED".to_owned(),
        reason: "not axis-aligned".to_owned(),
    }];
    i.mode_count = Some(7);
    let out = solve(&i);
    assert_eq!(out.skipped_panels.len(), 1);
    assert_eq!(out.skipped_panels[0].id, "TILTED");
}

/// 平行な同じ 2 枚の片持ち板（z = ±0.1 m、x の負の面を固定）。
fn twin_plates() -> ModalFemInput {
    let p = |id: &str, z: f64| panel(id, ModalFemAxis::Z, [0.0, 0.0, z], [0.2, 0.2], 0.002, 0.3);
    input(
        vec![p("A", -0.1), p("B", 0.1)],
        ModalFemSupport::BaseFixed {
            face: ModalFemFace::MinusX,
        },
    )
}

/// 2 枚の中間（どちらにも距離 0.1 m）にある点質量は、2 枚に半分ずつ配られる。
/// 配置は面 z = 0 について対称で、2 枚は互いにつながらないので、同じ 2 枚の片持ち板のモードは重根の組（振動数が等しい）になる。
#[test]
fn a_mass_midway_between_two_plates_is_split_equally() {
    let mut i = twin_plates();
    i.lumped = vec![lumped("M", [0.0, 0.0, 0.0], 0.2)];
    let out = solve(&i);
    assert_eq!(out.attachments.len(), 2);
    for a in &out.attachments {
        assert_eq!(a.id, "M");
        assert!((a.share.value() - 0.5).abs() < 1e-12);
        assert!((a.distance.value() - 0.1).abs() < 1e-12);
        assert!(a.automatic);
    }
    let panels: Vec<&str> = out.attachments.iter().map(|a| a.panel.as_str()).collect();
    assert_eq!(panels, ["A", "B"]);
    let (f1, f2) = (hz(&out.modes[0]), hz(&out.modes[1]));
    assert!(rel(f2, f1) < 1e-6, "{f1} {f2}");
    // 質量は板 2 枚（各 2700 × 0.002 × 0.04 = 0.216 kg）と点質量
    assert!((out.total_mass.value() - (0.432 + 0.2)).abs() < 1e-9);
    // 1 枚に全部をつないだ場合（名前で指定）より、2 枚に分けたほうが最低の振動数は高い
    let mut one = i.clone();
    one.lumped[0].panel = Some("A".to_owned());
    assert!(hz(&solve(&one).modes[0]) < f1);
}

/// 片方に明らかに近い点質量は、近い 1 枚に全部が付く（距離の差が許容差より大きい）。
#[test]
fn a_mass_nearer_to_one_plate_goes_wholly_to_it() {
    let mut i = twin_plates();
    i.lumped = vec![lumped("M", [0.0, 0.0, -0.08], 0.2)];
    let out = solve(&i);
    assert_eq!(out.attachments.len(), 1);
    let a = &out.attachments[0];
    assert_eq!((a.panel.as_str(), a.share.value()), ("A", 1.0));
    assert!((a.distance.value() - 0.02).abs() < 1e-12);
    // 負荷のある板 A が最低、無負荷の板 B は別の（高い）振動数
    assert!(hz(&out.modes[1]) > hz(&out.modes[0]) * 1.1);
}

/// 許容差（`merge_tolerance`）の範囲の差なら同じ距離とみなして等分し、範囲の外なら近いほうだけ。
#[test]
fn the_tie_set_follows_the_merge_tolerance() {
    let mut i = twin_plates();
    i.lumped = vec![lumped("M", [0.0, 0.0, -0.0004], 0.2)];
    // 距離 0.0996 と 0.1004（差 0.8 mm）。許容差 1 mm（既定）の範囲に入る
    assert_eq!(solve(&i).attachments.len(), 2);
    i.merge_tolerance = Some(len(0.0005));
    assert_eq!(solve(&i).attachments.len(), 1);
}
