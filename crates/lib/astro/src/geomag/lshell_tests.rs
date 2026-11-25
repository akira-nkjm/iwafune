//! McIlwain の L・B/B₀ のテスト（中心双極子の解析解、摂動のある場、L なしの場合、所要時間）。

use super::model_tests::{A, at, centred_dipole, epoch, model, term};
use super::*;
use crate::earth::geodetic_to_ecef;
use crate::ode::Dop853;

/// 中心双極子での L の許容誤差（積分の精度: 磁力線の追跡 rtol 1e-10、求積は節点 32 で 1e-9 以下、F の二分法 1e-13）。
const L_TOL: f64 = 1e-7;

/// 中心双極子では、磁力線上のどの点でも `L = r_eq/a`、`B/B₀ = √(4 − 3cos²λ)/cos⁶λ`。
#[test]
fn centred_dipole_l_and_b_ratio_are_analytic() {
    let snap = centred_dipole().at(epoch());
    let mut worst: f64 = 0.0;
    for (l_true, lats) in [
        (1.1, [0.0, 3.0, 10.0, 17.0]),
        (2.0, [0.0, 20.0, 35.0, 44.0]),
        (6.6, [1.0, 25.0, 50.0, 60.0]),
        (10.0, [0.0, 30.0, 55.0, 70.0]),
    ] {
        for lat_deg in lats {
            let lat = f64::to_radians(lat_deg);
            let r = l_true * A * lat.cos().powi(2);
            let got = snap
                .l_shell(at(r, lat, 0.7), &LOptions::default())
                .expect("closed line");
            let err = (got.l / l_true - 1.0).abs();
            worst = worst.max(err);
            assert!(
                err < L_TOL,
                "L={l_true} lat={lat_deg}: {} err {err:e}",
                got.l
            );
            let c2 = lat.cos().powi(2);
            let ratio = (4.0 - 3.0 * c2).sqrt() / c2.powi(3);
            let rerr = (got.b_over_b0 / ratio - 1.0).abs();
            assert!(
                rerr < 1e-8,
                "B/B0 L={l_true} lat={lat_deg}: {} vs {ratio} ({rerr:e})",
                got.b_over_b0
            );
        }
    }
    eprintln!("worst relative error of L on the centred dipole: {worst:e}");
}

/// 傾いた・ずれた中心双極子でも同じ（軸の向きに依らない）。
#[test]
fn tilted_dipole_l_is_analytic_in_its_own_axis() {
    let m = GaussModel::tilted_dipole(epoch(), A).expect("dipole");
    let snap = m.at(epoch());
    let axis = -super::dipole_axis_ecef(); // 北磁極側
    let e1 = axis.any_orthogonal().normalized().expect("unit");
    for lat_deg in [5.0_f64, 30.0, 60.0] {
        let lat = lat_deg.to_radians();
        let dir = e1 * lat.cos() + axis * lat.sin();
        let r = 3.0 * A * lat.cos().powi(2);
        let got = snap
            .l_shell(dir * r, &LOptions::default())
            .expect("closed line");
        assert!((got.l / 3.0 - 1.0).abs() < L_TOL, "{lat_deg}: {}", got.l);
    }
}

/// M の取り方: モデル自身の M と同じ値を固定で与えれば同じ L。固定値を 1 % 大きくすると、`L³ = M F(I³B/M)/B` で
/// `d ln(L³)/d ln M = 1 − X F′/F ∈ (0, 1]` なので、L は大きくなり、その比は 1.01^(1/3) 未満。
#[test]
fn fixed_moment_scales_l() {
    let snap = centred_dipole().at(epoch());
    let lat = 0.5_f64;
    let p = at(4.0 * A * lat.cos().powi(2), lat, 0.0);
    let base = snap.l_shell(p, &LOptions::default()).expect("L");
    let same = snap
        .l_shell(
            p,
            &LOptions {
                moment: MomentConvention::Fixed { b0_t: 30_000e-9 },
                ..LOptions::default()
            },
        )
        .expect("L");
    assert!((same.l - base.l).abs() < 1e-12);
    assert!((base.l / 4.0 - 1.0).abs() < L_TOL);
    let bigger = snap
        .l_shell(
            p,
            &LOptions {
                moment: MomentConvention::Fixed { b0_t: 30_300e-9 },
                ..LOptions::default()
            },
        )
        .expect("L");
    let ratio = bigger.l / base.l;
    assert!(ratio > 1.0 && ratio < 1.01_f64.cbrt(), "{ratio}");
}

/// 摂動のある場（合成の四重極などを足した場）。`scale` で非双極子の項の強さを変える。
fn perturbed(scale: f64) -> GaussModel {
    model(vec![
        term(1, 0, -30_000.0, 0.0),
        term(1, 1, -1_500.0, 4_800.0),
        term(2, 0, -2_000.0 * scale, 0.0),
        term(2, 1, 3_000.0 * scale, -1_500.0 * scale),
        term(2, 2, 1_700.0 * scale, -400.0 * scale),
        term(3, 1, -2_100.0 * scale, 300.0 * scale),
    ])
}

/// 点 `p0` を通る磁力線上の別の点（`sigma` の向きに弧長 `times` [m] 進んだ点）。
fn line_points(snap: &FieldSnapshot, p0: Vec3, sigma: f64, times: &[f64]) -> Vec<Vec3> {
    Dop853 {
        rtol: 1e-11,
        atol: 1e-6,
        ..Dop853::default()
    }
    .integrate_at(
        |_t, p: &[f64; 3]| {
            let b = snap.field_ecef_t(Vec3::new(p[0], p[1], p[2]));
            let n = b.norm();
            [sigma * b.x / n, sigma * b.y / n, sigma * b.z / n]
        },
        0.0,
        [p0.x, p0.y, p0.z],
        times,
    )
    .expect("trace")
    .into_iter()
    .map(|p| Vec3::new(p[0], p[1], p[2]))
    .collect()
}

/// 摂動のある場での L と I の不変性。
///
/// - 同じ磁力線の上で |B| が同じ 2 点（P と共役のミラー点）からは、I も L も一致する（積分の精度で）。
/// - 同じ磁力線の別の点では Bm が違うので、非双極子の場では L は厳密には一定でない（L が磁力線ごとの定数になるのは
///   双極子の場だけ）。ずれは小さく（ここでは非双極子の項が双極子の 7 % ほどで相対 2e-3。1e-2 未満）、非双極子の項を半分にすると 0.6 倍未満に減る（線形なら 0.5）。
#[test]
fn l_along_a_non_dipole_field_line() {
    let opts = LOptions::default();
    let p0 = geodetic_to_ecef(0.5, 0.4, 1.5e6);
    let spread = |scale: f64| {
        let snap = perturbed(scale).at(epoch());
        let base = snap.l_shell(p0, &opts).expect("L");
        // 共役のミラー点
        let conj = snap
            .l_shell(base.conjugate_ecef_m, &opts)
            .expect("L at conjugate");
        assert!(
            (conj.l / base.l - 1.0).abs() < 1e-7,
            "{} {}",
            conj.l,
            base.l
        );
        assert!(
            (conj.i_m / base.i_m - 1.0).abs() < 1e-6,
            "{} {}",
            conj.i_m,
            base.i_m
        );
        assert!((conj.b_t / base.b_t - 1.0).abs() < 1e-8);
        let mut worst: f64 = 0.0;
        for sigma in [1.0, -1.0] {
            for q in line_points(&snap, p0, sigma, &[3.0e5, 1.2e6, 2.5e6, 4.0e6]) {
                let got = snap.l_shell(q, &opts).expect("L on the same line");
                worst = worst.max((got.l / base.l - 1.0).abs());
                // 同じ磁力線の |B| の最小は同じ
                assert!((got.b0_t / base.b0_t - 1.0).abs() < 1e-6);
            }
        }
        worst
    };
    let (full, half) = (spread(1.0), spread(0.5));
    eprintln!("L spread along a perturbed line: {full:e} (full), {half:e} (half the perturbation)");
    assert!(full < 1e-2 && full > 1e-6, "{full:e}");
    assert!(half < 0.6 * full, "{half:e} vs {full:e}");
}

/// 閉じない磁力線・上限より遠い・場が 0 の点は、型つきの「L なし」を返す。
#[test]
fn no_l_is_a_typed_result() {
    let snap = centred_dipole().at(epoch());
    let opts = LOptions::default();
    // 高緯度（L = 1.2/cos²85° ≈ 158 > 上限 12）
    let lat = 85.0_f64.to_radians();
    let lat75 = 75.0_f64.to_radians();
    assert_eq!(
        snap.l_shell(at(1.2 * A, lat, 0.0), &opts).unwrap_err(),
        NoL::OpenFieldLine
    );
    // 上限の外の点
    assert_eq!(
        snap.l_shell(Vec3::new(20.0 * A, 0.0, 0.0), &opts)
            .unwrap_err(),
        NoL::OpenFieldLine
    );
    assert_eq!(snap.l_shell(Vec3::ZERO, &opts).unwrap_err(), NoL::NoField);
    assert_eq!(
        snap.l_shell(Vec3::new(f64::NAN, 0.0, 0.0), &opts)
            .unwrap_err(),
        NoL::NoField
    );
    // 上限を大きくすれば同じ点で L が出る（L ≈ 18 で 1e-8 の精度。L が 40 を超えると求積の節点の分解能が足りず
    // 誤差が 4e-7（L = 40）、7e-5（L = 158）と増える。AP-8・AE-8 の範囲（L ≲ 12）より十分外）。
    let big = LOptions {
        max_radius_ref: 400.0,
        ..opts
    };
    assert_eq!(
        snap.l_shell(at(1.2 * A, lat75, 0.0), &opts).unwrap_err(),
        NoL::OpenFieldLine
    );
    let ok = snap.l_shell(at(1.2 * A, lat75, 0.0), &big).expect("L");
    let want = 1.2 / lat75.cos().powi(2);
    assert!((ok.l / want - 1.0).abs() < 1e-8, "{} {want}", ok.l);
    // 不正な固定モーメント
    let bad = LOptions {
        moment: MomentConvention::Fixed { b0_t: 0.0 },
        ..opts
    };
    assert_eq!(
        snap.l_shell(Vec3::new(2.0 * A, 0.0, 0.0), &bad)
            .unwrap_err(),
        NoL::NoMoment
    );
}

/// 磁気赤道上の点（I = 0）でも L = r/a。
#[test]
fn point_on_the_magnetic_equator() {
    let snap = centred_dipole().at(epoch());
    let got = snap
        .l_shell(Vec3::new(0.0, 1.5 * A, 0.0), &LOptions::default())
        .expect("L");
    assert!((got.l / 1.5 - 1.0).abs() < 1e-9, "{}", got.l);
    assert!((got.b_over_b0 - 1.0).abs() < 1e-9);
    assert!(got.i_m.abs() < 1.0);
}

/// 1 回の L の所要時間（`cargo test -p sat-astro --release l_shell_timing -- --ignored --nocapture`）。
#[test]
#[ignore = "timing report only"]
fn l_shell_timing() {
    let m = model(vec![
        term(1, 0, -30_000.0, 0.0),
        term(1, 1, -1_500.0, 4_800.0),
        term(2, 0, -2_000.0, 0.0),
        term(2, 1, 3_000.0, -1_500.0),
        term(2, 2, 1_700.0, -400.0),
        term(3, 1, -2_100.0, 300.0),
    ]);
    let high = {
        let mut terms = Vec::new();
        for n in 1..=10u8 {
            for mm in 0..=n {
                let s = 30_000.0 / f64::from(n).powi(3);
                terms.push(term(
                    n,
                    mm,
                    -s * f64::from(mm + 1).recip(),
                    if mm == 0 { 0.0 } else { s * 0.3 },
                ));
            }
        }
        model(terms)
    };
    for (label, mdl) in [("degree 3", m), ("degree 10", high)] {
        let snap = mdl.at(epoch());
        let opts = LOptions::default();
        let pts: Vec<Vec3> = (0..200)
            .map(|i| {
                let lat = f64::from(i) * 0.031 - 3.0;
                let lon = f64::from(i) * 0.37;
                geodetic_to_ecef(lat.sin() * 1.2, lon, 550e3 + 20e3 * f64::from(i % 5))
            })
            .collect();
        let t0 = std::time::Instant::now();
        let mut ok = 0;
        for p in &pts {
            match snap.l_shell(*p, &opts) {
                Ok(_) => ok += 1,
                Err(e) => eprintln!("  {label}: {e:?} at r={:.0} km", p.norm() / 1e3),
            }
        }
        let dt = t0.elapsed().as_secs_f64() / pts.len() as f64;
        eprintln!("{label}: {:.1} us per L ({ok}/{} ok)", dt * 1e6, pts.len());
    }
}
