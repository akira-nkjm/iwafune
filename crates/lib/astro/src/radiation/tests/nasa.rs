//! NASA の例表（NSSDC データセット PT-11B の `radbelt.log` の AE8MIN の積分フラックス表）との照合。
//! 配布ファイル（`ae8min.asc`）がキャッシュにあるときだけ走る（無ければ読み飛ばしを表示する）。
//! 期待値は NASA の出力の数値（有効数字 4 桁、electrons/(cm² s)、B/B₀ = 1 の L 表と、L = 3.5・2.5・6.5 の B/B₀ 表）で、
//! 試験の側で調整した値ではない。

use super::*;
use crate::test_data::cached_bytes;

/// `(L, E [MeV], B/B₀, 期待値)`。出典: NASA NSSDC PT-11B のサンプル出力 `radbelt.log`（AE8MIN、積分フラックス）。
const AE8MIN_INTEGRAL: &[(f64, f64, f64, f64)] = &[
    // B/B₀ = 1 の L 表
    (1.5, 0.5, 1.0, 4.095e6),
    (1.5, 1.0, 1.0, 8.601e5),
    (1.5, 1.5, 1.0, 3.603e5),
    (1.5, 2.0, 1.0, 1.700e5),
    (1.5, 2.5, 1.0, 7.986e4),
    (1.5, 3.0, 1.0, 1.099e4),
    (2.5, 0.5, 1.0, 6.994e5),
    (2.5, 1.0, 1.0, 7.499e4),
    (2.5, 2.0, 1.0, 2.595e3),
    (3.5, 0.5, 1.0, 1.547e6),
    (3.5, 1.0, 1.0, 6.686e5),
    (3.5, 2.0, 1.0, 1.719e5),
    (4.5, 0.5, 1.0, 6.793e6),
    (4.5, 1.0, 1.0, 2.648e6),
    (4.5, 2.0, 1.0, 4.313e5),
    (5.5, 1.0, 1.0, 1.747e6),
    (7.5, 1.0, 1.0, 9.978e4),
    // L = 3.5 の B/B₀ 表（1 MeV と 0.5 MeV）
    (3.5, 1.0, 2.0, 5.261e5),
    (3.5, 1.0, 3.0, 4.139e5),
    (3.5, 1.0, 4.0, 3.338e5),
    (3.5, 1.0, 5.0, 2.737e5),
    (3.5, 1.0, 6.0, 2.245e5),
    (3.5, 1.0, 10.0, 1.446e5),
    (3.5, 1.0, 15.0, 9.686e4),
    (3.5, 1.0, 20.0, 6.957e4),
    (3.5, 1.0, 50.0, 1.189e4),
    (3.5, 0.5, 10.0, 3.344e5),
    (3.5, 0.5, 50.0, 2.751e4),
    // L = 2.5 の B/B₀ 表（0.5・1・3 MeV。E = 0.1 MeV と 0.05 MeV は E の補間を含む）
    (2.5, 0.5, 2.0, 4.491e5),
    (2.5, 1.0, 5.0, 2.942e3),
    (2.5, 0.5, 15.0, 4.758e1),
    (2.5, 0.1, 1.0, 3.249e7),
    (2.5, 0.05, 1.0, 5.037e7),
    (1.5, 0.1, 1.0, 7.449e7),
    (1.5, 0.05, 1.0, 1.002e8),
    // E = 6 MeV は節点（5.9998 MeV）の近く。L = 3・4 は節点の上
    (3.0, 6.0, 1.0, 1.998e2),
    (4.0, 6.0, 1.0, 1.048e1),
    (3.0, 3.0, 1.0, 1.398e4),
    (4.0, 0.5, 1.0, 3.293e6),
    // フラックス無し（NASA の表で 0.000E+00）
    (1.5, 6.0, 1.0, 0.0),
    (5.0, 6.0, 1.0, 0.0),
    (2.5, 0.5, 20.0, 0.0),
    (2.5, 0.5, 50.0, 0.0),
];

/// L が地図の節点の間にある点（AE-8 の L の節点は 6.0・6.5995・8.0・9.0・10.0 …）。B/B₀ = 1 では L の補間（log₁₀F の直線）だけが
/// 効き、4 桁で一致する。出典は同じ。
const AE8MIN_OFF_NODE_BB0_1: &[(f64, f64, f64, f64)] = &[
    (6.5, 0.5, 1.0, 3.202e6),
    (6.5, 1.0, 1.0, 5.414e5),
    (6.5, 1.5, 1.0, 1.385e5),
    (6.5, 2.0, 1.0, 4.644e4),
    (6.5, 3.0, 1.0, 4.433e3),
    (8.5, 0.5, 1.0, 6.978e4),
    (8.5, 1.0, 1.0, 4.661e3),
    (8.5, 2.0, 1.0, 1.540e2),
    (9.5, 0.5, 1.0, 1.373e3),
    (9.5, 1.0, 1.0, 6.118e1),
];

/// L = 6.5（節点 6.0 と 6.5995 の間）の B/B₀ 表。B/B₀ > 1 で `TRARA2` の節点の間の手順が効く点。NASA の手順を単精度で追うので、
/// 有効数字 4 桁の丸めの内で一致する（以前の log₁₀F を同じ B/B₀ で補間する近似では最大 0.19 % ずれていた）。
const AE8MIN_OFF_NODE_BB0: &[(f64, f64, f64, f64)] = &[
    (6.5, 0.5, 2.0, 2.566e6),
    (6.5, 1.0, 2.0, 4.339e5),
    (6.5, 2.0, 2.0, 3.722e4),
    (6.5, 3.0, 2.0, 3.552e3),
    (6.5, 0.5, 5.0, 1.478e6),
    (6.5, 1.0, 5.0, 2.499e5),
    (6.5, 2.0, 5.0, 2.143e4),
    (6.5, 3.0, 5.0, 2.045e3),
    (6.5, 0.5, 10.0, 8.855e5),
    (6.5, 1.0, 10.0, 1.497e5),
    (6.5, 2.0, 10.0, 1.284e4),
    (6.5, 3.0, 10.0, 1.226e3),
    (6.5, 0.5, 20.0, 5.360e5),
    (6.5, 1.0, 20.0, 9.063e4),
    (6.5, 2.0, 20.0, 7.773e3),
    (6.5, 3.0, 20.0, 7.417e2),
    (6.5, 0.5, 50.0, 2.810e5),
    (6.5, 1.0, 50.0, 4.751e4),
    (6.5, 2.0, 50.0, 4.073e3),
    (6.5, 3.0, 50.0, 3.885e2),
];

/// 値の組を地図から引いた値と比べ、最大の相対誤差を返す。`tol` を超えたら失敗。期待値 0 は 0 でなければならない。
fn max_relative_error(map: &TrappedMap, table: &[(f64, f64, f64, f64)], tol: f64) -> f64 {
    let mut worst = 0.0f64;
    for &(l, e, bb0, want) in table {
        let got = map.integral_flux(l, bb0, e);
        if want == 0.0 {
            assert_eq!(got, 0.0, "L={l} E={e} B/B0={bb0}");
            continue;
        }
        let rel = (got - want).abs() / want;
        worst = worst.max(rel);
        assert!(
            rel < tol,
            "L={l} E={e} B/B0={bb0}: got {got:.4e}, NASA {want:.4e} (rel {rel:.2e})"
        );
    }
    worst
}

#[test]
fn ae8min_reproduces_the_nasa_sample_tables() {
    let Some(bytes) = cached_bytes("ae8min.asc") else {
        return;
    };
    let map = TrappedMap::from_ascii_bytes(&bytes).expect("ae8min");
    assert_eq!((map.header().map_type, map.header().epoch_year), (8, 1964));
    // NASA の表は有効数字 4 桁なので、相対 5e-4（4 桁目の丸め）の内で一致する
    let on_node = max_relative_error(&map, AE8MIN_INTEGRAL, 5.0e-4);
    let bb0_1 = max_relative_error(&map, AE8MIN_OFF_NODE_BB0_1, 5.0e-4);
    let off_node = max_relative_error(&map, AE8MIN_OFF_NODE_BB0, 5.0e-4);
    eprintln!(
        "AE8MIN vs NASA radbelt.log max relative error: {on_node:.2e} on nodes ({} values), {bb0_1:.2e} off-node L at B/B0=1 ({}), {off_node:.2e} off-node L at B/B0>1 ({})",
        AE8MIN_INTEGRAL.len(),
        AE8MIN_OFF_NODE_BB0_1.len(),
        AE8MIN_OFF_NODE_BB0.len()
    );
}

/// 4 つの実ファイルの見出し（種類・元期・刻みの数）とエネルギーの範囲。
#[test]
fn the_four_real_maps_parse() {
    let want = [
        ("ap8min.asc", 2, 1964, 0.1, 400.0),
        ("ap8max.asc", 2, 1970, 0.1, 400.0),
        ("ae8min.asc", 8, 1964, 0.04, 7.0),
        ("ae8max.asc", 7, 1970, 0.04, 7.0),
    ];
    for (file, map_type, year, e_min, e_max) in want {
        let Some(bytes) = cached_bytes(file) else {
            continue;
        };
        let map = TrappedMap::from_ascii_bytes(&bytes).unwrap_or_else(|e| panic!("{file}: {e}"));
        let h = map.header();
        assert_eq!(
            (h.map_type, h.epoch_year, h.steps_per_decade),
            (map_type, year, 4),
            "{file}"
        );
        let energies = map.energies_mev();
        assert!((energies[0] - e_min).abs() < 1e-3, "{file}: {energies:?}");
        let top = *energies.last().expect("energies");
        assert!((top - e_max).abs() < 1e-3, "{file}: {top}");
    }
}

/// NASA の例表の陽子の表は AP8MAC（圧縮した古い地図）の値なので、AP8MAX・AP8MIN との照合は**ゆるい確認だけ**。
/// L = 1.5〜3 の B/B₀ = 1 では 1 % 以内で一致する（4 以上の L は古い地図と新しい地図が違う）。出典は同じ（`radbelt.log`）。
const AP8MAC_INTEGRAL: &[(f64, f64, f64)] = &[
    // (L, E [MeV], 積分フラックス [cm⁻² s⁻¹]) at B/B₀ = 1
    (1.5, 0.1, 5.328e5),
    (1.5, 1.0, 5.025e5),
    (1.5, 6.0, 3.338e5),
    (2.0, 0.1, 1.712e7),
    (2.0, 1.0, 8.467e6),
    (2.0, 6.0, 9.647e5),
    (2.5, 0.1, 1.420e8),
    (2.5, 1.0, 3.010e7),
    (2.5, 6.0, 2.751e5),
    (3.0, 0.1, 3.293e8),
    (3.0, 1.0, 3.803e7),
    (3.0, 6.0, 2.738e4),
];

#[test]
fn ap8_maps_agree_loosely_with_the_nasa_proton_sample_at_low_l() {
    for file in ["ap8max.asc", "ap8min.asc"] {
        let Some(bytes) = cached_bytes(file) else {
            continue;
        };
        let map = TrappedMap::from_ascii_bytes(&bytes).expect(file);
        let mut worst = 0.0f64;
        for &(l, e, want) in AP8MAC_INTEGRAL {
            let got = map.integral_flux(l, 1.0, e);
            let rel = (got - want).abs() / want;
            worst = worst.max(rel);
            assert!(
                rel < 1.0e-2,
                "{file} L={l} E={e}: got {got:.4e}, NASA (AP8MAC) {want:.4e}"
            );
        }
        eprintln!(
            "{file} vs the AP8MAC sample at B/B0=1, L<=3: max relative difference {worst:.2e}"
        );
    }
}

/// 公開の端から端までの点（NASA の `radbelt` パッケージの README の例）: 経度 −45°・緯度 −30°・高度 500 km、2021-03-01、
/// 太陽活動極大（AP8MAX）の陽子 E > 20 MeV、場はその日付の IGRF → 2642.50 cm⁻² s⁻¹。
/// 引き方（`TRARA1`・`TRARA2`）は NASA の手順どおりなので、残る差は (L, B/B₀) の定義と、位置（楕円体高度）の取り方から来る。
/// 2 つの定義で値を表示する。**どちらかに合わせて期待値を決めることはしない**: 検査は、値が同じ桁（2 倍以内）にあることだけ。
#[test]
fn nasa_readme_point() {
    use crate::geomag::IgrfTable;
    use crate::time::Epoch;
    let (Some(igrf), Some(map)) = (cached_bytes("igrf14coeffs.txt"), cached_bytes("ap8max.asc"))
    else {
        return;
    };
    let table = IgrfTable::from_bytes(&igrf).expect("igrf");
    let map = TrappedMap::from_ascii_bytes(&map).expect("ap8max");
    let epoch = Epoch::from_calendar(2021, 3, 1, 0, 0, 0.0);
    let snap = table.model_at(epoch).expect("model").at(epoch);
    let r = crate::earth::geodetic_to_ecef((-30f64).to_radians(), (-45f64).to_radians(), 500e3);
    for convention in [
        BeltConvention::Literature,
        BeltConvention::ModelDipoleLineMinimum,
    ] {
        let p = belt_position_with(&snap, r, convention).expect("position");
        let flux = map.integral_flux(p.l, p.b_over_b0, 20.0);
        eprintln!(
            "README point {convention:?}: L = {:.4}, B = {:.1} nT, B0 = {:.1} nT, B/B0 = {:.4}, flux(>20 MeV) = {flux:.1} (NASA 2642.50, {:+.1} %)",
            p.l,
            p.b_t * 1e9,
            p.b0_t * 1e9,
            p.b_over_b0,
            (flux / 2642.5 - 1.0) * 100.0
        );
        // README の値は IGRF 自身の双極子モーメントで L と B₀ を求める道具のもの。同じ定義の比較用では 1 % 以内で一致する。
        // 文献の定義は L の定義が違うので値が離れる（L が 0.01 違うと約 15 %）。こちらは桁だけを確かめる。
        let tolerance = match convention {
            BeltConvention::ModelDipoleLineMinimum => 0.01,
            BeltConvention::Literature => 1.0,
        };
        assert!(
            (flux / 2642.5 - 1.0).abs() < tolerance,
            "{convention:?}: {flux}"
        );
    }
}
