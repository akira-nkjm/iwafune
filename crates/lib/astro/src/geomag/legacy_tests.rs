//! 古い場の係数ファイル（Jensen & Cain 1960・GSFC 12/66）と IGRF の読み込みのテスト。
//! 合成の小さなファイルで換算を手計算と比べる。配布ファイルがキャッシュにあれば、本物でも確かめる（無ければ読み飛ばす）。

use super::legacy::jensen_cain_factor;
use super::*;
use crate::test_data::cached_bytes;

const A: f64 = REFERENCE_RADIUS_M;

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs().max(1e-30)
}

fn term_of(m: &GaussModel, n: u8, order: u8) -> GaussTerm {
    *m.terms()
        .iter()
        .find(|t| t.n == n && t.m == order)
        .expect("term")
}

/// 合成の Jensen & Cain 形式（`n`・`m` は 1 を足した値、`F11` の欄）。
const JC_SAMPLE: &str = "\
 5 1960.0       JENSEN CAIN
  2  1    30000.0
  3  2     3000.0      -900.0
  3  3     1000.0       100.0
";

#[test]
fn jensen_cain_factors_match_hand_calculation() {
    // 次数 1・位数 0: −1!/(1!!·1) = −1
    assert!(near(jensen_cain_factor(1, 0), -1.0, 1e-14));
    // (2, 1): S = √(2·1!/3!) = 1/√3、(2n−1)!! = 3 → −1/(3/√3) = −1/√3
    assert!(near(jensen_cain_factor(2, 1), -1.0 / 3f64.sqrt(), 1e-14));
    // (2, 2): S = √(2·0!/4!) = 1/√12 → −1/(3/√12) = −√12/3
    assert!(near(jensen_cain_factor(2, 2), -12f64.sqrt() / 3.0, 1e-14));
    // (3, 0): −3!/15 = −0.4
    assert!(near(jensen_cain_factor(3, 0), -0.4, 1e-14));
}

#[test]
fn jensen_cain_file_is_converted_to_schmidt_igrf_convention() {
    let m = GaussModel::from_jensen_cain_bytes(JC_SAMPLE.as_bytes()).expect("parse");
    assert_eq!(m.degree(), 2);
    assert_eq!(m.ref_radius_m(), A);
    assert_eq!(m.epoch(), Epoch::from_calendar(1960, 1, 1, 0, 0, 0.0));
    let g10 = term_of(&m, 1, 0);
    assert!(near(g10.g, -30_000.0, 1e-14) && g10.h == 0.0);
    let t21 = term_of(&m, 2, 1);
    assert!(near(t21.g, -3000.0 / 3f64.sqrt(), 1e-13), "{t21:?}");
    assert!(near(t21.h, 900.0 / 3f64.sqrt(), 1e-13), "{t21:?}");
    let t22 = term_of(&m, 2, 2);
    assert!(near(t22.g, -1000.0 * 12f64.sqrt() / 3.0, 1e-13), "{t22:?}");
    assert!(near(t22.h, -100.0 * 12f64.sqrt() / 3.0, 1e-13), "{t22:?}");
}

/// 次数 1 の場は、符号の反転だけで標準の双極子になる: 北極の表面で下向きに 2·|g10|。
#[test]
fn jensen_cain_dipole_points_down_at_the_north_pole() {
    let m = GaussModel::from_jensen_cain_bytes(" 1 1960.0 X\n  2  1    30000.0\n".as_bytes())
        .expect("parse");
    let b = m.field_ecef_t(Vec3::new(0.0, 0.0, A), m.epoch());
    assert!(near(b.z, -60_000e-9, 1e-9), "{b:?}");
}

/// 合成の GSFC 12/66 形式の 1 行（`n`・`m` は 1 を足した値、`F11` の欄 6 つ、行番号の欄）。
fn gsfc_row(n: u8, m: u8, v: [f64; 6]) -> String {
    let fields: String = v.iter().map(|x| format!("{x:11.3}")).collect();
    format!("{n:3}{m:3}{fields}12660030\n")
}

fn gsfc_sample() -> String {
    format!(
        "00 1960.0      GSFC(12/66) SET 1 ROUNDED                                12660010\nGSFC1266               10                                               12660020\n{}{}      BLOCK DATA                                                        12660680\n",
        gsfc_row(2, 1, [-30_000.0, 0.0, 15.0, 0.0, -0.1, 0.0]),
        gsfc_row(2, 2, [-2_000.0, 5_000.0, 10.0, -4.0, 0.2, -0.04]),
    )
}

#[test]
fn gsfc_coefficients_are_extrapolated_with_a_half_second_derivative() {
    let m = GaussModel::from_gsfc1266_bytes(gsfc_sample().as_bytes(), 1970.0).expect("parse");
    assert_eq!(m.epoch(), Epoch::from_calendar(1970, 1, 1, 0, 0, 0.0));
    // g(1970) = g + ġ·10 + ½·g̈·100
    let g10 = term_of(&m, 1, 0);
    assert!(near(g10.g, -30_000.0 + 150.0 - 5.0, 1e-14), "{g10:?}");
    let t11 = term_of(&m, 1, 1);
    assert!(near(t11.g, -2000.0 + 100.0 + 10.0, 1e-14), "{t11:?}");
    assert!(near(t11.h, 5000.0 - 40.0 - 2.0, 1e-14), "{t11:?}");
    // 固定の場: 年変化は 0（`at` を別の日に求めても変わらない）
    assert_eq!((g10.g_dot, t11.h_dot), (0.0, 0.0));
    // 1960 年に外挿しなければ元の値
    let base = GaussModel::from_gsfc1266_bytes(gsfc_sample().as_bytes(), 1960.0).expect("parse");
    assert!(near(term_of(&base, 1, 0).g, -30_000.0, 1e-14));
}

#[test]
fn legacy_parsers_reject_files_without_coefficients() {
    assert!(GaussModel::from_jensen_cain_bytes(b" 1 1960.0 X\n").is_err());
    assert!(GaussModel::from_gsfc1266_bytes(b"not a file", 1970.0).is_err());
    assert!(GaussModel::from_jensen_cain_bytes(&[0xff, 0xfe]).is_err());
}

const IGRF_SAMPLE: &str = "\
# sample
c/s deg ord IGRF IGRF SV
g/h n m 2000.0 2005.0 2005-10
g 1 0 -30000 -29900 10
g 1 1 -2000 -1950 0
h 1 1 5000 5100 20
";

#[test]
fn igrf_is_interpolated_between_epochs_and_extrapolated_with_the_secular_variation() {
    let t = IgrfTable::from_bytes(IGRF_SAMPLE.as_bytes()).expect("parse");
    assert_eq!(t.year_range(), (2000.0, 2005.0));
    let mid = Epoch::from_calendar(2002, 7, 2, 12, 0, 0.0); // 2002.5（Julian 年で）
    let m = t.model_at(mid).expect("model");
    let snap = m.at(mid);
    // g10 は 2000 → 2005 で +100 nT: 2.5 年で -29950 nT（北極の表面で z = 2·g10）
    let z = snap.field_ecef_t(Vec3::new(0.0, 0.0, A)).z;
    assert!(near(z, 2.0 * -29_950e-9, 2e-4), "{z}");
    // 最後の元期の後は ġ = 10 nT/yr（2 年後に -29900 + 20 = -29880）
    let late = Epoch::from_calendar(2007, 1, 1, 0, 0, 0.0);
    let z = t.model_at(late).expect("model").at(late);
    let z = z.field_ecef_t(Vec3::new(0.0, 0.0, A)).z;
    assert!(near(z, 2.0 * -29_880e-9, 2e-4), "{z}");
    // 最初の元期より前は使えない
    assert!(
        t.model_at(Epoch::from_calendar(1990, 1, 1, 0, 0, 0.0))
            .is_err()
    );
}

// ---- 配布ファイルがあるときだけ走る試験 ----

/// 北極の表面（r = a）では m = 0 の項だけが効き、`B_z = Σ (n+1)·gₙ⁰`。配布ファイルの Jensen & Cain の `gₙ⁰`
/// （次数 1〜6）から、換算 `−n!/(2n−1)!!` を手で当てて足した値と比べる。
#[test]
fn jensen_cain_north_pole_field_matches_hand_sum() {
    let Some(bytes) = cached_bytes("jensen_cain_62.dat") else {
        return;
    };
    let m = GaussModel::from_jensen_cain_bytes(&bytes).expect("parse");
    // ファイルの `gₙ⁰`（n = 1..=6）[nT]
    let listed = [30_411.2, 2_403.5, -3_151.8, -4_179.4, 1_625.6, -1_952.3];
    let mut fact = 1.0;
    let mut dfact = 1.0;
    let mut expected = 0.0;
    for (i, g) in listed.iter().enumerate() {
        let n = (i + 1) as f64;
        fact *= n;
        dfact *= 2.0 * n - 1.0;
        expected += (n + 1.0) * (-g * fact / dfact);
    }
    // 手計算の合計 −56102.2 nT（北極で下向き）
    assert!(near(expected, -56_102.2, 1e-5), "{expected}");
    let b = m.field_ecef_t(Vec3::new(0.0, 0.0, A), m.epoch());
    assert!(near(b.z, expected * 1e-9, 1e-9), "{b:?}");
}

/// 双極子の項は、Jensen & Cain のファイルの `g₁⁰ = 30411.2`・`g₁¹ = 2147.4`・`h₁¹ = −5798.9` の符号を反転した値で、
/// 赤道の強さ `√(g10² + g11² + h11²) = 31033.7 nT`（0.310 G）。
#[test]
fn jensen_cain_dipole_terms_and_moment() {
    let Some(bytes) = cached_bytes("jensen_cain_62.dat") else {
        return;
    };
    let m = GaussModel::from_jensen_cain_bytes(&bytes).expect("parse");
    let (g10, g11) = (term_of(&m, 1, 0), term_of(&m, 1, 1));
    assert!(near(g10.g, -30_411.2, 1e-12));
    assert!(near(g11.g, -2_147.4, 1e-12) && near(g11.h, 5_798.9, 1e-12));
    let b0 = m.at(m.epoch()).dipole_b0_t() * 1e9;
    assert!(near(b0, 31_033.7, 2e-5), "{b0}");
}

/// 換算の確かめ: 1960 年の IGRF（DGRF）の係数と、次数 4 までで 120 nT 以内に合う（換算を誤ると数千 nT ずれる）。
#[test]
fn jensen_cain_matches_dgrf1960() {
    let (Some(jc), Some(igrf)) = (
        cached_bytes("jensen_cain_62.dat"),
        cached_bytes("igrf14coeffs.txt"),
    ) else {
        return;
    };
    let old = GaussModel::from_jensen_cain_bytes(&jc).expect("jc");
    let table = IgrfTable::from_bytes(&igrf).expect("igrf");
    let epoch = Epoch::from_calendar(1960, 1, 1, 0, 0, 0.0);
    let new = table.model_at(epoch).expect("igrf 1960");
    for t in old.terms().iter().filter(|t| t.n <= 4) {
        let r = term_of(&new, t.n, t.m);
        assert!(
            (t.g - r.g).abs() < 120.0,
            "g({},{}): {} vs {}",
            t.n,
            t.m,
            t.g,
            r.g
        );
        assert!(
            (t.h - r.h).abs() < 120.0,
            "h({},{}): {} vs {}",
            t.n,
            t.m,
            t.h,
            r.h
        );
    }
}

/// GSFC 12/66 の 1960 年の係数は DGRF 1960 に近く（次数 4 まで 110 nT 以内）、1970 年への外挿は DGRF 1970 に近い。
#[test]
fn gsfc1266_is_close_to_dgrf() {
    let (Some(gsfc), Some(igrf)) = (cached_bytes("gsfc66.dat"), cached_bytes("igrf14coeffs.txt"))
    else {
        return;
    };
    let table = IgrfTable::from_bytes(&igrf).expect("igrf");
    for year in [1960_i64, 1970] {
        let old = GaussModel::from_gsfc1266_bytes(&gsfc, year as f64).expect("gsfc");
        let new = table
            .model_at(Epoch::from_calendar(year, 1, 1, 0, 0, 0.0))
            .expect("igrf");
        for t in old.terms().iter().filter(|t| t.n <= 4) {
            let r = term_of(&new, t.n, t.m);
            assert!(
                (t.g - r.g).abs() < 110.0,
                "{year} g({},{}): {} vs {}",
                t.n,
                t.m,
                t.g,
                r.g
            );
            assert!(
                (t.h - r.h).abs() < 110.0,
                "{year} h({},{}): {} vs {}",
                t.n,
                t.m,
                t.h,
                r.h
            );
        }
    }
}
