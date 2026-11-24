use super::*;
use sat_astro::math::Quat;

/// 一辺 2 の立方体（中心は原点、軸は座標軸）。
fn cube() -> OrientedBox {
    OrientedBox::from_size(Vec3::ZERO, Quat::IDENTITY, Vec3::new(2.0, 2.0, 2.0))
}

fn v(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3::new(x, y, z)
}

/// 箱を貫く線分は通り、箱の手前で終わる線分・横を通る線分は通らない。
#[test]
fn through_stopping_short_and_beside() {
    let b = cube();
    assert!(b.blocks_segment(v(-3.0, 0.0, 0.0), v(3.0, 0.0, 0.0), 0.0));
    assert!(b.blocks_segment(v(3.0, 0.5, -0.5), v(-3.0, -0.5, 0.5), 0.0));
    assert!(!b.blocks_segment(v(-3.0, 0.0, 0.0), v(-1.5, 0.0, 0.0), 0.0));
    assert!(!b.blocks_segment(v(-3.0, 2.5, 0.0), v(3.0, 2.5, 0.0), 0.0));
}

/// 始点か終点が箱の中にある線分は通る（箱の中から出る視線は遮られる）。
#[test]
fn an_endpoint_inside_blocks() {
    let b = cube();
    assert!(b.blocks_segment(v(0.0, 0.0, 0.0), v(5.0, 0.0, 0.0), 0.0));
    assert!(b.blocks_segment(v(5.0, 0.0, 0.0), v(0.5, 0.5, 0.5), 0.0));
}

/// 面や辺や頂点をかすめるだけの線分は通らない（区間の長さが 0）。面に沿って走る線分も通らない。
#[test]
fn grazing_the_surface_does_not_block() {
    let b = cube();
    assert!(!b.blocks_segment(v(-3.0, 1.0, 0.0), v(3.0, 1.0, 0.0), 0.0));
    assert!(!b.blocks_segment(v(-3.0, 1.0, 1.0), v(3.0, 1.0, 1.0), 0.0));
    // 辺（x = y = 1）に 1 点で触れるだけの線分
    assert!(!b.blocks_segment(v(-1.0, 3.0, 0.0), v(3.0, -1.0, 0.0), 0.0));
}

/// `shrink` は箱を縮める: 表面から 0.05 だけ中に入る線分は、縮めなければ通り、0.1 縮めれば通らない。
#[test]
fn shrink_lets_a_shallow_graze_pass() {
    let b = cube();
    let (p, q) = (v(-3.0, 0.95, 0.0), v(3.0, 0.95, 0.0));
    assert!(b.blocks_segment(p, q, 0.0));
    assert!(!b.blocks_segment(p, q, 0.1));
}

/// 回した箱（Z まわり 45°）: 対角線方向の線分は、回転前の軸に沿う位置では通らず、回した軸の方向では通る。
#[test]
fn a_turned_box_is_tested_in_its_own_frame() {
    let q45 = Quat::from_axis_angle(Vec3::Z, std::f64::consts::FRAC_PI_4);
    let b = OrientedBox::from_size(Vec3::ZERO, q45, v(2.0, 2.0, 2.0));
    // 回した立方体の頂点は x 軸上で ±√2 にある: x = 1.3 の線は頂点の近くを通る
    assert!(b.blocks_segment(v(1.3, -3.0, 0.0), v(1.3, 3.0, 0.0), 0.0));
    assert!(!b.blocks_segment(v(1.5, -3.0, 0.0), v(1.5, 3.0, 0.0), 0.0));
}
