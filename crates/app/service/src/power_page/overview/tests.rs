use crate::Workspace;
use crate::dto::{AnalysisOverview, KpiKey, PowerPageView};
use std::path::Path;

fn page(case: Option<&str>) -> PowerPageView {
    let ws =
        Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
            .unwrap();
    crate::power_page::power_page(&ws, case).unwrap()
}

fn of<'a>(page: &'a PowerPageView, id: &str) -> &'a AnalysisOverview {
    &page
        .analyses
        .iter()
        .find(|a| a.result.id == id)
        .unwrap()
        .overview
}

fn bar_keys(o: &AnalysisOverview) -> Vec<&str> {
    o.chart
        .as_ref()
        .unwrap()
        .bars
        .iter()
        .map(|b| b.key.as_str())
        .collect()
}

#[test]
fn every_power_analysis_has_main_results_and_a_chart() {
    for case in [None, Some("hot_imaging")] {
        let page = page(case);
        for a in &page.analyses {
            // 主な結果は結果の KPI の先頭（定義の順に 3 つまで）
            let n = a.result.kpis.len().min(3);
            assert_eq!(a.overview.main, a.result.kpis[..n], "{}", a.result.id);
            let chart = a.overview.chart.as_ref().unwrap();
            assert!(!chart.bars.is_empty(), "{}", a.result.id);
            for b in &chart.bars {
                assert!((0.0..=1.0).contains(&b.fill), "{} {}", a.result.id, b.key);
            }
        }
        let keys: Vec<_> = of(&page, "power.energy_balance")
            .main
            .iter()
            .map(|k| k.key)
            .collect();
        assert_eq!(keys, [KpiKey::OrbitBalance, KpiKey::MaxDod]);
    }
}

#[test]
fn mode_load_bars_scale_to_the_largest_mode() {
    let page = page(None);
    let o = of(&page, "power.mode_load");
    assert_eq!(bar_keys(o), ["NOMINAL", "MISSION", "DOWNLINK", "SAFE"]);
    let bars = &o.chart.as_ref().unwrap().bars;
    let largest = bars
        .iter()
        .map(|b| b.value.as_ref().unwrap().si)
        .fold(0.0, f64::max);
    for b in bars {
        let v = b.value.as_ref().unwrap().si;
        assert!((b.fill - v / largest).abs() < 1e-12);
    }
    // 手で: デモの DOWNLINK が最大（17.36 W）で棒は満ちる
    let downlink = bars.iter().find(|b| b.key == "DOWNLINK").unwrap();
    assert_eq!(downlink.fill, 1.0);
    assert!((downlink.value.as_ref().unwrap().si - 17.35775).abs() < 1e-9);
}

#[test]
fn orbit_generation_shows_bol_eol_and_sunlit_fraction() {
    let page = page(None);
    let o = of(&page, "power.orbit_generation");
    assert_eq!(bar_keys(o), ["bol", "eol", "sunlit_fraction"]);
    let bars = &o.chart.as_ref().unwrap().bars;
    // BOL ≥ EOL なので BOL が満ち、EOL はその比
    assert_eq!(bars[0].fill, 1.0);
    let ratio = bars[1].value.as_ref().unwrap().si / bars[0].value.as_ref().unwrap().si;
    assert!((bars[1].fill - ratio).abs() < 1e-12);
    // 日照率は値そのもの（デモは 78.4 %）
    assert!((bars[2].fill - 0.784).abs() < 1e-3);
    assert_eq!(bars[2].value.as_ref().unwrap().display, "78.4 %");
}

#[test]
fn energy_balance_shows_dod_per_case_against_the_limit() {
    let page = page(None);
    let o = of(&page, "power.energy_balance");
    assert_eq!(
        bar_keys(o),
        ["NOMINAL", "MISSION", "DOWNLINK", "SAFE", "sequence"]
    );
    let bars = &o.chart.as_ref().unwrap().bars;
    for b in bars {
        let r = b.reference.as_ref().unwrap();
        assert_eq!(r.value.display, "30.0 %");
        // メーターは上限の 1.5 倍まで（上限の線は 2/3）
        assert!((r.at - 1.0 / 1.5).abs() < 1e-12);
        let dod = b.value.as_ref().unwrap().si;
        assert!((b.fill - dod / (0.3 * 1.5)).abs() < 1e-9);
        assert_eq!(b.verdict, Some(sat_analysis::spec::Verdict::Ok));
    }
    // 判定と同じ運用の値: DOWNLINK は最長継続時間で評価した 15.1 %（連続なら 100 %）
    let downlink = bars.iter().find(|b| b.key == "DOWNLINK").unwrap();
    assert_eq!(downlink.value.as_ref().unwrap().display, "15.1 %");
    assert_eq!(downlink.label.label, "ダウンリンク");
}

#[test]
fn energy_balance_checks_are_labelled_with_their_case() {
    for (case, scopes) in [
        (
            None,
            vec!["定常", "撮像", "ダウンリンク", "セーフ", "モードの並び"],
        ),
        (Some("hot_imaging"), vec!["撮像", "モードの並び"]),
    ] {
        let page = page(case);
        let groups = &of(&page, "power.energy_balance").checks;
        let got: Vec<_> = groups.iter().map(|g| g.scope.as_deref()).collect();
        assert_eq!(got, scopes.into_iter().map(Some).collect::<Vec<_>>());
        for g in groups {
            let keys: Vec<_> = g.checks.iter().map(|c| c.key.as_str()).collect();
            assert_eq!(keys, ["dod", "energy_balance"]);
        }
        // サイジングの判定は出力の最上位（持ち主なし）の 1 まとまり
        let sizing = &of(&page, "power.sizing").checks;
        assert_eq!(sizing.len(), 1);
        assert_eq!(sizing[0].scope, None);
    }
}

#[test]
fn sizing_compares_current_with_required() {
    let page = page(None);
    let o = of(&page, "power.sizing");
    assert_eq!(bar_keys(o), ["area", "capacity"]);
    let bars = &o.chart.as_ref().unwrap().bars;
    // デモ: 設計 2990 cm² / 必要 1295 cm²、設計 45.0 Wh / 必要 22.9 Wh（判定の文と同じ）
    assert_eq!(bars[0].value.as_ref().unwrap().display, "2990 cm²");
    assert_eq!(
        bars[0].reference.as_ref().unwrap().value.display,
        "1295 cm²"
    );
    assert_eq!(bars[1].value.as_ref().unwrap().display, "45.0 Wh");
    assert_eq!(bars[1].reference.as_ref().unwrap().value.display, "22.9 Wh");
    let ratio: f64 = 2990.0 / 1295.0;
    assert!((bars[0].fill - (ratio / 1.5).min(1.0)).abs() < 1e-3);
    assert_eq!(bars[0].reference.as_ref().unwrap().label.label, "必要");
}

#[test]
fn tables_repeated_across_analyses_are_hidden_once() {
    let page = page(None);
    let hidden = |id: &str| of(&page, id).hidden_tables.clone();
    // 電力収支の中の負荷の表は負荷の解析と同じ
    assert_eq!(hidden("power.energy_balance"), [["mode_loads", "modes"]]);
    assert!(hidden("power.mode_load").is_empty());
    assert!(hidden("power.orbit_generation").is_empty());
    // ケースを選ぶと蝕の表も同じになり、隠す
    let page = self::page(Some("hot_imaging"));
    assert_eq!(
        of(&page, "power.energy_balance").hidden_tables,
        [vec!["mode_loads", "modes"], vec!["eclipses"]]
    );
}
