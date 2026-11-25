//! 球面調和モデル（Schmidt 準正規化・傾斜双極子との一致・年変化・文字列形式）のテスト。共通の道具は L のテストも使う。

use super::*;

pub(super) const A: f64 = 6_371_200.0;

pub(super) fn epoch() -> Epoch {
    Epoch::parse("2027-04-01T00:00:00Z").expect("epoch")
}

pub(super) fn term(n: u8, m: u8, g: f64, h: f64) -> GaussTerm {
    GaussTerm {
        n,
        m,
        g,
        h,
        g_dot: 0.0,
        h_dot: 0.0,
    }
}

pub(super) fn model(terms: Vec<GaussTerm>) -> GaussModel {
    GaussModel::new("test", epoch(), A, terms).expect("model")
}

/// 軸が z の中心双極子（g10 = −30000 nT）。
pub(super) fn centred_dipole() -> GaussModel {
    model(vec![term(1, 0, -30_000.0, 0.0)])
}

/// 磁気緯度 `lat`、距離 `r` の点（経度 `lon`）。
pub(super) fn at(r: f64, lat: f64, lon: f64) -> Vec3 {
    Vec3::new(
        r * lat.cos() * lon.cos(),
        r * lat.cos() * lon.sin(),
        r * lat.sin(),
    )
}

#[test]
pub(super) fn centred_dipole_pole_and_equator() {
    let snap = centred_dipole().at(epoch());
    let eq = snap.field_ecef_t(Vec3::new(A, 0.0, 0.0));
    assert!((eq.norm() - 30_000e-9).abs() < 1e-15, "{eq:?}");
    assert!(eq.z > 0.0 && eq.x.abs() < 1e-15, "北向き: {eq:?}");
    let pole = snap.field_ecef_t(Vec3::new(0.0, 0.0, A));
    assert!((pole.norm() - 60_000e-9).abs() < 1e-14, "{pole:?}");
    assert!(pole.z < 0.0, "北極では下向き");
    // 距離の 3 乗に反比例
    let far = snap.field_ecef_t(Vec3::new(2.0 * A, 0.0, 0.0));
    assert!((far.norm() * 8.0 - 30_000e-9).abs() < 1e-15);
    assert_eq!(snap.field_ecef_t(Vec3::ZERO), Vec3::ZERO);
}

/// 既存の傾斜双極子は次数 1 の Gauss 係数で表せ、同じ場を返す（g10・g11・h11 は軸と B₀ から決まる）。
#[test]
fn tilted_dipole_is_degree_one_case() {
    let m = GaussModel::tilted_dipole(epoch(), A).expect("dipole");
    assert_eq!(m.degree(), 1);
    // 係数のノルムは基準半径の球面上の赤道強度 B₀ (R_E/a)³。
    let g = |n, mm| {
        m.terms()
            .iter()
            .find(|t| t.n == n && t.m == mm)
            .copied()
            .expect("term")
    };
    let norm = (g(1, 0).g.powi(2) + g(1, 1).g.powi(2) + g(1, 1).h.powi(2)).sqrt();
    let expect = B0_EARTH * (R_EARTH / A).powi(3) * 1e9;
    assert!((norm - expect).abs() < 1e-9, "{norm} {expect}");
    // g10 は負（モーメントは南向き）。
    assert!(g(1, 0).g < 0.0);
    for (i, r) in [
        Vec3::new(7.0e6, 1.0e6, 2.0e6),
        Vec3::new(-3.0e6, 5.5e6, -4.0e6),
        Vec3::new(0.1e6, -0.2e6, 6.9e6),
        Vec3::new(2.5e7, -1.0e7, 3.0e6),
    ]
    .into_iter()
    .enumerate()
    {
        let (a, b) = (m.field_eci_t(r, epoch()), dipole_field_eci_t(r, epoch()));
        assert!((a - b).norm() / b.norm() < 1e-12, "{i}: {a:?} {b:?}");
    }
}

/// n = 2 の Schmidt 準正規化: ポテンシャル `V = a (a/r)³ (g22 cos2φ + h21 cosφ ...)` の閉じた形の勾配と一致する。
#[test]
fn schmidt_degree_two_matches_closed_form() {
    let (g22, g21, h21) = (700.0, -1200.0, 900.0);
    let snap = model(vec![term(2, 2, g22, 0.0), term(2, 1, g21, h21)]).at(epoch());
    let (r, th, ph) = (1.7 * A, 1.1_f64, 0.6_f64);
    let (st, ct, sp, cp) = (th.sin(), th.cos(), ph.sin(), ph.cos());
    let (s2p, c2p) = ((2.0 * ph).sin(), (2.0 * ph).cos());
    // Schmidt: P22 = (√3/2) sin²θ、P21 = √3 sinθ cosθ（非正規化 3 sin²θ・3 sinθ cosθ に √((2−δ)(n−m)!/(n+m)!) を掛けたもの）
    let (p22, dp22) = (3.0_f64.sqrt() / 2.0 * st * st, 3.0_f64.sqrt() * st * ct);
    let (p21, dp21) = (
        3.0_f64.sqrt() * st * ct,
        3.0_f64.sqrt() * (ct * ct - st * st),
    );
    let k = A * (A / r).powi(3); // V = k·(…)
    // V = k [ g22 cos2φ P22 + (g21 cosφ + h21 sinφ) P21 ]、B = −∇V（nT）
    let v = g22 * c2p * p22 + (g21 * cp + h21 * sp) * p21;
    let b_r = 3.0 * k * v / r;
    let b_th = -k / r * (g22 * c2p * dp22 + (g21 * cp + h21 * sp) * dp21);
    let b_ph = -k / (r * st) * (-2.0 * g22 * s2p * p22 + (-g21 * sp + h21 * cp) * p21);
    // 球面成分 → 直交成分
    let (er, eth, eph) = (
        Vec3::new(st * cp, st * sp, ct),
        Vec3::new(ct * cp, ct * sp, -st),
        Vec3::new(-sp, cp, 0.0),
    );
    let expect = (er * b_r + eth * b_th + eph * b_ph) * 1e-9;
    let got = snap.field_ecef_t(er * r);
    assert!(
        (got - expect).norm() / expect.norm() < 1e-12,
        "{got:?} vs {expect:?}"
    );
}

/// 次数 2 の帯状項 g20: `B_r = 3 g20 (a/r)⁴ P2`、`B_θ = 3 g20 (a/r)⁴ sinθ cosθ`。
#[test]
fn zonal_quadrupole_matches_closed_form() {
    let snap = model(vec![term(2, 0, 1000.0, 0.0)]).at(epoch());
    let (r, th) = (1.3 * A, 0.9_f64);
    let f = 3.0 * 1000.0 * (A / r).powi(4);
    let p2 = (3.0 * th.cos().powi(2) - 1.0) / 2.0;
    let (b_r, b_th) = (f * p2, f * th.sin() * th.cos());
    let (er, eth) = (
        Vec3::new(th.sin(), 0.0, th.cos()),
        Vec3::new(th.cos(), 0.0, -th.sin()),
    );
    let expect = (er * b_r + eth * b_th) * 1e-9;
    let got = snap.field_ecef_t(er * r);
    assert!((got - expect).norm() / expect.norm() < 1e-12);
}

#[test]
fn secular_variation_is_linear_in_years() {
    let mut t = term(1, 0, -30_000.0, 0.0);
    t.g_dot = 20.0;
    let m = model(vec![t]);
    let later = Epoch::from_calendar(2032, 4, 1, 0, 0, 0.0);
    let years =
        later.seconds_since(epoch()) / (crate::constants::SECONDS_PER_JULIAN_CENTURY / 100.0);
    let b0 = m.at(later).dipole_b0_t() * 1e9;
    assert!((b0 - (30_000.0 - 20.0 * years)).abs() < 1e-9, "{b0}");
}

#[test]
fn text_round_trip_and_errors() {
    let mut a = term(2, 1, -1234.5, 99.25);
    a.g_dot = 1.5;
    a.h_dot = -0.75;
    let m = GaussModel::new(
        "synthetic test model",
        epoch(),
        A,
        vec![term(1, 0, -30_000.5, 0.0), term(1, 1, -1500.0, 4800.125), a],
    )
    .expect("model");
    let back = GaussModel::from_text_bytes(m.to_text().as_bytes()).expect("parse");
    assert_eq!(back, m);
    // 年変化の列は省略でき、コメントと空行は読み飛ばす。
    let text =
        "# c\nname: x\nepoch: 2027-04-01T00:00:00Z\nradius_km: 6371.2\n\n1 0 -3.0e4 0 # trailing\n";
    let p = GaussModel::from_text_bytes(text.as_bytes()).expect("short rows");
    assert_eq!(p.terms()[0].g_dot, 0.0);
    assert_eq!(p.ref_radius_m(), A);
    for bad in [
        "epoch: 2027-04-01T00:00:00Z\n1 0 1 0\n", // radius なし
        "epoch: 2027-04-01T00:00:00Z\nradius_km: 6371.2\n1 2 1 0\n", // m > n
        "epoch: 2027-04-01T00:00:00Z\nradius_km: 6371.2\n1 0 1 5\n", // m = 0 で h ≠ 0
        "epoch: 2027-04-01T00:00:00Z\nradius_km: 6371.2\n1 0 1 0\n1 0 2 0\n", // 重複
        "epoch: 2027-04-01T00:00:00Z\nradius_km: 6371.2\n1 0 x 0\n",
        "epoch: 2027-04-01T00:00:00Z\nradius_km: 6371.2\n", // 係数なし
        "epoch: nonsense\nradius_km: 6371.2\n1 0 1 0\n",
    ] {
        assert!(
            GaussModel::from_text_bytes(bad.as_bytes()).is_err(),
            "{bad}"
        );
    }
    assert!(GaussModel::from_text_bytes(&[0xff, 0xfe]).is_err());
}
