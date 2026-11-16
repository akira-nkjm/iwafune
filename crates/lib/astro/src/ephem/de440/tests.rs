use super::*;
use crate::ephem::{Body, analytic_position_eci_m};

/// 小さな合成の表: 4 区間（SSB→EMB, EMB→地球, EMB→月, SSB→太陽）、各 2 区間・係数 3 個（2 個は f64、1 個は f32）。
/// 係数は成分ごとに (c0, c1, c2) で、位置 = c0 + c1·x + c2·(2x² − 1)。
fn synthetic() -> Vec<u8> {
    let segs: [(i32, i32, [[f64; 3]; 3]); 4] = [
        (
            0,
            3,
            [[1.0e8, 10.0, 0.5], [2.0e7, 0.0, 0.0], [0.0, 0.0, 0.0]],
        ),
        (
            3,
            399,
            [[4600.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        ),
        (
            3,
            301,
            [[-3.8e5, 0.0, 0.0], [0.0, 1.0e3, 0.0], [0.0, 0.0, 0.0]],
        ),
        (0, 10, [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0e3, 0.0, 0.0]]),
    ];
    let (n_intervals, n_coeffs, n_f64, components) = (2u32, 3u32, 2u32, 3u32);
    let (init, interval) = (-86_400.0_f64, 86_400.0_f64);
    let mut b = Vec::new();
    b.extend_from_slice(MAGIC);
    b.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    b.extend_from_slice(&(segs.len() as u32).to_le_bytes());
    for (c, t, _) in &segs {
        b.extend_from_slice(&c.to_le_bytes());
        b.extend_from_slice(&t.to_le_bytes());
        for v in [n_intervals, n_coeffs, n_f64, components] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&init.to_le_bytes());
        b.extend_from_slice(&interval.to_le_bytes());
    }
    for (_, _, coeffs) in &segs {
        for comp in coeffs {
            for _ in 0..n_intervals {
                b.extend_from_slice(&comp[0].to_le_bytes());
                b.extend_from_slice(&comp[1].to_le_bytes());
                b.extend_from_slice(&(comp[2] as f32).to_le_bytes());
            }
        }
    }
    b
}

#[test]
fn parses_and_evaluates_synthetic_table() {
    let t = De440::from_bytes(synthetic()).expect("synthetic table");
    assert_eq!(t.span_s(), (-86_400.0, 86_400.0));
    // 区間 1 の中央（x = 0）: 時刻は TDB なので、TT − TDB 補正の分だけ x が 0 からずれる
    let e = Epoch {
        j2000_s: 43_200.0 - (tt_to_tdb_s(43_200.0) - 43_200.0),
    };
    assert!(t.covers(e));
    let moon = t.geocentric_position_m(body_id::MOON, e).expect("moon");
    // 月 = (EMB→月) − (EMB→地球) = (−3.8e5 − 4600, 1e3·x, 0) km、x ≈ 0
    assert!((moon.x - (-3.8e5 - 4600.0) * 1e3).abs() < 1e-3, "{moon:?}");
    assert!(moon.y.abs() < 1e-3);
    // 太陽 = (SSB→太陽) − (SSB→EMB) − (EMB→地球)。z は c0 = 1000 km、x は −(1e8 + 10x + 0.5(2x²−1)) − 4600
    let sun = t.geocentric_position_m(body_id::SUN, e).expect("sun");
    assert!((sun.z - 1.0e6).abs() < 1e-6);
    assert!(
        (sun.x - (-(1.0e8 - 0.5) - 4600.0) * 1e3).abs() < 1.0,
        "{sun:?}"
    );
    // 期間外
    assert!(!t.covers(Epoch { j2000_s: 1e6 }));
    assert!(
        t.geocentric_position_m(body_id::SUN, Epoch { j2000_s: 1e6 })
            .is_none()
    );
    assert!(
        t.geocentric_position_m(body_id::MARS_BARYCENTER, e)
            .is_none()
    );
}

#[test]
fn rejects_malformed_bytes() {
    assert!(De440::from_bytes(b"NOTDE440xxxxxxxx".to_vec()).is_err());
    let mut short = synthetic();
    short.pop();
    assert!(De440::from_bytes(short).is_err());
    let mut bad_version = synthetic();
    bad_version[8] = 9;
    assert!(De440::from_bytes(bad_version).is_err());
    assert!(De440::from_bytes(Vec::new()).is_err());
}

/// 実データ（環境変数 SAT_ASTRO_DE440 のパス）を読み、解析式と比べる。
/// `SAT_ASTRO_DE440=/path/de440.bin cargo test -p sat-astro -- --ignored de440` で実行する。
#[test]
#[ignore = "needs the real de440.bin (20.7 MB)"]
fn real_table_agrees_with_analytic_series() {
    let Ok(path) = std::env::var("SAT_ASTRO_DE440") else {
        return;
    };
    let bytes = std::fs::read(path).expect("cannot read de440.bin");
    let t = De440::from_bytes(bytes).expect("real data");
    let e = Epoch::parse("2027-04-01T00:00:00Z").expect("epoch");
    assert!(t.covers(e));
    for (body, id, max_deg, max_rel) in [
        (Body::Sun, body_id::SUN, 0.001, 1e-4),
        (Body::Moon, body_id::MOON, 0.01, 1e-3),
        (Body::Jupiter, body_id::JUPITER_BARYCENTER, 0.5, 0.01),
    ] {
        let de = t.geocentric_position_m(id, e).expect("within range");
        let an = analytic_position_eci_m(body, e);
        let ang = de.angle_to(an).to_degrees();
        let rel = (de.norm() / an.norm() - 1.0).abs();
        assert!(ang < max_deg && rel < max_rel, "{body:?}: {ang}° / {rel}");
    }
    let (start, end) = t.span_s();
    assert!(start < -4.7e9 && end > 4.7e9, "{start} {end}");
}
