//! SHIELDOSE-2 の読み取りと線量の計算の試験。手計算・文献の値と比べる。配布ファイル（`shieldose2.dat`）が要る試験は、
//! キャッシュに無ければ読み飛ばす（`docs/trapped-radiation.md`）。

use super::dose::sphere_from_slab;
use super::*;
use crate::math::spline::NaturalSpline;
use crate::test_data::cached_bytes;

fn real() -> Option<ShieldingData> {
    cached_bytes("shieldose2.dat").map(|b| ShieldingData::from_bytes(&b).expect("shieldose2.dat"))
}

/// 必須の配列をすべて持つ小さな合成のファイル（値はすべて 1）。`skip` の配列は書かない。
fn synthetic(skip: &str, extra: &str) -> String {
    let dims: &[(&str, &[usize])] = &[
        ("ep", &[133]),
        ("rp", &[133]),
        ("tepn", &[28]),
        ("fepn", &[28]),
        ("tp", &[49]),
        ("zrp", &[51]),
        ("dalp", &[2, 49, 51]),
        ("dratp", &[11, 49, 51]),
        ("ee", &[81]),
        ("re", &[81]),
        ("ye", &[81]),
        ("te", &[14]),
        ("ar", &[14]),
        ("rs", &[14]),
        ("bs", &[34]),
        ("zre", &[51]),
        ("zs", &[37]),
        ("zb", &[47]),
        ("dale", &[14, 34]),
        ("dalb", &[14, 37]),
        ("drate", &[11, 2, 14, 51]),
        ("dratb", &[11, 2, 14, 47]),
    ];
    let mut out = String::from("# test\ntag proton p\ntag electron e\n");
    for (name, d) in dims {
        if *name == skip {
            continue;
        }
        let n: usize = d.iter().product();
        let dims_text: Vec<String> = d.iter().map(ToString::to_string).collect();
        out += &format!(
            "array {name} {}\n{}\n",
            dims_text.join(" "),
            vec!["1.0"; n].join(" ")
        );
    }
    out + extra + "end\n"
}

#[test]
fn reader_accepts_a_complete_file_and_rejects_broken_ones() {
    let ok = ShieldingData::from_bytes(synthetic("", "").as_bytes()).unwrap();
    assert_eq!((ok.proton_tag(), ok.electron_tag()), ("p", "e"));
    assert_eq!(ok.vector("ep").len(), 133);
    assert_eq!(ok.row("drate", &[10, 1, 13]).len(), 51);
    assert!(ok.row("drate", &[11, 0, 0]).is_empty());
    // 配列が無い・個数が違う・名前が違う・end が無い・end の後ろにデータ・数でない値
    assert!(ShieldingData::from_bytes(synthetic("dratb", "").as_bytes()).is_err());
    let short = synthetic("", "").replacen("array ep 133\n1.0 ", "array ep 133\n", 1);
    assert!(ShieldingData::from_bytes(short.as_bytes()).is_err());
    assert!(ShieldingData::from_bytes(synthetic("", "array zzz 1\n1.0\n").as_bytes()).is_err());
    let no_end = synthetic("", "").replace("end\n", "");
    assert!(ShieldingData::from_bytes(no_end.as_bytes()).is_err());
    let after = synthetic("", "") + "# more\nextra\n";
    assert!(ShieldingData::from_bytes(after.as_bytes()).is_err());
    let nan = synthetic("", "").replacen("1.0", "abc", 1);
    assert!(ShieldingData::from_bytes(nan.as_bytes()).is_err());
    let wrong_dims = synthetic("", "").replace("array tp 49", "array tp 48");
    assert!(ShieldingData::from_bytes(wrong_dims.as_bytes()).is_err());
}

/// 変換した値と、IRBEM のソース（`prot_and_elbr_bas2.f`）の DATA 文の値の抜き取り照合（変換器とは別の方法で全 1297 文を照合済み。
/// ここには代表を書く）。順序（最後の添字が速い・検出器と `m` の位置）が正しいことの確認。
#[test]
fn converted_values_match_the_source_data_statements() {
    let Some(d) = real() else { return };
    assert!(d.proton_tag().contains("ICRU-49"));
    assert!(d.electron_tag().contains("ETRAN"));
    assert_eq!(d.vector("ep")[0], 0.001);
    assert_eq!(d.vector("rp")[132], 5757.0);
    assert_eq!(d.vector("fepn")[27], 0.2970);
    assert_eq!(d.vector("bs")[33], 2.0);
    assert_eq!(d.row("dalp", &[0, 0])[0], 3.525);
    assert_eq!(d.row("dratp", &[10, 48])[50], 1.279); // DRATP(49, 11, 51)
    assert_eq!(d.row("dale", &[13])[33], 1.0e-7);
    assert_eq!(d.row("dalb", &[13])[36], 1.0e-35); // 0.1000D-34
    assert_eq!(d.row("drate", &[10, 1, 13])[50], 0.9943); // DRATE(14, 11, 51, 2)
    assert_eq!(d.row("dratb", &[10, 1, 13])[46], 0.9594); // DRATB(14, 11, 47, 2)
    assert_eq!(d.row("dratb", &[10, 0, 13])[46], 0.9699); // DRATB(14, 11, 47, 1)
    assert_eq!(d.row("drate", &[0, 1, 0])[0], 0.6326); // DRATE(1, 1, 1, 2)
}

fn model(d: &ShieldingData, det: Detector, nuc: NuclearOption) -> ShieldingModel {
    ShieldingModel::new(d, det, nuc).expect("model")
}

/// 100 MeV 付近の細い帯（99〜101 MeV で一定）の陽子を、アルミの浅い所（0.01 g/cm²。飛程 約 10 g/cm² よりずっと浅い）で。
/// 手計算: 片面の板の線量 = φ/2 × 質量阻止能 × 1.6021892e-10 Gy g/MeV（`φ` は全方向のフルエンス率、1/2 は片面の半空間、
/// 斜入射の経路の延びの平均 2 を含む。モジュール `dose` の説明）。阻止能はデータの飛程の表の dE/dR（ln-ln のスプラインの微分）。
#[test]
fn proton_dose_near_the_surface_is_half_the_flux_times_the_stopping_power() {
    let Some(d) = real() else { return };
    let grid = EnergyGrid::new(99.0, 101.0, 201).unwrap();
    let spec = GridSpectrum::from_table(grid, &[99.0, 100.0, 101.0], &[1.0, 1.0, 1.0]).unwrap();
    let phi = spec.integral_flux();
    assert!((phi - 2.0).abs() < 1e-6, "{phi}");
    let ln = |v: &[f64]| v.iter().map(|x| x.ln()).collect::<Vec<_>>();
    let range = NaturalSpline::new(&ln(d.vector("ep")), &ln(d.vector("rp"))).unwrap();
    let ln_e = 100.0f64.ln();
    let stopping = range.eval(ln_e).exp().recip() * 100.0 / range.derivative(ln_e);
    // 文献: 100 MeV の陽子のアルミでの質量阻止能は 約 5.7 MeV cm²/g（ICRU Report 49・NIST PSTAR）
    assert!((stopping - 5.7).abs() / 5.7 < 0.03, "{stopping}");
    let doses = model(&d, Detector::Aluminum, NuclearOption::None)
        .proton_dose(&spec, &[0.01, 0.02, 0.05])
        .unwrap();
    let expected = 0.5 * phi * stopping * GY_PER_MEV_PER_G;
    let rel = (doses.semi_infinite[0] - expected) / expected;
    eprintln!(
        "surface dose {} Gy/s, expected {expected}, relative {rel:.4}",
        doses.semi_infinite[0]
    );
    assert!(rel.abs() < 0.03, "{rel}");
    // 球の中心の「半分の線量」も深さ 0 の極限では同じ（直進近似で D(1 − dlnD/dlnz) → D）。実際の線量はその 2 倍。
    let sphere = doses.sphere_half[0];
    assert!(
        (sphere / doses.semi_infinite[0] - 1.0).abs() < 0.02,
        "{sphere}"
    );
    assert_eq!(doses.sphere_centre()[0], 2.0 * sphere);
    assert_eq!(doses.finite_slab, doses.semi_infinite);
}

/// 深さ 0 の極限の核は、単一エネルギーごとに（板の線量 = φ/2 × 阻止能）。0.1 MeV〜1 GeV の 9 点で、データの飛程の表の dE/dR と 1 % 以内
/// （実測のばらつきは ±0.6 %。核の表（深さ線量）と飛程の表の導関数は別々に作られた表なので、完全には一致しない）。
#[test]
fn proton_kernel_matches_the_range_table_derivative_across_energies() {
    let Some(d) = real() else { return };
    let ln = |v: &[f64]| v.iter().map(|x| x.ln()).collect::<Vec<_>>();
    let range = NaturalSpline::new(&ln(d.vector("ep")), &ln(d.vector("rp"))).unwrap();
    let m = model(&d, Detector::Aluminum, NuclearOption::None);
    for e in [0.1f64, 0.3, 1.0, 3.0, 10.0, 30.0, 100.0, 300.0, 1000.0] {
        let grid = EnergyGrid::new(e * 0.99, e * 1.01, 201).unwrap();
        let spec =
            GridSpectrum::from_table(grid, &[e * 0.99, e, e * 1.01], &[1.0, 1.0, 1.0]).unwrap();
        let x = e.ln();
        let s = e / range.eval(x).exp() / range.derivative(x);
        let dose = m
            .proton_dose(&spec, &[1e-6, 2e-6, 4e-6])
            .unwrap()
            .semi_infinite[0];
        let expected = 0.5 * spec.integral_flux() * s * GY_PER_MEV_PER_G;
        assert!(
            (dose / expected - 1.0).abs() < 0.01,
            "E {e}: {}",
            dose / expected
        );
    }
}

/// 飛程が遮蔽より短い陽子（10 MeV の飛程は約 0.17 g/cm²）は、深さ 1 g/cm² の板では線量 0。
#[test]
fn protons_that_cannot_cross_the_shield_give_zero_dose() {
    let Some(d) = real() else { return };
    let grid = EnergyGrid::new(9.0, 11.0, 201).unwrap();
    let spec = GridSpectrum::from_table(grid, &[9.0, 10.0, 11.0], &[1.0, 1.0, 1.0]).unwrap();
    let m = model(&d, Detector::Silicon, NuclearOption::None);
    let doses = m.proton_dose(&spec, &[0.05, 0.1, 1.0, 2.0]).unwrap();
    assert!(doses.semi_infinite[0] > 0.0 && doses.semi_infinite[1] > 0.0);
    assert_eq!(doses.semi_infinite[2], 0.0);
    assert_eq!(doses.semi_infinite[3], 0.0);
    assert_eq!(doses.sphere_half[3], 0.0);
}

/// 球と板: 板の線量が `D = A z^-p`（べき乗）なら `dlnD/dlnz = -p` が解析的で、球の「半分の線量」は `D (1 + p)`。
/// スプラインは ln-ln で直線を厳密に通すので、全深さで一致する。
#[test]
fn sphere_from_a_power_law_slab_curve_matches_the_analytic_derivative() {
    let z = [0.01, 0.03, 0.1, 0.3, 1.0, 3.0];
    let ln_z: Vec<f64> = z.iter().map(|v: &f64| v.ln()).collect();
    let p = 1.7;
    let slab: Vec<f64> = z.iter().map(|v: &f64| 4.0 * v.powf(-p)).collect();
    let sphere = sphere_from_slab(&ln_z, &slab);
    for (s, d) in sphere.iter().zip(&slab) {
        assert!((s / d - (1.0 + p)).abs() < 1e-12, "{s} {d}");
    }
    // ln D が ln z の 2 次式（dlnD/dlnz = -1 - 0.2 ln z）でも、細かい格子の内部では 1e-3 以内
    let zs: Vec<f64> = (0..41).map(|k| 0.01 * 1.2f64.powi(k)).collect();
    let lz: Vec<f64> = zs.iter().map(|v| v.ln()).collect();
    let slab: Vec<f64> = lz.iter().map(|l| (-l - 0.1 * l * l).exp()).collect();
    let sphere = sphere_from_slab(&lz, &slab);
    for i in 5..35 {
        let expected = slab[i] * (2.0 + 0.2 * lz[i]);
        assert!((sphere[i] / expected - 1.0).abs() < 1e-3, "{i}");
    }
    // 線量が 0 になった深さから先は 0。正の深さが 3 点未満なら全部 0
    let cut = sphere_from_slab(&ln_z, &[3.0, 2.0, 1.0, 0.0, 0.0, 0.0]);
    assert!(cut[0] > 0.0 && cut[2] > 0.0 && cut[3..].iter().all(|v| *v == 0.0));
    assert!(
        sphere_from_slab(&ln_z, &[1.0, 0.5, 0.0, 0.0, 0.0, 0.0])
            .iter()
            .all(|v| *v == 0.0)
    );
}

/// NBS TN 1116 の付録 C の例（静止軌道の太陽陽子: 指数分布 E0 = 26.5 MeV、フルエンス C = 2.45e10、範囲 2〜1000 MeV）の印刷値
/// `INT SPEC 2.2719E+10`・`EAV 28.50006`。厳密な値は積分フラックス `C (e^{-2/E0} − e^{-1000/E0})` と、平均エネルギー 28.5 MeV。
#[test]
fn exponential_spectrum_reproduces_the_tn1116_sample_check_values() {
    let (e0, c): (f64, f64) = (26.5, 2.45e10);
    let exact_int = c * ((-2.0 / e0).exp() - (-1000.0 / e0).exp());
    for points in [201, 501, 1001] {
        let grid = EnergyGrid::new(2.0, 1000.0, points).unwrap();
        let s = GridSpectrum::exponential(grid, 2.0, 1000.0, e0, c);
        let (i, eav) = (s.integral_flux(), s.mean_energy());
        eprintln!("NPTS {points}: INT SPEC {i:.5e} (exact {exact_int:.5e}) EAV {eav:.6}");
        assert!((i / 2.2719e10 - 1.0).abs() < 1e-4, "{i}");
        assert!((i / exact_int - 1.0).abs() < 1e-5, "{i}");
        assert!((eav - 28.5).abs() < 1e-4, "{eav}");
    }
}

/// 積分フラックス → 微分フラックス: べき乗 `F = A E^-g` の微分は `A g E^-(g+1)`。刻み比 1.1 の対数格子で 0.2 % 以内。
/// 上端で 0 になる積分フラックスは、正の差分が続く範囲だけを返す。
#[test]
fn differential_from_a_power_law_integral_flux_and_its_cut_off() {
    let (a, g) = (1.0e6, 1.5);
    let es: Vec<f64> = (0..60).map(|k| 0.1 * 1.1f64.powi(k)).collect();
    let f: Vec<f64> = es.iter().map(|e| a * e.powf(-g)).collect();
    let (e, j) = differential_from_integral(&es, &f);
    assert_eq!(e.len(), es.len());
    assert_eq!(e[0], es[0]);
    for (x, y) in e.iter().zip(&j).skip(1) {
        let exact = a * g * x.powf(-g - 1.0);
        assert!((y / exact - 1.0).abs() < 2e-3, "{x}: {y} vs {exact}");
    }
    // 先頭の点は直線の延長なので、べき乗では厳密に近い
    assert!((j[0] / (a * g * es[0].powf(-g - 1.0)) - 1.0).abs() < 2e-3);
    let mut cut = f.clone();
    for v in cut.iter_mut().skip(40) {
        *v = 0.0;
    }
    let (e2, _) = differential_from_integral(&es, &cut);
    assert_eq!(e2.len(), 41); // F が 0 になる区間までの 40 区間 + 先頭の延長
    assert!(*e2.last().unwrap() < es[40]);
    assert!(
        differential_from_integral(&es, &vec![0.0; es.len()])
            .0
            .is_empty()
    );
}

/// 電子: アルミの深さが電子の飛程を十分越えれば電子の線量は 0、制動放射は残る。電子の線量は深さとともに単調に減る。
#[test]
fn electrons_stop_in_thick_shields_but_bremsstrahlung_remains() {
    let Some(d) = real() else { return };
    let grid = EnergyGrid::new(1.0, 2.0, 201).unwrap();
    let spec = GridSpectrum::from_table(grid, &[1.0, 1.5, 2.0], &[1.0, 1.0, 1.0]).unwrap();
    let m = model(&d, Detector::Silicon, NuclearOption::None);
    let depths = [0.05, 0.2, 0.5, 1.0, 3.0, 10.0];
    let doses = m.electron_dose(&spec, &depths).unwrap();
    let e = &doses.electrons.semi_infinite;
    assert!(e[0] > e[1] && e[1] > e[2] && e[2] > e[3], "{e:?}");
    assert_eq!(e[4], 0.0); // 2 MeV の電子の飛程は約 1.2 g/cm²
    assert_eq!(e[5], 0.0);
    let b = &doses.bremsstrahlung.semi_infinite;
    assert!(b.iter().all(|v| *v > 0.0), "{b:?}");
    assert!(b[0] < e[0]);
}

/// 核相互作用の選択: 選択 3 は 2 に中性子のエネルギーの項を足すので 2 より大きい。選択 2 と 1 の大小は決まっていない
/// （選択 2 は陽子が核反応で減るが、荷電の二次粒子のエネルギーはその場に付与するので、深い所で 1 より大きくなりうる）ので、値を出すだけ。
#[test]
fn nuclear_options_order_the_proton_dose() {
    let Some(d) = real() else { return };
    let grid = EnergyGrid::new(150.0, 250.0, 201).unwrap();
    let spec = GridSpectrum::from_table(grid, &[150.0, 200.0, 250.0], &[1.0, 1.0, 1.0]).unwrap();
    let dose = |n| {
        model(&d, Detector::Aluminum, n)
            .proton_dose(&spec, &[1.0, 5.0])
            .unwrap()
            .semi_infinite[1]
    };
    let (d1, d2, d3) = (
        dose(NuclearOption::None),
        dose(NuclearOption::Local),
        dose(NuclearOption::LocalWithNeutrons),
    );
    eprintln!("INUC 1/2/3: {d1:e} {d2:e} {d3:e}");
    assert!(d1 > 0.0 && d2 > 0.0 && d3 > d2, "{d1} {d2} {d3}");
}

#[test]
fn depths_must_increase_and_grids_must_be_valid() {
    let Some(d) = real() else { return };
    let grid = EnergyGrid::new(99.0, 101.0, 21).unwrap();
    let spec = GridSpectrum::from_table(grid, &[99.0, 100.0, 101.0], &[1.0, 1.0, 1.0]).unwrap();
    let m = model(&d, Detector::Silicon, NuclearOption::None);
    assert!(m.proton_dose(&spec, &[1.0, 0.5]).is_err());
    assert!(m.proton_dose(&spec, &[]).is_err());
    assert!(EnergyGrid::new(1.0, 1.0, 10).is_err());
    assert!(GridSpectrum::from_table(grid, &[1.0, 2.0], &[1.0, 1.0]).is_err());
    assert!(GridSpectrum::from_table(grid, &[1.0, 2.0, 3.0], &[1.0, 0.0, 1.0]).is_err());
    assert_eq!(Detector::Silicon.index(), 2);
}
