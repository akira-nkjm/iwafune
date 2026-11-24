//! 回転座標系の軌跡のテスト。固定点は導ける値（太陽–地球の直線上、距離比）、ハローは表の周期・時刻の写像。

use super::*;
use crate::orbit::Trajectory;

fn epoch() -> Epoch {
    Epoch::parse("2030-01-01T00:00:00Z").expect("epoch")
}

fn sun_earth(point: LibrationPoint, halo: Option<HaloSpec>) -> SynodicSpec {
    SynodicSpec::new(epoch(), CentralBody::Sun, CentralBody::Earth, point, halo)
}

fn halo_120k(branch: Branch) -> HaloSpec {
    HaloSpec {
        amplitude_z_m: 120_000e3,
        branch,
    }
}

/// 太陽–地球 L2 の固定点は、地球から見て太陽と反対の向きの直線上にあり、距離は `γ·d`（`d` は瞬間の距離、`γ` は CR3BP の点の距離）。
/// 数か月にわたって確かめる。
#[test]
fn fixed_l2_stays_on_the_sun_earth_line_beyond_the_earth() {
    let spec = sun_earth(LibrationPoint::L2, None);
    let gamma = spec
        .cr3bp()
        .expect("cr")
        .gamma(LibrationPoint::L2)
        .expect("g");
    let times: Vec<f64> = (0..8).map(|i| f64::from(i) * 45.0 * 86_400.0).collect();
    let states = spec.states_at(&times).expect("states");
    for (t, s) in times.iter().zip(&states) {
        let e = epoch().plus_seconds(*t);
        let to_sun = CentralBody::Earth.relative_position_m(crate::ephem::Body::Sun, e);
        let away = -to_sun.normalized().expect("dir");
        // 方向: 太陽と反対向き
        assert!(
            s.r_eci.normalized().expect("n").dot(away) > 1.0 - 1e-9,
            "t={t}"
        );
        // 距離: γ·d（ここで d = 地球–太陽の瞬間の距離）
        let expected = gamma * to_sun.norm();
        assert!((s.r_eci.norm() / expected - 1.0).abs() < 1e-9, "t={t}");
        // 約 1.5 百万 km
        assert!((1.4e9..1.6e9).contains(&s.r_eci.norm()));
    }
}

/// L1 は太陽側（太陽の向きの直線上）。
#[test]
fn fixed_l1_is_on_the_sun_side() {
    let spec = sun_earth(LibrationPoint::L1, None);
    let s = spec.states_at(&[0.0]).expect("s")[0];
    let to_sun = CentralBody::Earth
        .relative_position_m(crate::ephem::Body::Sun, epoch())
        .normalized()
        .expect("dir");
    assert!(s.r_eci.normalized().expect("n").dot(to_sun) > 1.0 - 1e-9);
}

/// 固定点の速度は、点が副天体とともに主天体のまわりを回る速度（`ω × r` に距離の変化分）。大きさは
/// 地球の公転速度に `γ` ぶんの距離の比を掛けたものの近傍（地球に対する相対速度は ~ n·r = 0.03 km/s × ...）。
/// 手計算: 地球から 1.5e9 m の点が角速度 n = 1.99e-7 rad/s で回る相対速度は 約 300 m/s。
#[test]
fn fixed_point_velocity_is_the_rotation_of_the_line() {
    let spec = sun_earth(LibrationPoint::L2, None);
    let s = spec.states_at(&[0.0]).expect("s")[0];
    let n = spec.mean_motion_rad_s().expect("n");
    let hand = n * s.r_eci.norm();
    assert!(
        (s.v_eci.norm() / hand - 1.0).abs() < 0.05,
        "{} {}",
        s.v_eci.norm(),
        hand
    );
    // 速度は位置に直交（回転）に近い
    assert!(
        s.v_eci
            .normalized()
            .expect("v")
            .dot(s.r_eci.normalized().expect("r"))
            .abs()
            < 0.05
    );
}

/// 繰り返しの長さ: 固定点は公転周期（約 365 日）、ハローは約半年。
#[test]
fn periods() {
    let fixed = sun_earth(LibrationPoint::L2, None).period_s() / 86_400.0;
    assert!((fixed - 365.25).abs() < 1.0, "{fixed}");
    let halo = sun_earth(LibrationPoint::L2, Some(halo_120k(Branch::North))).period_s() / 86_400.0;
    assert!((170.0..190.0).contains(&halo), "{halo}");
}

/// ハロー軌道を回転系に戻すと CR3BP の軌道になる: ヤコビ定数が一定（補正した軌道の値）で、軌跡は点から 10⁵ km 級に収まる。
/// 暦の地球の離心率で距離が伸び縮みしても、脈動座標系に戻せば同じ軌道（暦の位置と速度を使った写像の逆）。
/// 許容 1e-6 はエルミート補間（400 区間、位置と速度は表の値）の速度の誤差と、暦の速度の中心差分（±60 s）の誤差の合計の見積もり。
#[test]
fn halo_maps_back_to_a_cr3bp_orbit_with_constant_jacobi() {
    let spec = sun_earth(LibrationPoint::L2, Some(halo_120k(Branch::North)));
    let cr = spec.cr3bp().expect("cr");
    let jacobi0 = spec.halo_table().expect("table").jacobi;
    let times: Vec<f64> = (0..=60).map(|i| f64::from(i) * 7.3 * 86_400.0).collect();
    let states = spec.states_at(&times).expect("states");
    let gamma = cr.gamma(LibrationPoint::L2).expect("g");
    let x_point = cr.libration_point(LibrationPoint::L2).expect("p")[0];
    for (t, s) in times.iter().zip(&states) {
        let sec = CentralBody::Earth
            .state_about_parent(epoch().plus_seconds(*t))
            .expect("sec");
        let primary_centred = OrbitState {
            r_eci: s.r_eci + sec.r_eci,
            v_eci: s.v_eci + sec.v_eci,
        };
        let x = cr
            .inertial_to_rotating(&sec, &primary_centred)
            .expect("rot");
        assert!(
            (cr.jacobi(&x) - jacobi0).abs() < 1e-6,
            "t={t}: {} {jacobi0}",
            cr.jacobi(&x)
        );
        // 点からの距離は 軌道の大きさ（x 方向 ~ 1.5e5〜2.5e5 km を γ で割った量の数倍以内）
        assert!(((x[0] - x_point) / gamma).abs() < 0.5, "{t}");
    }
}

/// 北と南は軌道面に対する鏡像: 元期の位置の軌道面に垂直な成分が逆符号で、面内の成分は同じ。
#[test]
fn north_and_south_are_mirror_images_about_the_orbital_plane() {
    let n = sun_earth(LibrationPoint::L2, Some(halo_120k(Branch::North)));
    let s = sun_earth(LibrationPoint::L2, Some(halo_120k(Branch::South)));
    let sec = CentralBody::Earth.state_about_parent(epoch()).expect("sec");
    let ez = sec.r_eci.cross(sec.v_eci).normalized().expect("ez");
    let (a, b) = (
        n.states_at(&[0.0]).expect("n")[0],
        s.states_at(&[0.0]).expect("s")[0],
    );
    assert!((a.r_eci.dot(ez) + b.r_eci.dot(ez)).abs() < 1e-6 * a.r_eci.norm());
    assert!(a.r_eci.dot(ez) > 0.0, "north is on the +z side at epoch");
    assert!(
        ((a.r_eci - ez * a.r_eci.dot(ez)) - (b.r_eci - ez * b.r_eci.dot(ez))).norm()
            < 1e-6 * a.r_eci.norm()
    );
}

/// 元期の付け替え: 同じ時刻の状態が一致する（固定点・ハロー）。ハローは付け替え後の元期が表の始めでなくても同じ軌跡。
#[test]
fn rebasing_keeps_the_same_orbit() {
    for halo in [None, Some(halo_120k(Branch::North))] {
        let orig = Trajectory::Synodic(sun_earth(LibrationPoint::L2, halo));
        let start = epoch().plus_seconds(37.5 * 86_400.0);
        let rebased = orig.starting_at(start).expect("rebased");
        let (a, b) = (
            orig.states_at(&[(100.0 + 37.5) * 86_400.0]).expect("a")[0],
            rebased.states_at(&[100.0 * 86_400.0]).expect("b")[0],
        );
        assert!(
            (a.r_eci - b.r_eci).norm() < 1e3,
            "{}",
            (a.r_eci - b.r_eci).norm()
        );
        assert!((a.v_eci - b.v_eci).norm() < 1e-3);
        assert_eq!(rebased.epoch(), start);
    }
}

/// 検査: 親でない組、振幅つきの L3・L4、負の振幅。
#[test]
fn validation_rejects_bad_specs() {
    let bad_pair = SynodicSpec::new(
        epoch(),
        CentralBody::Earth,
        CentralBody::Mars,
        LibrationPoint::L2,
        None,
    );
    assert!(bad_pair.validate().is_err());
    assert!(
        sun_earth(LibrationPoint::L3, Some(halo_120k(Branch::North)))
            .validate()
            .is_err()
    );
    assert!(
        sun_earth(LibrationPoint::L4, Some(halo_120k(Branch::North)))
            .validate()
            .is_err()
    );
    let neg = HaloSpec {
        amplitude_z_m: -1.0,
        branch: Branch::North,
    };
    assert!(sun_earth(LibrationPoint::L2, Some(neg)).validate().is_err());
    assert!(sun_earth(LibrationPoint::L4, None).validate().is_ok());
}

/// 地球–月: 副天体は月、日陰を作る追加の天体は地球。太陽–地球は追加なし。
#[test]
fn central_and_shadow_casters() {
    let em = SynodicSpec::new(
        epoch(),
        CentralBody::Earth,
        CentralBody::Moon,
        LibrationPoint::L2,
        None,
    );
    let t = Trajectory::Synodic(em);
    assert_eq!(t.central(), CentralBody::Moon);
    assert_eq!(t.extra_shadow_caster(), Some(CentralBody::Earth));
    let se = Trajectory::Synodic(sun_earth(LibrationPoint::L2, None));
    assert_eq!(se.central(), CentralBody::Earth);
    assert_eq!(se.extra_shadow_caster(), None);
    assert_eq!(se.altitude_m(), None);
    assert!(se.elements().is_none());
}

/// JSON の往復（`source = "libration"`）。
#[test]
fn json_roundtrip() {
    let t = Trajectory::Synodic(sun_earth(
        LibrationPoint::L2,
        Some(halo_120k(Branch::South)),
    ));
    let json = serde_json::to_value(&t).expect("ser");
    assert_eq!(json["source"], "libration");
    assert_eq!(json["point"], "l2");
    assert_eq!(json["halo"]["branch"], "south");
    let back: Trajectory = serde_json::from_value(json.clone()).expect("de");
    assert_eq!(serde_json::to_value(&back).expect("ser"), json);
}
