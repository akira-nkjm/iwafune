//! 升に分けた板の外面の、影・覆いの升ごとの評価のテスト（#456）。板は 1 m × 1 m × 10 mm、外面 "P" は 0.01 m²（法線 +Z、α = 0.9、ε = 0.8）で、
//! 3 × 2 の升（u = x、v = y。x の境は −1/6・1/6）に分ける。升の節点は `P[i,j]`（節点の並びは通し番号 `2 i + j`）。箱の影・覆いは x に沿って変わる。
//! 影と覆いは多角形の厳密な面積なので、保存（升の和 = 面全体の値）は丸めの誤差（相対 1e-12）で成り立つ。

use super::shading_tests::{TOP, boxed, network, overhead};
use super::*;
use crate::thermal::network::ThermalNetwork;

const FLUX: f64 = 1361.0;
const CELLS: [u32; 2] = [3, 2];

fn compiled(net: &ThermalNetwork) -> Compiled {
    Compiled::new(net).unwrap()
}

fn sun(dir: Vec3) -> ThermalExposure {
    ThermalExposure::Fixed {
        sun_dir_body: Some(dir),
        nadir_body: None,
        altitude: Qty::from_raw(500.0e3),
        central: CentralBody::Earth,
    }
}

fn heat(net: &Compiled, exposure: &ThermalExposure) -> Vec<f64> {
    exposure_heat(net, &ThermalEnvironment::default(), exposure)
        .unwrap()
        .q
}

/// 箱 "A"（x ∈ [0, 0.5]、板の面から 0.1〜0.2 m の高さ）の影が板の半分に落ちる。真上の太陽では、影は x ∈ [0, 0.5] で、
/// 升 i = 0（x < −1/6）は全部日照・i = 1 は半分・i = 2 は全部日陰。直達日射は升ごとに `α · 1361 · (0.01/6) · 日照割合`
/// （0 W、1/2 倍、満額）。合計は分けない面の値 `0.5 · α · 1361 · 0.01` に等しい。影を切れば 6 つの升は同じ値。
#[test]
fn the_direct_sun_of_each_cell_follows_its_own_lit_fraction() {
    let net = compiled(&network(None, Some(CELLS)));
    let q = heat(&net, &sun(Vec3::Z));
    let full = 0.9 * FLUX * 0.01 / 6.0;
    let by_i = [1.0, 0.5, 0.0];
    for (k, q) in q.iter().enumerate() {
        assert!(
            (q - full * by_i[k / 2]).abs() < 1e-12 * full,
            "cell {k}: {q}"
        );
    }
    assert_eq!(q[4], 0.0);
    let whole = heat(&compiled(&network(None, None)), &sun(Vec3::Z))[0];
    assert!((q.iter().sum::<f64>() - whole).abs() < 1e-12 * whole);
    assert!((whole - 0.5 * 0.9 * FLUX * 0.01).abs() < 1e-9);
    let off = heat(&compiled(&network(Some(false), Some(CELLS))), &sun(Vec3::Z));
    assert!(
        off.iter()
            .all(|q| (q - 0.9 * FLUX * 0.01 / 6.0).abs() < 1e-12 * full)
    );
}

/// 手計算（傾けた太陽 d = (1, 0, 2)/√5。影は x ∈ [−0.1, 0.45]、升の日照は 1・0.2・0.15。保存の確かめと同じ値は `external_geometry` の試験）:
/// 升の直達日射は `α · 1361 · (0.01/6) · (n·d = 2/√5) · 日照割合`。合計は面全体（日照 0.45）の値に等しい。
#[test]
fn an_oblique_shadow_edge_gives_partial_cells_and_conserves_the_face_total() {
    let d = Vec3::new(1.0, 0.0, 2.0);
    let cos = 2.0 / 5.0_f64.sqrt();
    let q = heat(&compiled(&network(None, Some(CELLS))), &sun(d));
    let base = 0.9 * FLUX * 0.01 / 6.0 * cos;
    let by_i = [1.0, 0.2, 0.15];
    for (k, q) in q.iter().enumerate() {
        assert!(
            (q - base * by_i[k / 2]).abs() < 1e-12 * base,
            "cell {k}: {q}"
        );
    }
    let whole = heat(&compiled(&network(None, None)), &sun(d))[0];
    assert!((whole - 6.0 * base * 0.45).abs() < 1e-12 * whole);
    assert!((q.iter().sum::<f64>() - whole).abs() < 1e-12 * whole);
}

/// 板の x ∈ [0, 0.5] に箱 "R" が載って覆う（外面を持たない）。覆いの割合は升 i = 0・1・2 で 0・0.5・1。覆われた面積は宇宙へ放射せず
/// （`ε σ (0.01/6)(1 − 覆い)`）、アルベドと惑星の赤外も受けない。完全に覆われた升 i = 2 の入熱は 0（直達日射もアルベドも赤外も）。
/// 合計は、分けない面（半分が覆われる）の値と放射の係数の合計に等しい（保存）。影を切れば覆いも数えず 6 つの升は同じ。
#[test]
fn a_cover_over_half_the_plate_stops_the_exchange_of_those_cells_only() {
    let covered = |enabled: Option<bool>, cells: Option<[u32; 2]>| {
        let mut net = network(enabled, cells);
        let input = net.shadowing.as_mut().unwrap();
        input.boxes.pop();
        input
            .boxes
            .push(boxed("R", [0.25, 0.0, TOP + 0.05], [0.5, 1.0, 0.1]));
        compiled(&net)
    };
    let sigma = sat_astro::constants::SIGMA_SB;
    let (split, whole, off) = (
        covered(None, Some(CELLS)),
        covered(None, None),
        covered(Some(false), Some(CELLS)),
    );
    let by_i = [1.0, 0.5, 0.0];
    for (k, s) in split.surfaces.iter().enumerate() {
        assert!(
            (s.exposed - by_i[k / 2]).abs() < 1e-12,
            "cell {k}: {}",
            s.exposed
        );
        let want = 0.8 * sigma * 0.01 / 6.0 * by_i[k / 2];
        assert!((split.space[s.node] - want).abs() < 1e-12 * 0.8 * sigma * 0.01);
    }
    assert!((split.space.iter().sum::<f64>() - whole.space[0]).abs() < 1e-12 * whole.space[0]);
    assert!(off.surfaces.iter().all(|s| s.exposed == 1.0));
    // 真上の太陽と真上の地球: 完全に覆われた升の入熱は 0、合計は分けない面の値に等しい
    let exposure = overhead(Some(Vec3::Z));
    let q = heat(&split, &exposure);
    assert_eq!((q[4], q[5]), (0.0, 0.0));
    assert!(q[0] > q[2] && q[2] > q[4]);
    let total = heat(&whole, &exposure)[0];
    assert!((q.iter().sum::<f64>() - total).abs() < 1e-12 * total);
}

/// 板の縁の面（法線が面内の軸）は升に分けず、面積の割合で配る（升の長方形を持たない）。
/// 太陽が +X の縁の面に当たる入熱は、6 つの升で同じ値。
#[test]
fn a_surface_on_the_plate_edge_is_spread_by_area() {
    let mut net = network(None, Some(CELLS));
    net.surfaces[0].normal = Vec3::X;
    let net = compiled(&net);
    assert!(net.surfaces.iter().all(|s| s.cell.is_none()));
    let q = heat(&net, &sun(Vec3::X));
    let first = q[0];
    assert!(first > 0.0 && q.iter().all(|x| (x - first).abs() < 1e-12 * first));
}

/// 升に分けた板の定常のエネルギー収支: 影に入った升は冷え、日照の升は熱くなる（面内の伝導で均される）。
/// 吸収した入熱の合計は、宇宙への放射の合計 `Σ ε σ A (T⁴ − T_space⁴)` に等しい（升の間の伝導は対で打ち消す）。
#[test]
fn the_steady_balance_of_a_partly_shaded_plate_closes() {
    let net = compiled(&network(None, Some(CELLS)));
    let q = heat(&net, &sun(Vec3::Z));
    let sol = net.solve_steady(&q, 3.0, 300.0).unwrap();
    let t = &sol.temps;
    assert!(t[0] > t[2] && t[2] > t[4], "{t:?}");
    let emitted: f64 = (0..net.len())
        .map(|i| net.space[i] * (t[i].powi(4) - 3.0_f64.powi(4)))
        .sum();
    let absorbed: f64 = q.iter().sum();
    assert!(
        (emitted - absorbed).abs() < 1e-9 * absorbed,
        "{emitted} vs {absorbed}"
    );
}
