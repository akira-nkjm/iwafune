//! 捕捉粒子のフラックス解析の試験。合成のマップ（同じ形式の小さなもの）と中心双極子の場で、赤道面の円軌道の値を手計算と比べる。
//! 実データでの確認は `crates/app/service/tests/trapped_flux.rs`（配布ファイルがあるときだけ）。

use super::*;
use sat_astro::constants::R_EARTH;
use sat_astro::geomag::{GaussModel, GaussTerm, REFERENCE_RADIUS_M as A};
use sat_astro::orbit::{OrbitSpec, Propagator};
use sat_astro::time::Epoch;

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

/// 合成のマップ。倍率はすべて 100、1/4 桁刻み。エネルギー 0.1 MeV と 400 MeV のサブマップで、L = 1 と L = 3 の B/B₀ = 1 の log₁₀F は
/// 0.1 MeV で `top` と `top − 2`、400 MeV で `top − 2` と `top − 3`（B/B₀ の増分は 1 つだけ: 位置 1 → 2）。
fn map(top: i64) -> TrappedMap {
    let sub = |energy: i64, f1: i64, f3: i64| {
        let mut body = vec![0, energy, 3, 0, 0, 3, 90, 0];
        body.extend([5, 100, f1, 100, 100]);
        body.extend([5, 300, f3, 100, 100]);
        // 実データと同じく、最後の節点の先はフラックス無しの節点（L = 3.2）
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

/// 軸が z の中心双極子（`g10` は符号つきの赤道強度。B/B₀ = 1 になる）。
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

/// 赤道面（傾斜 0）の円軌道。半径 `l`·a で、中心双極子の L = l、B/B₀ = 1 が一定。
fn equatorial_input(l: f64, span_s: f64) -> TrappedFluxInput {
    let epoch = Epoch::from_calendar(2027, 4, 1, 0, 0, 0.0);
    let mut orbit =
        OrbitSpec::circular(epoch, Qty::from_raw(l * A - R_EARTH), Qty::ZERO, Qty::ZERO);
    orbit.propagator = Propagator::Kepler;
    TrappedFluxInput {
        sampling: OrbitSampling::new(orbit, Qty::from_raw(span_s), Qty::from_raw(600.0)),
        mission_duration: Qty::from_raw(1.0e7),
    }
}

struct Fixture {
    maps: [TrappedMap; 4],
    fields: [GaussModel; 2],
}

impl Fixture {
    /// 陽子・電子の極小は `top = 4`、極大は 1 桁上（`top = 5`）。
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

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs()
}

/// L = 2 の赤道面の円軌道: 0.1 MeV は log₁₀F = top − 1（L = 1 と 3 の中間）、400 MeV は top − 2.5。
/// 途中のエネルギーは E の直線補間。フラックスは cm⁻² s⁻¹ から m⁻² s⁻¹（×1e4）に直し、フルエンス = 平均 × 期間。
#[test]
fn equatorial_circular_orbit_matches_hand_calculation() {
    let fx = Fixture::new();
    let out = compute(&equatorial_input(2.0, 7200.0), &fx.tables()).expect("run");
    assert_eq!(out.cases.len(), 4);
    let ids: Vec<_> = out.cases.iter().map(|c| c.model.as_str()).collect();
    assert_eq!(ids, ["ap8min", "ap8max", "ae8min", "ae8max"]);
    let ap_min = &out.cases[0];
    assert_eq!(
        (ap_min.species, ap_min.solar),
        (Species::Proton, SolarCondition::Minimum)
    );
    // 0.1 MeV: top = 4 → 10^3 cm⁻² s⁻¹ = 1e7 m⁻² s⁻¹
    let first = &ap_min.spectrum[0];
    assert_eq!(first.energy_mev, 0.1);
    assert!(
        near(first.mean_flux.value(), 1.0e7, 1e-4),
        "{:?}",
        first.mean_flux
    );
    assert!(near(first.peak_flux.value(), 1.0e7, 1e-4));
    assert!(near(first.fluence.value(), 1.0e7 * 1.0e7, 1e-4));
    assert_eq!(first.time_fraction.value(), 1.0);
    // 400 MeV（最後の節点）: 10^(4 − 2.5) = 31.62 cm⁻² s⁻¹
    let last = ap_min.spectrum.last().expect("last");
    assert_eq!(last.energy_mev, 400.0);
    assert!(
        near(last.mean_flux.value(), 10f64.powf(1.5) * 1e4, 1e-4),
        "{:?}",
        last.mean_flux
    );
    // 10 MeV: 0.1 MeV の log 3 と 400 MeV の log 1.5 を E で直線補間（t = 9.9/399.9）
    let t = 9.9 / 399.9;
    let ten = ap_min
        .spectrum
        .iter()
        .find(|p| p.energy_mev == 10.0)
        .expect("10 MeV");
    assert!(near(
        ten.mean_flux.value(),
        10f64.powf(3.0 - 1.5 * t) * 1e4,
        1e-4
    ));
    // 太陽活動極大のマップは 1 桁上
    let ap_max = &out.cases[1];
    assert!(near(ap_max.spectrum[0].mean_flux.value(), 1.0e8, 1e-4));
    // 組にした場の名前: AP-8 MAX だけが 2 つめの場
    assert_eq!(out.cases[0].field, "test JC");
    assert_eq!(out.cases[1].field, "test GSFC");
    assert_eq!(out.cases[3].field, "test JC");
    // L は 2、サンプルはすべてマップの中
    assert!(near(ap_min.l_min.expect("l"), 2.0, 1e-4) && near(ap_min.l_max.expect("l"), 2.0, 1e-4));
    assert_eq!(ap_min.samples_without_l, 0);
    assert_eq!(out.used.samples, 13);
    assert_eq!(ap_min.epoch_year, 1970);
}

/// 電子のエネルギーの格子（0.04〜7 MeV）はマップの最小のエネルギー（0.1 MeV）より下を、最初の 2 つのサブマップからの直線で外挿する（NASA の手順）。
#[test]
fn electron_grid_starts_below_the_first_node() {
    let fx = Fixture::new();
    let out = compute(&equatorial_input(2.0, 1200.0), &fx.tables()).expect("run");
    let ae = &out.cases[2];
    assert_eq!(ae.species, Species::Electron);
    assert_eq!(ae.spectrum.len(), electron_energies_mev().len());
    // log₁₀F（cm⁻² s⁻¹）= 3 − 1.5·(0.04 − 0.1)/399.9
    let expected = 10f64.powf(3.0 + 1.5 * 0.06 / 399.9) * 1e4;
    assert!(
        near(ae.spectrum[0].mean_flux.value(), expected, 1e-3),
        "{:?}",
        ae.spectrum[0].mean_flux
    );
}

/// L = 3.5（フラックスのある最後の節点 L = 3 と、その先のフラックス無しの節点 L = 3.2 の外）はマップの外としてフラックス 0。時間の割合 0。
#[test]
fn orbit_outside_the_map_has_no_flux() {
    let fx = Fixture::new();
    let out = compute(&equatorial_input(3.5, 1200.0), &fx.tables()).expect("run");
    let first = &out.cases[0].spectrum[0];
    assert_eq!(first.mean_flux.value(), 0.0);
    assert_eq!(first.peak_flux.value(), 0.0);
    assert_eq!(first.time_fraction.value(), 0.0);
}

#[test]
fn missing_tables_and_bad_duration_are_reported() {
    let none = RadiationTables::default();
    let err = compute(&equatorial_input(2.0, 1200.0), &none).expect_err("no tables");
    let text = err.to_string();
    assert!(
        text.contains("not installed") && text.contains("ap8min"),
        "{text}"
    );
    let fx = Fixture::new();
    let mut input = equatorial_input(2.0, 1200.0);
    input.mission_duration = Qty::from_raw(0.0);
    assert!(compute(&input, &fx.tables()).is_err());
}

#[test]
fn standard_grids_lie_inside_the_model_energy_ranges() {
    assert_eq!(
        (
            proton_energies_mev()[0],
            *proton_energies_mev().last().expect("p")
        ),
        (0.1, 400.0)
    );
    assert_eq!(
        (
            electron_energies_mev()[0],
            *electron_energies_mev().last().expect("e")
        ),
        (0.04, 7.0)
    );
    for grid in [proton_energies_mev(), electron_energies_mev()] {
        assert!(grid.windows(2).all(|w| w[0] < w[1]));
    }
    for needed in [10.0, 30.0, 100.0] {
        assert!(proton_energies_mev().contains(&needed));
    }
    for needed in [0.5, 1.0, 2.0] {
        assert!(electron_energies_mev().contains(&needed));
    }
}

/// サンプルを塊に分けて並列に求めた (L, B/B₀) は、直列に 1 点ずつ求めた値と一致する（ビットまで）。
#[test]
fn threaded_positions_equal_serial_positions() {
    let fx = Fixture::new();
    let input = equatorial_input(2.0, 86_400.0);
    let points = input.sampling.states().expect("states");
    let field = fx.fields[0].at(fx.fields[0].epoch());
    let threaded = positions_in(&field, &points, BeltConvention::default());
    for (p, got) in points.iter().zip(&threaded) {
        let serial = belt_position_with(
            &field,
            eci_to_ecef(p.state.r_eci, p.epoch),
            BeltConvention::default(),
        );
        assert_eq!(&serial, got);
    }
}
