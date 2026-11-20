//! SVG・Typst のソース・ファイルの並びのスナップショット（公開の入口だけを使う。座標は小数 2 桁に固定されるので、バイト単位で安定）。

use fune_report::chart::{Chart, Limit, Panel, Series, render};
use fune_report::model::{Item, Lang, ReportData, SCHEMA};
use fune_report::{build_files, typst};

fn chart() -> Chart {
    let x: Vec<f64> = (0..=8).map(f64::from).collect();
    Chart {
        x_label: "Elapsed time [h]".into(),
        panels: vec![
            Panel {
                y_label: "Temperature [°C]".into(),
                series: vec![
                    Series {
                        label: "BAT".into(),
                        x: x.clone(),
                        y: x.iter().map(|v| 10.0 + 8.0 * (v * 0.9).sin()).collect(),
                    },
                    Series {
                        label: "CAM".into(),
                        x: x.clone(),
                        y: x.iter().map(|v| 25.0 - 2.0 * v).collect(),
                    },
                ],
                limits: vec![Limit {
                    label: "Allowable range (operating): BAT [0 – 45 °C]".into(),
                    lo: Some(0.0),
                    hi: Some(45.0),
                }],
            },
            Panel {
                y_label: "State of charge [%]".into(),
                series: vec![Series {
                    label: "SOC".into(),
                    x: x.clone(),
                    y: x.iter().map(|v| 100.0 - v).collect(),
                }],
                limits: Vec::new(),
            },
        ],
        shade: vec![(1.0, 2.0), (5.0, 6.5)],
        shade_label: "Eclipse".into(),
    }
}

#[test]
fn chart_svg_snapshot() {
    insta::assert_snapshot!(render(&chart()));
}

#[test]
fn chart_is_byte_stable_and_uses_one_style_per_series() {
    let a = render(&chart());
    assert_eq!(a, render(&chart()));
    // 系列の色は段をまたいで通し番号で決まる（3 本目は 3 色目）
    assert!(a.contains("stroke=\"#009E73\""));
    assert!(a.contains("<text"), "text stays as real text, not outlines");
}

#[test]
fn files_and_example_follow_the_exported_items() {
    let data = ReportData {
        schema: SCHEMA,
        revision: "b3:abc".into(),
        project: "demo".into(),
        lang: Lang::Ja,
        items: vec![Item::Mass, Item::ThermalHistory],
        cases: vec!["hot".into(), "cold".into()],
        tables: Vec::new(),
        figures: Vec::new(),
        values: Vec::new(),
    };
    let files = build_files(
        &data,
        &[
            ("figs/b.svg".into(), "<svg/>".into()),
            ("figs/a.svg".into(), "<svg/>".into()),
        ],
    )
    .unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "data.json",
            "iwafune.typ",
            "example.typ",
            "figs/a.svg",
            "figs/b.svg"
        ]
    );
    insta::assert_snapshot!(typst::example(&data));
    let json: serde_json::Value = serde_json::from_str(&files[0].content).unwrap();
    assert_eq!(json["schema"], 1);
    assert_eq!(
        json["items"],
        serde_json::json!(["mass", "thermal-history"])
    );
}

#[test]
fn module_has_no_language_specific_text() {
    // 見出し・題・注記は data.json にある。モジュールに日本語・英語の文言を持たせない
    assert!(typst::MODULE.is_ascii());
    for name in [
        "mass-table",
        "mode-power-table",
        "link-budget",
        "thermal-result",
        "thermal-history",
        "power-history",
        "value",
    ] {
        assert!(typst::MODULE.contains(&format!("#let {name}(")), "{name}");
    }
}
