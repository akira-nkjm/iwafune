//! 日付の探索（`propulsion.launch_window`）のテスト。値の突き合わせ先は `propulsion.maneuvers`（同じ日付を書いたマヌーバ）と、
//! 円軌道・同一面のホーマン遷移の手計算（参考の範囲）。許容誤差と期待値は合わせて調整しない（外れたら理由つきで `#[ignore]`）。

use super::super::dated_tests::mars_input;
use super::super::plan::Orbit;
use super::super::tests::{input, maneuver, raise};
use super::super::{DatedTransferInput, DvCategory, Maneuvers, TransferMethod};
use super::*;
use sat_astro::constants::GM_SUN;
use sat_astro::ephem::CentralBody;
use sat_astro::time::Epoch;
use sat_units::typed::Time;

fn epoch(s: &str) -> Epoch {
    Epoch::parse(s).unwrap()
}

fn undated(incl_deg: f64) -> PropulsionInput {
    let mut i = mars_input(incl_deg, "1996-11-07T00:00:00Z");
    i.maneuvers[0].transfer = None;
    i
}

fn run(
    i: PropulsionInput,
    settings: LaunchWindowSettings,
) -> Result<LaunchWindowOutput, AnalysisError> {
    LaunchWindow::run(&LaunchWindowInput {
        propulsion: i,
        settings,
    })
}

fn layer(w: &LaunchWindowResult, kind: LaunchWindowQuantity) -> &[Option<f64>] {
    &w.grid
        .layers
        .iter()
        .find(|l| l.kind == kind)
        .unwrap()
        .values
}

/// 解けた点の添字 `(i, j)`（行 = 出発）。
fn solved_cells(w: &LaunchWindowResult) -> Vec<(usize, usize)> {
    let n = w.grid.arrivals.len();
    let dv = layer(w, LaunchWindowQuantity::DeltaV);
    (0..dv.len())
        .filter(|k| dv[*k].is_some())
        .map(|k| (k / n, k % n))
        .collect()
}

/// 天体間の遷移が無い設計（地球のまわりのマヌーバだけ）は空の結果（`propulsion.transfer_check` と同じ）。
#[test]
fn no_interplanetary_maneuver_gives_an_empty_result() {
    let out = run(
        input(vec![
            raise(),
            maneuver(
                "LOWER",
                DvCategory::Deorbit,
                TransferMethod::Impulsive,
                250.0,
            ),
        ]),
        LaunchWindowSettings::default(),
    )
    .unwrap();
    assert!(out.windows.is_empty());
}

/// 既定の軸: 窓 = 会合周期、飛行時間 = ホーマンの 0.5〜1.5 倍、始め = 元期（日付なし）。
/// 会合周期・ホーマンの時間は円軌道の手計算（半径は暦の表の長半径）。
#[test]
fn default_axes_follow_the_body_pair() {
    let i = undated(97.0);
    let epoch0 = i.initial_orbit.epoch;
    let out = run(i, LaunchWindowSettings::default()).unwrap();
    let w = &out.windows[0];
    let (r1, r2) = (
        CentralBody::Earth.orbit_radius_m().unwrap(),
        CentralBody::Mars.orbit_radius_m().unwrap(),
    );
    let hohmann = std::f64::consts::PI * ((0.5 * (r1 + r2)).powi(3) / GM_SUN).sqrt();
    let n = |r: f64| (GM_SUN / r.powi(3)).sqrt();
    let synodic = std::f64::consts::TAU / (n(r1) - n(r2)).abs();
    assert!((w.hohmann_time.value() - hohmann).abs() < 1.0);
    assert!((w.synodic_period.value() - synodic).abs() < 1.0);
    assert_eq!(w.span.departure_start, epoch0);
    let span = w.span.departure_end.seconds_since(w.span.departure_start);
    assert!((span - synodic).abs() <= 1.0, "{span}");
    assert!((w.span.time_of_flight_min.value() - 0.5 * hohmann).abs() <= 1.0);
    assert!((w.span.time_of_flight_max.value() - 1.5 * hohmann).abs() <= 1.0);
    assert_eq!(w.grid.departures.len(), DEFAULT_STEPS as usize);
    assert_eq!(w.grid.arrivals.len(), DEFAULT_STEPS as usize);
    for k in &w.grid.layers {
        assert_eq!(
            k.values.len(),
            w.grid.departures.len() * w.grid.arrivals.len()
        );
    }
    assert!(w.written.is_none());
    // ホーマンは約 259 日（記憶による参考値。検証していない）
    assert!((hohmann / 86_400.0 - 259.0).abs() < 2.0);
}

/// 日付が書かれていれば、書かれた出発の日が窓の真ん中になり、書かれた日付の値が `propulsion.maneuvers` と一致する。
#[test]
fn written_dates_centre_the_window_and_match_the_maneuvers_analysis() {
    let i = mars_input(97.0, "1996-11-07T00:00:00Z");
    let reference = Maneuvers::run(&i).unwrap().maneuvers[0].clone();
    let d = reference.dated_transfer.clone().unwrap();
    let out = run(i, LaunchWindowSettings::default()).unwrap();
    let w = &out.windows[0];
    let mid = w
        .span
        .departure_start
        .plus_seconds(0.5 * w.span.departure_end.seconds_since(w.span.departure_start));
    assert!((mid.seconds_since(epoch("1996-11-07T00:00:00Z"))).abs() <= 1.0);
    let wr = w.written.clone().unwrap();
    let p = wr.point.clone().unwrap();
    assert_eq!(p.c3.value(), d.c3.value());
    assert_eq!(p.v_inf_departure.value(), d.v_inf_departure.value());
    assert_eq!(p.v_inf_arrival.value(), d.v_inf_arrival.value());
    assert_eq!(
        p.departure_declination.value(),
        d.departure_declination.value()
    );
    assert_eq!(p.delta_v.value(), reference.ideal_delta_v.value());
    let min = w.minimum_delta_v.clone().unwrap();
    assert_eq!(
        wr.delta_v_above_minimum.unwrap().value(),
        p.delta_v.value() - min.delta_v.value()
    );
    assert_eq!(
        wr.c3_above_minimum.unwrap().value(),
        p.c3.value() - w.minimum_c3.clone().unwrap().c3.value()
    );
}

/// 格子の点ごとに、その日付をマヌーバに書いて `propulsion.maneuvers` を回した値と一致する（C3・v∞・赤緯・ΔV。相対 1e-12 以内）。
#[test]
fn grid_points_equal_the_maneuvers_analysis_for_those_dates() {
    let i = undated(97.0);
    let w = run(i.clone(), LaunchWindowSettings::default())
        .unwrap()
        .windows
        .remove(0);
    let cells = solved_cells(&w);
    assert!(cells.len() > 500, "{}", cells.len());
    let n = w.grid.arrivals.len();
    let (mut checked, mut max_rel) = (0, 0.0_f64);
    for (idx, (di, aj)) in cells.iter().enumerate() {
        // 解けた点を等間隔に 40 点ほど（先頭・末尾を含む）
        if idx % (cells.len() / 40).max(1) != 0 && idx + 1 != cells.len() {
            continue;
        }
        let mut j = i.clone();
        j.maneuvers[0].transfer = Some(DatedTransferInput {
            departure: w.grid.departures[*di],
            arrival: w.grid.arrivals[*aj],
            departure_by: Default::default(),
        });
        let m = Maneuvers::run(&j).unwrap().maneuvers.remove(0);
        let d = m.dated_transfer.unwrap();
        let k = di * n + aj;
        let pairs = [
            (LaunchWindowQuantity::C3, d.c3.value()),
            (
                LaunchWindowQuantity::VInfDeparture,
                d.v_inf_departure.value(),
            ),
            (LaunchWindowQuantity::VInfArrival, d.v_inf_arrival.value()),
            (
                LaunchWindowQuantity::DepartureDeclination,
                d.departure_declination.value(),
            ),
            (LaunchWindowQuantity::DeltaV, m.ideal_delta_v.value()),
        ];
        for (q, expected) in pairs {
            let got = layer(&w, q)[k].unwrap();
            let rel = (got - expected).abs() / expected.abs().max(1e-30);
            max_rel = max_rel.max(rel);
            assert!(rel <= 1e-12, "{q:?} at ({di},{aj}): {got} vs {expected}");
        }
        checked += 1;
    }
    assert!(checked >= 40, "{checked}");
    eprintln!("checked {checked} grid points, max relative difference {max_rel:e}");
}

/// 最小の点は格子の最小で、最小の点の値は格子の値と同じ。解けた点の数・解こうとした点の数は配列と合う。
#[test]
fn minimum_points_are_minima_of_the_grid() {
    let w = run(undated(97.0), LaunchWindowSettings::default())
        .unwrap()
        .windows
        .remove(0);
    let dv = layer(&w, LaunchWindowQuantity::DeltaV);
    let c3 = layer(&w, LaunchWindowQuantity::C3);
    let min = |v: &[Option<f64>]| v.iter().flatten().copied().fold(f64::INFINITY, f64::min);
    let (m_dv, m_c3) = (
        w.minimum_delta_v.clone().unwrap(),
        w.minimum_c3.clone().unwrap(),
    );
    assert_eq!(m_dv.delta_v.value(), min(dv));
    assert_eq!(m_c3.c3.value(), min(c3));
    assert!(dv.iter().flatten().all(|v| *v >= m_dv.delta_v.value()));
    assert!(c3.iter().flatten().all(|v| *v >= m_c3.c3.value()));
    let n = w.grid.arrivals.len();
    let at = |p: &LaunchWindowPoint, v: &[Option<f64>]| {
        let (i, j) = (
            w.grid
                .departures
                .iter()
                .position(|d| *d == p.departure)
                .unwrap(),
            w.grid
                .arrivals
                .iter()
                .position(|a| *a == p.arrival)
                .unwrap(),
        );
        v[i * n + j]
    };
    assert_eq!(at(&m_dv, dv), Some(m_dv.delta_v.value()));
    assert_eq!(at(&m_c3, c3), Some(m_c3.c3.value()));
    assert_eq!(w.solved_cells as usize, dv.iter().flatten().count());
    assert!(w.evaluated_cells >= w.solved_cells);
    assert!(w.ephemeris == EphemerisSource::Analytic);
    assert!(w.notes.contains(&LaunchWindowNote::AnalyticEphemeris));
}

/// 格子の最小のまわりを連続の日付で下り（コンパス探索）、格子の最小からどれだけ下がるかを、格子の間隔と曲率から決まる上限と比べる。
///
/// 上限の導出: 連続の最小 `x*`（なめらかな谷）に最も近い格子点 `x_g` は、各軸で半間隔以内（`|Δ_d| ≤ h_d/2`, `|Δ_a| ≤ h_a/2`）。
/// 2 次近似 `f(x_g) − f(x*) ≈ ½ Δᵀ H Δ ≤ ½ λ_max |Δ|²`、格子の最小 `≤ f(x_g)` なので
/// `格子の最小 − f(x*) ≤ λ_max (h_d² + h_a²) / 8`。`λ_max` は最小の点の 2 階差分から作った Hessian の
/// Gershgorin の上限 `max(|a|+|b|, |c|+|b|)`（`a`・`c` は軸ごとの 2 階差分、`b` は混合差分）。
#[test]
fn refining_around_the_minimum_gains_no_more_than_the_grid_bound() {
    let i = undated(97.0);
    let w = run(i.clone(), LaunchWindowSettings::default())
        .unwrap()
        .windows
        .remove(0);
    assert!(
        !w.notes.contains(&LaunchWindowNote::MinimumAtGridEdge),
        "{:?}",
        w.notes
    );
    let min = w.minimum_delta_v.clone().unwrap();
    let (di, aj) = (
        w.grid
            .departures
            .iter()
            .position(|d| *d == min.departure)
            .unwrap(),
        w.grid
            .arrivals
            .iter()
            .position(|a| *a == min.arrival)
            .unwrap(),
    );
    let n = w.grid.arrivals.len();
    let dv = layer(&w, LaunchWindowQuantity::DeltaV);
    let f = |i: usize, j: usize| dv[i * n + j].unwrap();
    let hd = w.grid.departures[1].seconds_since(w.grid.departures[0]);
    let ha = w.grid.arrivals[1].seconds_since(w.grid.arrivals[0]);
    let a = (f(di + 1, aj) - 2.0 * f(di, aj) + f(di - 1, aj)) / (hd * hd);
    let c = (f(di, aj + 1) - 2.0 * f(di, aj) + f(di, aj - 1)) / (ha * ha);
    let b = (f(di + 1, aj + 1) - f(di + 1, aj - 1) - f(di - 1, aj + 1) + f(di - 1, aj - 1))
        / (4.0 * hd * ha);
    let lambda = (a.abs() + b.abs()).max(c.abs() + b.abs());
    let bound = lambda * (hd * hd + ha * ha) / 8.0;
    // コンパス探索（制約の無い連続の日付。秒の丸めはしない）
    let from = Orbit::of(&i.initial_orbit).unwrap();
    let mut ev = Evaluator::new(from, &i.maneuvers[0]);
    let value = |ev: &mut Evaluator, d: f64, a: f64| {
        ev.at(min.departure.plus_seconds(d), min.arrival.plus_seconds(a))
            .map(|v| v.delta_v_m_s)
            .unwrap_or(f64::INFINITY)
    };
    let (mut x, mut best, mut step) = ((0.0_f64, 0.0_f64), value(&mut ev, 0.0, 0.0), hd.max(ha));
    while step > 1.0 {
        let mut improved = false;
        for (dx, dy) in [(step, 0.0), (-step, 0.0), (0.0, step), (0.0, -step)] {
            let v = value(&mut ev, x.0 + dx, x.1 + dy);
            if v < best {
                (x, best, improved) = ((x.0 + dx, x.1 + dy), v, true);
            }
        }
        if !improved {
            step *= 0.5;
        }
    }
    let gain = min.delta_v.value() - best;
    eprintln!(
        "grid min {:.3} m/s, refined {best:.3} m/s, gain {gain:.3} m/s, bound {bound:.3} m/s (offset {:.2} d, {:.2} d; lambda {lambda:e})",
        min.delta_v.value(),
        x.0 / 86_400.0,
        x.1 / 86_400.0
    );
    assert!(
        gain >= -1e-9,
        "the refined value is not below the grid minimum: {gain}"
    );
    assert!(
        gain <= bound,
        "gain {gain} m/s exceeds the bound {bound} m/s"
    );
}

/// 最小 C3 の下限と範囲（円軌道・同一面のホーマンとの比較。**参照ではなく範囲**）。
///
/// 下限（厳密）: 出発の日心距離 `r₁`、到着の天体の距離 `r₂` を決めると、`r₂` に届く遷移の出発の速さは
/// 最小エネルギー楕円（近点 `r₁`・遠点 `r₂`）の近点の速さ `v_H(r₁, r₂) = √(μ(2/r₁ − 2/(r₁+r₂)))` 以上。
/// 地球の速さ `v_E(r₁) = √(μ(2/r₁ − 1/a_E))` を引くと `v∞ ≥ v_H − v_E`（三角不等式）。`r₁ ∈ [a_E(1−e_E), a_E(1+e_E)]`、
/// `r₂ ∈ [a_M(1−e_M), a_M(1+e_M)]`（離心率は暦の表の J2000 の値 0.016711・0.093394）で最小を取る。
/// 上限（目安）: 同じ箱の上端でのホーマンの v∞ に、火星の軌道傾斜 1.85° の面外成分（`v_H sin i` の程度）を足した値。
/// 厳密な定理ではなく、外れたら理由を調べる（調整しない）。文献の値（最小 C3 は約 8.6〜9 km²/s²、記憶による参考値。検証していない）は
/// ここでは使わず、結果に印字するだけ。
#[test]
fn minimum_c3_lies_in_the_hohmann_range() {
    let (e_e, e_m) = (0.016_711_2, 0.093_394_1);
    let (a_e, a_m) = (
        CentralBody::Earth.orbit_radius_m().unwrap(),
        CentralBody::Mars.orbit_radius_m().unwrap(),
    );
    let v_h = |r1: f64, r2: f64| (GM_SUN * (2.0 / r1 - 2.0 / (r1 + r2))).sqrt();
    let v_e = |r1: f64| (GM_SUN * (2.0 / r1 - 1.0 / a_e)).sqrt();
    let v_inf_h = |r1: f64, r2: f64| (v_h(r1, r2) - v_e(r1)).max(0.0);
    let mut lower = f64::INFINITY;
    let mut upper = 0.0_f64;
    for r1 in [a_e * (1.0 - e_e), a_e, a_e * (1.0 + e_e)] {
        for r2 in [a_m * (1.0 - e_m), a_m, a_m * (1.0 + e_m)] {
            lower = lower.min(v_inf_h(r1, r2));
            upper = upper.max(v_inf_h(r1, r2));
        }
    }
    let upper = upper + v_h(a_e, a_m * (1.0 + e_m)) * 1.85_f64.to_radians().sin();
    let circular = v_inf_h(a_e, a_m).powi(2);
    // 1 会合周期の窓（始めは 1995-06-01。1996 年 11 月の機会が窓の中ほどに入る）
    let s = LaunchWindowSettings {
        departure_start: Some(epoch("1995-06-01T00:00:00Z")),
        ..LaunchWindowSettings::default()
    };
    let w = run(undated(97.0), s).unwrap().windows.remove(0);
    let c3 = w.minimum_c3.clone().unwrap().c3.value();
    eprintln!(
        "min C3 {:.3} km2/s2 (circular Hohmann {:.3}; hand range {:.3} to {:.3}) at {}",
        c3 / 1e6,
        circular / 1e6,
        lower.powi(2) / 1e6,
        upper.powi(2) / 1e6,
        w.minimum_c3.clone().unwrap().departure.to_iso()
    );
    assert!(
        c3 >= lower * lower,
        "{c3} below the lower bound {}",
        lower * lower
    );
    assert!(
        c3 <= upper * upper,
        "{c3} above the upper bound {}",
        upper * upper
    );
}

/// 暦の範囲（1800〜2050 年）にかかる窓: 範囲の外の点は `None` で、窓全体の誤りにはしない。
#[test]
fn dates_outside_the_ephemeris_are_unsolved_cells() {
    let s = LaunchWindowSettings {
        departure_start: Some(epoch("2049-06-01T00:00:00Z")),
        ..LaunchWindowSettings::default()
    };
    let w = run(undated(97.0), s).unwrap().windows.remove(0);
    assert!(
        w.solved_cells > 0 && w.solved_cells < w.evaluated_cells,
        "{} / {}",
        w.solved_cells,
        w.evaluated_cells
    );
    let dv = layer(&w, LaunchWindowQuantity::DeltaV);
    let n = w.grid.arrivals.len();
    let end = epoch("2051-01-01T00:00:00Z");
    for (k, v) in dv.iter().enumerate() {
        let (d, a) = (w.grid.departures[k / n], w.grid.arrivals[k % n]);
        if v.is_some() {
            assert!(d.seconds_since(end) < 0.0 && a.seconds_since(end) < 0.0);
        }
    }
    // すべて範囲の外なら `propulsion.maneuvers` と同じ種類の誤り
    let s = LaunchWindowSettings {
        departure_start: Some(epoch("2060-01-01T00:00:00Z")),
        ..LaunchWindowSettings::default()
    };
    let err = run(undated(97.0), s).unwrap_err();
    assert!(
        matches!(
            err,
            AnalysisError::Astro(sat_astro::AstroError::OutsideEphemeris { .. })
        ),
        "{err:?}"
    );
}

/// 設定の検査と上書き。
#[test]
fn settings_override_defaults_and_are_validated() {
    let i = undated(97.0);
    let q = |s: f64| Some(Qty::<Time>::from_raw(s));
    let day = 86_400.0;
    let s = LaunchWindowSettings {
        departure_start: Some(epoch("1996-10-01T00:00:00Z")),
        departure_span: q(60.0 * day),
        time_of_flight_min: q(200.0 * day),
        time_of_flight_max: q(320.0 * day),
        departure_steps: Some(7),
        arrival_steps: Some(9),
    };
    let w = run(i.clone(), s.clone()).unwrap().windows.remove(0);
    assert_eq!((w.grid.departures.len(), w.grid.arrivals.len()), (7, 9));
    assert_eq!(w.grid.departures[0], epoch("1996-10-01T00:00:00Z"));
    assert_eq!(w.grid.departures[6], epoch("1996-11-30T00:00:00Z"));
    assert_eq!(w.grid.arrivals[0], epoch("1997-04-19T00:00:00Z"));
    assert_eq!(w.grid.arrivals[8], epoch("1997-10-16T00:00:00Z"));
    let bad = |f: &dyn Fn(&mut LaunchWindowSettings)| {
        let mut t = s.clone();
        f(&mut t);
        assert!(matches!(
            run(i.clone(), t),
            Err(AnalysisError::InvalidInput(_))
        ));
    };
    bad(&|t| t.time_of_flight_max = q(100.0 * day));
    bad(&|t| t.departure_steps = Some(1));
    bad(&|t| t.departure_span = q(-1.0));
    bad(&|t| (t.departure_steps, t.arrival_steps) = (Some(1000), Some(1000)));
}

/// 軌道面: 出発の赤緯が駐機軌道の傾斜角を超える最小の点には注記が付く（`dated_transfer_plane` と同じ見方）。
/// 傾斜角 0° では赤緯の絶対値が 0 でない限り付き、傾斜角 97°（赤緯は ±90° 以内）では付かない。
#[test]
fn declination_above_the_parking_inclination_is_noted() {
    let w = run(undated(0.0), LaunchWindowSettings::default())
        .unwrap()
        .windows
        .remove(0);
    let dec = w
        .minimum_delta_v
        .unwrap()
        .departure_declination
        .value()
        .abs();
    assert!(dec > 0.0);
    assert!(
        w.notes
            .contains(&LaunchWindowNote::DeclinationAboveInclination)
    );
    let w = run(undated(97.0), LaunchWindowSettings::default())
        .unwrap()
        .windows
        .remove(0);
    assert!(
        !w.notes
            .contains(&LaunchWindowNote::DeclinationAboveInclination)
    );
}

/// 表の行（`points`）は `windows` の最小の 2 点と書かれた日付の点で、種類と ID が付く。
#[test]
fn points_table_lists_the_minimum_points_and_the_written_dates() {
    let out = run(
        mars_input(97.0, "1996-11-07T00:00:00Z"),
        LaunchWindowSettings::default(),
    )
    .unwrap();
    let w = &out.windows[0];
    let kinds: Vec<_> = out.points.iter().map(|p| p.kind).collect();
    assert_eq!(
        kinds,
        [
            LaunchWindowPointKind::MinDeltaV,
            LaunchWindowPointKind::MinC3,
            LaunchWindowPointKind::Written
        ]
    );
    assert!(out.points.iter().all(|p| p.maneuver == "MARS"));
    assert_eq!(Some(&out.points[0]), w.minimum_delta_v.as_ref());
    assert_eq!(Some(&out.points[1]), w.minimum_c3.as_ref());
    assert_eq!(
        Some(&out.points[2]),
        w.written.as_ref().unwrap().point.as_ref()
    );
    let none = run(undated(97.0), LaunchWindowSettings::default()).unwrap();
    assert_eq!(none.points.len(), 2);
}
