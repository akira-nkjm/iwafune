//! 不変性: モデル全体の平行移動・軸の入れ替え・反転・機軸まわりの 90° 回転で、固有振動数は変わらない。

use super::*;

/// 底面（−z）を固定した 0.1 × 0.2 × 0.3 m の箱に、非対称な集中質量を載せたもの。要素 50 mm。
fn model() -> ModalFemInput {
    let mut i = input(
        box_panels(0.1, 0.2, 0.3, 0.0015),
        clamp_face(ModalFemFace::MinusZ),
    );
    i.mesh_size = Some(len(0.05));
    i.lumped = vec![
        lumped("A", [0.02, 0.05, 0.10], 0.4),
        lumped("B", [-0.03, -0.06, -0.05], 0.7),
        lumped("C", [0.0, 0.08, 0.12], 0.2),
    ];
    i
}

fn same_frequencies(a: &ModalFemOutput, b: &ModalFemOutput, tol: f64) {
    assert_eq!(a.modes.len(), b.modes.len());
    for (x, y) in a.modes.iter().zip(&b.modes) {
        assert!(rel(hz(x), hz(y)) < tol, "{} vs {}", hz(x), hz(y));
    }
}

/// 平行移動（許容差 1e-7: 座標が変わると丸めが変わる分と反復の収束の許容値）。
#[test]
fn translation_invariance() {
    let base = solve(&model());
    let moved = solve(&remap(&model(), [0, 1, 2], [1.0; 3], [0.3, -0.2, 0.7]));
    same_frequencies(&base, &moved, 1e-7);
}

/// 軸の入れ替え（x←z, y←x, z←y）と、反転を含む入れ替え。機軸と固定面も一緒に動かす。許容差 1e-7（節点の並び順が変わる分の丸め）。
#[test]
fn axis_permutation_invariance() {
    let base = solve(&model());
    let permuted = remap(&model(), [2, 0, 1], [1.0; 3], [0.0; 3]);
    assert_eq!(permuted.launch_axis, ModalFemAxis::X);
    same_frequencies(&base, &solve(&permuted), 1e-7);
    let mirrored = remap(&model(), [1, 2, 0], [-1.0, 1.0, -1.0], [0.1, 0.0, 0.2]);
    same_frequencies(&base, &solve(&mirrored), 1e-7);
}

/// 機軸（z）まわりに 90° 回しても振動数は同じで、有効質量の割合は x と y が入れ替わるだけ（z はそのまま）。
/// 回転: 新しい x = −古い y、新しい y = 古い x。許容差 1e-7（振動数）、1e-5（割合: 重根でなければモードの形は一意）。
#[test]
fn rotation_about_the_launch_axis() {
    let base = solve(&model());
    let rotated = solve(&remap(&model(), [1, 0, 2], [-1.0, 1.0, 1.0], [0.0; 3]));
    same_frequencies(&base, &rotated, 1e-7);
    assert_ne!(base.lowest_lateral.map(Qty::value), None);
    for (a, b) in base.modes.iter().zip(&rotated.modes) {
        let (fa, fb) = (a.effective_mass_fraction, b.effective_mass_fraction);
        assert!((fa[0].value() - fb[1].value()).abs() < 1e-5, "x -> y");
        assert!((fa[1].value() - fb[0].value()).abs() < 1e-5, "y -> x");
        assert!((fa[2].value() - fb[2].value()).abs() < 1e-5, "z unchanged");
        assert_eq!(a.class, b.class);
    }
}
