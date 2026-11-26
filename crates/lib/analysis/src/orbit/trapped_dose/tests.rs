//! 捕捉粒子の線量解析の試験。合成の地図（`trapped_flux` の試験と同じ作り）と中心双極子の場の赤道面の円軌道で、SHIELDOSE-2 の表（配布ファイル。
//! キャッシュに無ければ該当の試験は読み飛ばす）を使って手計算と比べる。

use super::*;
use sat_astro::constants::R_EARTH;
use sat_astro::geomag::{GaussModel, GaussTerm, REFERENCE_RADIUS_M as A};
use sat_astro::math::spline::NaturalSpline;
use sat_astro::orbit::{OrbitSpec, Propagator};
use sat_astro::radiation::TrappedMap;
use sat_astro::time::Epoch;
use sat_astro::timeline::OrbitSampling;

fn ascii(words: &[i64]) -> String {
    words
        .chunks(12)
        .map(|row| {
            let body: String = row.iter().map(|w| format!("{w:6}")).collect();
            format!(" {body}\n")
        })
        .collect()
}

/// 合成の地図（`trapped_flux` の試験と同じ）。L = 2 の赤道では 0.1 MeV が log₁₀F = top − 1、400 MeV が top − 2.5 で、E の直線。
fn map(top: i64) -> TrappedMap {
    let sub = |energy: i64, f1: i64, f3: i64| {
        let mut body = vec![0, energy, 3, 0, 0, 3, 90, 0];
        body.extend([5, 100, f1, 100, 100]);
        body.extend([5, 300, f3, 100, 100]);
        body.extend([3, 320, 0]);
        body.extend([3, 32767, 0]);
        body[0] = body.len() as i64;
        body
    };
    let mut body = sub(10, top * 100, (top - 2) * 100);
    body.extend(sub(40000, (top - 2) * 100, (top - 3) * 100));
    body.extend([0, 0, 0]);
    let mut words = vec![99, 4, 1970, 100, 100, 100, 100, body.len() as i64];
    words.extend(body);
    TrappedMap::from_ascii_bytes(ascii(&words).as_bytes()).expect("map")
}

fn field(name: &str) -> GaussModel {
    let epoch = Epoch::from_calendar(1970, 1, 1, 0, 0, 0.0);
    let term = GaussTerm {
        n: 1,
        m: 0,
        g: -31_165.3,
        h: 0.0,
        g_dot: 0.0,
        h_dot: 0.0,
    };
    GaussModel::new(name, epoch, A, vec![term]).expect("field")
}

fn input(thicknesses: Option<Vec<f64>>) -> TrappedDoseInput {
    let epoch = Epoch::from_calendar(2027, 4, 1, 0, 0, 0.0);
    let mut orbit = OrbitSpec::circular(
        epoch,
        Qty::from_raw(2.0 * A - R_EARTH),
        Qty::ZERO,
        Qty::ZERO,
    );
    orbit.propagator = Propagator::Kepler;
    TrappedDoseInput {
        sampling: OrbitSampling::new(orbit, Qty::from_raw(1200.0), Qty::from_raw(600.0)),
        thicknesses: thicknesses.map(|v| v.into_iter().map(Qty::from_raw).collect()),
        geometry: None,
        nuclear: None,
    }
}

struct Fixture {
    maps: [TrappedMap; 4],
    fields: [GaussModel; 2],
}

impl Fixture {
    fn new() -> Self {
        Self {
            maps: [map(4), map(5), map(4), map(5)],
            fields: [field("test JC"), field("test GSFC")],
        }
    }
    fn tables(&self) -> RadiationTables<'_> {
        RadiationTables {
            maps: [0, 1, 2, 3].map(|i| Some(&self.maps[i])),
            fields: [Some(&self.fields[0]), Some(&self.fields[1])],
        }
    }
}

fn data() -> Option<ShieldingData> {
    // 配布ファイルの探索は sat-astro の試験用の規則（`IWAFUNE_DATA_DIR`）と同じ
    let dir = std::env::var_os("IWAFUNE_DATA_DIR").filter(|d| !d.is_empty())?;
    let bytes = std::fs::read(std::path::Path::new(&dir).join("shieldose2.dat")).ok()?;
    ShieldingData::from_bytes(&bytes).ok()
}

/// 配布ファイルの配列 `name`（先頭から `n` 個）。
fn array(name: &str, n: usize) -> Vec<f64> {
    let dir = std::env::var_os("IWAFUNE_DATA_DIR").unwrap_or_default();
    let text =
        std::fs::read_to_string(std::path::Path::new(&dir).join("shieldose2.dat")).expect("data");
    let head = format!("array {name} ");
    let at = text.find(&head).expect("array");
    let body = &text[at..];
    let body = &body[body.find('\n').expect("line") + 1..];
    body.split_whitespace()
        .take(n)
        .map(|v| v.parse::<f64>().expect("number"))
        .collect()
}

/// 手計算: 深さがほぼ 0（SHIELDOSE-2 の最小の深さ 1e-6 g/cm²）での球の中心の線量
/// = ∫ j(E) S(E) r(E) dE × 1.6021892e-10 [Gy/s]。`j(E) = F(E) · 1.5 ln10 / 399.9` は合成の地図の微分フラックス
/// （F は L = 2 で `10^(3 − 1.5 (E−0.1)/399.9)`）、`S` はデータの飛程の表から求めた**アルミの**質量阻止能（dE/dR）、
/// `r` はデータの検出器の比（シリコン / アルミ、深さ 0）。許容 1.5 % の内訳: 微分の格子の上端の半区間（積分フラックスで 0.6 %、
/// 線量ではそれ以下）、核の表と飛程の表の導関数の食い違い（単一エネルギーで調べると ±0.6 %）。
#[test]
fn proton_dose_near_zero_depth_matches_the_integral_of_flux_times_stopping_power() {
    let Some(shield) = data() else {
        eprintln!("SKIPPED: shieldose2.dat is not in IWAFUNE_DATA_DIR");
        return;
    };
    let fx = Fixture::new();
    // 3.7e-6 mm のアルミ = 1e-6 g/cm²
    let thin = 1.0e-6 / 2.70 / 100.0;
    let out = compute(
        &input(Some(vec![thin, thin * 2.0, thin * 4.0])),
        &fx.tables(),
        &shield,
    )
    .expect("run");
    let measured = out.cases[0].table[0].protons.value();
    let (e_lo, e_hi) = (0.1f64, 400.0f64);
    let ln = |v: Vec<f64>| v.into_iter().map(f64::ln).collect::<Vec<_>>();
    let range = NaturalSpline::new(&ln(array("ep", 133)), &ln(array("rp", 133))).expect("spline");
    let tp = ln(array("tp", 49));
    let dratp = array("dratp", 11 * 49 * 51);
    let si_ratio: Vec<f64> = (0..49).map(|n| dratp[(2 * 49 + n) * 51]).collect();
    let ratio = sat_astro::math::spline::ExtendedLinear::new(&tp, &si_ratio).expect("ratio");
    let stopping = |e: f64| {
        let x = e.ln();
        e / range.eval(x).exp() / range.derivative(x)
    };
    let n = 20001;
    let step = (e_hi / e_lo).ln() / (n - 1) as f64;
    let mut sum = 0.0;
    for k in 0..n {
        let e = e_lo * (k as f64 * step).exp();
        let j = 10f64.powf(3.0 - 1.5 * (e - 0.1) / 399.9) * 1.5 * 10f64.ln() / 399.9;
        let w = if k == 0 || k == n - 1 {
            1.0
        } else if k % 2 == 1 {
            4.0
        } else {
            2.0
        };
        sum += w * j * stopping(e) * ratio.eval(e.ln()) * e;
    }
    let expected = sum * step / 3.0 * 1.6021892e-10;
    let rel = (measured - expected) / expected;
    eprintln!(
        "proton sphere dose near zero depth: {measured:e} Gy/s, hand integral {expected:e}, relative {rel:.4}"
    );
    assert!(rel.abs() < 0.015, "{rel}");
}

#[test]
fn table_has_the_default_depths_geometry_and_sources() {
    let Some(shield) = data() else {
        eprintln!("SKIPPED: shieldose2.dat is not in IWAFUNE_DATA_DIR");
        return;
    };
    let fx = Fixture::new();
    let out = compute(&input(None), &fx.tables(), &shield).expect("run");
    assert_eq!(out.cases.len(), 2);
    assert_eq!(out.cases[0].proton_model, "ap8min");
    assert_eq!(out.cases[1].electron_model, "ae8max");
    assert_eq!(out.cases[0].table.len(), DEFAULT_THICKNESSES_MM.len());
    let u = &out.used;
    assert_eq!(u.geometry, DoseGeometry::SphereCentre);
    assert_eq!(u.geometry_source, DoseSettingSource::Default);
    assert_eq!(u.detector, "silicon");
    assert_eq!(u.nuclear, NuclearAttenuation::None);
    assert_eq!(u.thickness_source, DoseSettingSource::Default);
    assert!(u.scope.contains("no solar energetic protons"));
    assert_eq!(u.spectra.len(), 4);
    // 解析の線量は厚さとともに減り、合計は 3 つの和。極大の地図は 1 桁多いので線量も約 10 倍
    for c in &out.cases {
        for p in &c.table {
            let sum = p.protons.value() + p.electrons.value() + p.bremsstrahlung.value();
            assert!((p.total.value() - sum).abs() <= 1e-12 * sum);
        }
        assert!(
            c.table
                .windows(2)
                .all(|w| w[1].total.value() <= w[0].total.value())
        );
    }
    let ratio = out.cases[1].table[2].total.value() / out.cases[0].table[2].total.value();
    assert!((ratio - 10.0).abs() < 1e-3, "{ratio}");
    // 最悪ケースの表は極大（ここでは常に大きい）の合計
    let worst = out.worst_case_table();
    assert!(!worst.is_empty());
    assert_eq!(worst[0].rate.value(), out.cases[1].table[0].total.value());
}

/// 球の中心は片面の板の線量の約 2 倍（浅い深さで）、`semi_infinite` と `finite_slab` の選択も出力に反映される。
#[test]
fn geometry_choice_changes_the_table() {
    let Some(shield) = data() else {
        eprintln!("SKIPPED: shieldose2.dat is not in IWAFUNE_DATA_DIR");
        return;
    };
    let fx = Fixture::new();
    let thin = vec![1.0e-3, 2.0e-3, 4.0e-3];
    let mut sphere = input(Some(thin.clone()));
    sphere.geometry = Some(DoseGeometry::SphereCentre);
    let mut slab = input(Some(thin));
    slab.geometry = Some(DoseGeometry::SemiInfiniteSlab);
    let a = compute(&sphere, &fx.tables(), &shield).expect("sphere");
    let b = compute(&slab, &fx.tables(), &shield).expect("slab");
    assert_eq!(b.used.geometry_source, DoseSettingSource::Given);
    let r = a.cases[0].table[0].protons.value() / b.cases[0].table[0].protons.value();
    eprintln!("sphere / slab proton dose at 1 mm: {r:.3}");
    assert!(r > 1.5 && r < 3.0, "{r}");
}

#[test]
fn inputs_are_checked() {
    let fx = Fixture::new();
    let Some(shield) = data() else {
        eprintln!("SKIPPED: shieldose2.dat is not in IWAFUNE_DATA_DIR");
        return;
    };
    assert!(compute(&input(Some(vec![1e-3, 2e-3])), &fx.tables(), &shield).is_err());
    assert!(compute(&input(Some(vec![1e-3, 1e-3, 2e-3])), &fx.tables(), &shield).is_err());
    assert!(compute(&input(Some(vec![-1e-3, 1e-3, 2e-3])), &fx.tables(), &shield).is_err());
    let empty = RadiationTables::default();
    let err = compute(&input(None), &empty, &shield)
        .unwrap_err()
        .to_string();
    assert!(err.contains("not installed"), "{err}");
}

#[test]
fn worst_case_table_takes_the_larger_total_and_stops_at_zero() {
    let p = |t: f64, total: f64| TrappedDosePoint {
        shielding: Qty::from_raw(t),
        protons: Qty::from_raw(total),
        electrons: Qty::ZERO,
        bremsstrahlung: Qty::ZERO,
        total: Qty::from_raw(total),
    };
    let case = |solar, table| TrappedDoseCase {
        solar,
        proton_model: "p".into(),
        electron_model: "e".into(),
        table,
    };
    let out = TrappedDoseOutput {
        cases: vec![
            case(
                SolarCondition::Minimum,
                vec![p(1e-3, 5.0), p(2e-3, 1.0), p(3e-3, 0.0)],
            ),
            case(
                SolarCondition::Maximum,
                vec![p(1e-3, 3.0), p(2e-3, 2.0), p(3e-3, 0.0)],
            ),
        ],
        used: TrappedDoseUsed {
            span: Qty::ZERO,
            step: Qty::ZERO,
            samples: 0,
            samples_without_l: 0,
            geometry: DoseGeometry::SphereCentre,
            geometry_source: DoseSettingSource::Default,
            detector: "silicon".into(),
            nuclear: NuclearAttenuation::None,
            nuclear_source: DoseSettingSource::Default,
            thickness_source: DoseSettingSource::Default,
            integration_points: 0,
            spectra: vec![],
            scope: String::new(),
        },
    };
    let t = out.worst_case_table();
    assert_eq!(t.len(), 2);
    assert_eq!((t[0].rate.value(), t[1].rate.value()), (5.0, 2.0));
}
