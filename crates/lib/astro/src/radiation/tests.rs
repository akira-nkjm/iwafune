//! 束縛粒子の地図の読み取りと補間のテスト。合成の小さな地図（同じ形式で試験の中に書く）で手計算と比べる。
//! NASA の例表（`radbelt.log`）・README の例との照合は [`nasa`]（配布ファイルがキャッシュにあるときだけ走る）。

use super::*;

mod nasa;

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * b.abs().max(1e-30)
}

/// 語の列を `(1X,12I6)` の文字列にする。
fn ascii(words: &[i64]) -> String {
    words
        .chunks(12)
        .map(|row| {
            let body: String = row.iter().map(|w| format!("{w:6}")).collect();
            format!(" {body}\n")
        })
        .collect()
}

/// 合成の地図。倍率はすべて 100、1/4 桁刻み。エネルギー 1 MeV と 2 MeV の副地図:
///
/// - 1 MeV: L = 1.0 は log₁₀F₀ = 3、B/B₀ の増分 0.5・0.5・1.0（位置 1・1.5・2・3）、L = 2.0 は log₁₀F₀ = 2、増分 1・1（位置 1・2・3）
/// - 2 MeV: L = 1.0 は log₁₀F₀ = 2、増分 0.5・0.5（位置 1・1.5・2）、L = 2.0 はフラックス無し
///
/// どの副地図も `L = 0` と `L = 0.9`（フラックス無し）の見かけの節点で始まり、`L = 327.67` の節点で終わる。
pub(super) fn synthetic_words() -> Vec<i64> {
    let sub = |energy: i64, nodes: &[&[i64]]| -> Vec<i64> {
        let mut body = vec![0, energy];
        body.extend([3, 0, 0, 3, 90, 0]);
        for n in nodes {
            body.extend_from_slice(n);
        }
        body.extend([3, 32767, 0]);
        body[0] = body.len() as i64;
        body
    };
    let mut body = sub(
        100,
        &[
            &[6, 100, 300, 50, 50, 100][..],
            &[5, 200, 200, 100, 100][..],
        ],
    );
    body.extend(sub(200, &[&[5, 100, 200, 50, 50][..], &[3, 200, 0][..]]));
    body.extend([0, 0, 0]);
    let mut words = vec![99, 4, 1970, 100, 100, 100, 100, body.len() as i64];
    words.extend(body);
    words
}

pub(super) fn synthetic_map() -> TrappedMap {
    TrappedMap::from_ascii_bytes(ascii(&synthetic_words()).as_bytes()).expect("synthetic map")
}

#[test]
fn parser_reads_header_energies_and_structure() {
    let m = synthetic_map();
    let h = m.header();
    assert_eq!(
        (h.map_type, h.epoch_year, h.steps_per_decade),
        (99, 1970, 4)
    );
    assert_eq!(m.energies_mev(), vec![1.0, 2.0]);
}

#[test]
fn parser_rejects_malformed_files() {
    // 見出しの語数と実際が違う
    let mut w = synthetic_words();
    w[7] += 1;
    assert!(TrappedMap::from_ascii_bytes(ascii(&w).as_bytes()).is_err());
    // 副地図の語数が全体からはみ出す
    let mut w = synthetic_words();
    w[8] = 9999;
    assert!(TrappedMap::from_ascii_bytes(ascii(&w).as_bytes()).is_err());
    // エネルギーが昇順でない
    let mut w = synthetic_words();
    let second = 8 + w[8] as usize;
    w[second + 1] = 50;
    assert!(TrappedMap::from_ascii_bytes(ascii(&w).as_bytes()).is_err());
    // 整数でない欄・空の入力・ASCII でない入力
    assert!(TrappedMap::from_ascii_bytes(b" 1 2 x 4 5 6 7 8\n").is_err());
    assert!(TrappedMap::from_ascii_bytes(b"").is_err());
    assert!(TrappedMap::from_ascii_bytes(&[0xff, 0xfe, 0xfd]).is_err());
}

/// 節点の上の B/B₀ の折れ線: `log₁₀F = log₁₀F₀ − (k + 区間内の割合)/4`。
#[test]
fn flux_along_b_over_b0_on_a_node_matches_hand_calculation() {
    let m = synthetic_map();
    let f = |bb0: f64| m.integral_flux(1.0, bb0, 1.0);
    assert!(near(f(1.0), 1000.0));
    // 区間 0（位置 1〜1.5）の中点: 3 − 0.5/4 = 2.875
    assert!(near(f(1.25), 10f64.powf(2.875)));
    // 位置 1.5 でちょうど 1/4 桁下がる
    assert!(near(f(1.5), 10f64.powf(2.75)));
    // 区間 2（位置 2〜3）の中点: 3 − 2.5/4 = 2.375
    assert!(near(f(2.5), 10f64.powf(2.375)));
    // 増分が尽きた先（位置 3 以上）は 0
    assert_eq!(f(3.0), 0.0);
    assert_eq!(f(10.0), 0.0);
    // B/B₀ < 1 は 1 とみなす
    assert!(near(f(0.5), 1000.0));
}

/// B/B₀ = 1 では L の補間は log₁₀F₀ の直線補間。B/B₀ > 1 は 2 つの副副地図の折れ線の間（NASA の手順）。
#[test]
fn flux_between_l_nodes_at_the_equator_is_log_linear() {
    let m = synthetic_map();
    // L = 1.5、B/B₀ = 1: log 3 と 2 の中間 2.5
    assert!(near(m.integral_flux(1.5, 1.0, 1.0), 10f64.powf(2.5)));
    // L = 1.25: 3 − 0.25 = 2.75
    assert!(near(m.integral_flux(1.25, 1.0, 1.0), 10f64.powf(2.75)));
    // B/B₀ = 2.5、L = 1.5: 両側の節点の値（L = 1 は 2.375、L = 2 は 1.625）の間にある
    let f = m.integral_flux(1.5, 2.5, 1.0);
    assert!(f > 10f64.powf(1.625) && f < 10f64.powf(2.375), "{f}");
    // 節点の上（L = 1）の B/B₀ = 1.25 は、手で追った手順の結果と同じ: 10^2.875
    assert!(near(m.integral_flux(1.0, 1.25, 1.0), 10f64.powf(2.875)));
}

/// フラックス無しの副副地図は対数フラックス 0 として扱われる（TRARA2 の「空」の分岐）。範囲の外と不正な引数は 0。
#[test]
fn empty_sub_sub_maps_and_invalid_arguments() {
    let m = synthetic_map();
    // 見かけの節点 L = 0.9（空）と L = 1.0 の間（B/B₀ = 1）: 空の側の log₁₀F を 0 として直線補間 → 3·0.5 = 1.5
    assert!(near(m.integral_flux(0.95, 1.0, 1.0), 10f64.powf(1.5)));
    // L = 0.5 は両方空で 0
    assert_eq!(m.integral_flux(0.5, 1.0, 1.0), 0.0);
    // この合成の地図は、フラックスのある最後の節点 L = 2 のすぐ後ろが見かけの節点（L = 327.67）なので、その間は log₁₀F が直線に下がる
    // （実際の地図は最後の節点の後ろにフラックス無しの節点がある）。L = 1.5 より外の L の方が小さい値になる
    assert!(m.integral_flux(50.0, 1.0, 1.0) < m.integral_flux(2.0, 1.0, 1.0));
    // B/B₀ が増分の尽きた先（位置 3 を越える）は 0
    assert_eq!(m.integral_flux(1.0, 3.5, 1.0), 0.0);
    assert_eq!(m.integral_flux(f64::NAN, 1.0, 1.0), 0.0);
    assert_eq!(m.integral_flux(-1.0, 1.0, 1.0), 0.0);
    assert_eq!(m.integral_flux(1.0, 1.0, 0.0), 0.0);
}

/// エネルギーの補間は log₁₀F の E についての直線補間。範囲の外は隣り合う 2 つの副地図からの直線の外挿。
#[test]
fn flux_between_energies_is_log_linear_in_energy() {
    let m = synthetic_map();
    // L = 1、B/B₀ = 1: 1 MeV で log 3、2 MeV で log 2
    assert!(near(m.integral_flux(1.0, 1.0, 1.5), 10f64.powf(2.5)));
    assert!(near(m.integral_flux(1.0, 1.0, 1.25), 10f64.powf(2.75)));
    assert!(near(m.integral_flux(1.0, 1.0, 2.0), 100.0));
    // 最後の副地図より上、最初より下は外挿（log₁₀F = 3 − (E − 1)）
    assert!(near(m.integral_flux(1.0, 1.0, 2.5), 10f64.powf(1.5)));
    assert!(near(m.integral_flux(1.0, 1.0, 0.5), 10f64.powf(3.5)));
    // 外挿で log₁₀F ≤ 0 になればフラックス 0
    assert_eq!(m.integral_flux(1.0, 1.0, 4.5), 0.0);
    // 2 MeV の L = 2 は空: L = 1.5 の 2 MeV は log₁₀F = 2·0.5 = 1（空の側を 0 として補間）→ 10
    assert!(near(m.integral_flux(1.5, 1.0, 2.0), 10.0));
    // 1 MeV の log 2.5 と 2 MeV の log 1 の間（E = 1.5）: 1.75
    assert!(near(m.integral_flux(1.5, 1.0, 1.5), 10f64.powf(1.75)));
}

#[test]
fn model_table_pairs_each_map_with_its_field() {
    use BeltField::*;
    assert_eq!(TrappedModel::Ap8Max.field(), Gsfc1266To1970);
    for m in [
        TrappedModel::Ap8Min,
        TrappedModel::Ae8Min,
        TrappedModel::Ae8Max,
    ] {
        assert_eq!(m.field(), JensenCain1960);
    }
    for m in TrappedModel::ALL {
        assert_eq!(TrappedModel::of(m.species(), m.solar()), m);
    }
}

/// 中心双極子の場では、L は赤道面の距離 / a、B/B₀ は 1（赤道）・`√(1+3sin²λ)/cos⁶λ`（磁気緯度 λ）。
/// 強さの違う中心双極子: 文献の定義は固定の M で L を引くので `L = r/a·(M/M')^(1/3)`、B/B₀ は 1。
/// 比較用の定義は場自身のモーメントなので L = r/a。
#[test]
fn conventions_differ_by_the_moment_used() {
    use crate::geomag::{GaussModel, GaussTerm, REFERENCE_RADIUS_M as A};
    use crate::math::Vec3;
    use crate::time::Epoch;
    let epoch = Epoch::from_calendar(1970, 1, 1, 0, 0, 0.0);
    let term = GaussTerm {
        n: 1,
        m: 0,
        g: -30_000.0,
        h: 0.0,
        g_dot: 0.0,
        h_dot: 0.0,
    };
    let snap = GaussModel::new("weaker", epoch, A, vec![term])
        .expect("model")
        .at(epoch);
    let r = Vec3::new(3.0 * A, 0.0, 0.0);
    let lit = belt_position_with(&snap, r, BeltConvention::Literature).expect("position");
    let expected = 3.0 * (31_165.3f64 / 30_000.0).cbrt();
    assert!((lit.l - expected).abs() < 2e-5, "{lit:?}");
    assert!((lit.b_over_b0 - 1.0).abs() < 1e-4, "{lit:?}");
    let own =
        belt_position_with(&snap, r, BeltConvention::ModelDipoleLineMinimum).expect("position");
    assert!(
        (own.l - 3.0).abs() < 2e-5 && (own.b_over_b0 - 1.0).abs() < 1e-4,
        "{own:?}"
    );
    assert_eq!(BeltConvention::default(), BeltConvention::Literature);
}

#[test]
fn belt_position_in_a_centred_dipole() {
    use crate::geomag::{GaussModel, GaussTerm, REFERENCE_RADIUS_M as A};
    use crate::math::Vec3;
    use crate::time::Epoch;
    // 中心双極子（g10 = −31165.3 nT）。L は赤道面の距離 / a、B/B₀ は磁力線に沿う最小値に対する比
    let epoch = Epoch::from_calendar(1970, 1, 1, 0, 0, 0.0);
    let term = GaussTerm {
        n: 1,
        m: 0,
        g: -31_165.3,
        h: 0.0,
        g_dot: 0.0,
        h_dot: 0.0,
    };
    let model = GaussModel::new("dipole", epoch, A, vec![term]).expect("model");
    let snap = model.at(epoch);
    let p = belt_position(&snap, Vec3::new(3.0 * A, 0.0, 0.0)).expect("position");
    assert!((p.l - 3.0).abs() < 1e-5, "{p:?}");
    assert!((p.b_over_b0 - 1.0).abs() < 1e-5, "{p:?}");
    // L = 2、磁気緯度 30°: r = L cos²λ a。B/B₀ = √(1 + 3 sin²λ)/cos⁶λ = √1.75/0.421875 = 3.1357
    let lat = 30f64.to_radians();
    let r = 2.0 * lat.cos().powi(2) * A;
    let p = belt_position(&snap, Vec3::new(r * lat.cos(), 0.0, r * lat.sin())).expect("position");
    assert!((p.l - 2.0).abs() < 1e-5, "{p:?}");
    let expected = (1.0 + 3.0 * lat.sin().powi(2)).sqrt() / lat.cos().powi(6);
    assert!((p.b_over_b0 - expected).abs() < 1e-4, "{p:?} {expected}");
}
