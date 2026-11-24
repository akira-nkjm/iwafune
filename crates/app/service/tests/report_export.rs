//! 設計書の表と図の書き出し（`report::export`）。デモの設計から、6 項目が解析・バジェットの結果と一致して出ること、
//! 同じ入力なら同じバイト列になること、入力が足りない項目は注記つきで出て全体は失敗しないことを確かめる。
//! 設計データには書き込まない。

mod common_report;

use common_report::{data, demo, demo_copy, export, file};
use fune_service::dto::{
    BudgetKind, BudgetRequest, ReportItem, ReportLang, ReportRequest, RunAnalysisRequest,
};
use fune_service::{analysis, budget, report};
use serde_json::Value;

fn req(items: &[ReportItem], cases: &[&str]) -> ReportRequest {
    ReportRequest {
        items: items.to_vec(),
        cases: cases.iter().map(|c| (*c).to_owned()).collect(),
        lang: ReportLang::En,
    }
}

fn tables<'a>(d: &'a Value, item: &str) -> Vec<&'a Value> {
    d["tables"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["item"] == item)
        .collect()
}

/// 数値が（JSON の読み戻しの丸めを除いて）等しい。
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * a.abs().max(b.abs()).max(1e-300)
}

fn assert_close(a: &Value, b: &Value) {
    let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
    assert!(close(a, b), "{a} != {b}");
}

/// `data.json` の形の概要（表・図の項目・ケース・列の見出し・行数、図の段と系列、値のキー数）。
fn outline(d: &Value) -> String {
    let mut out = format!(
        "schema {} lang {} items {} cases {}\n",
        d["schema"], d["lang"], d["items"], d["cases"]
    );
    for t in d["tables"].as_array().unwrap() {
        let heads: Vec<String> = t["columns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| match c["unit"].as_str() {
                Some(u) => format!("{} [{u}]", c["header"].as_str().unwrap()),
                None => c["header"].as_str().unwrap().to_owned(),
            })
            .collect();
        out.push_str(&format!(
            "table {} case={} subject={} variant={} rows={} | {}\n  caption: {}\n",
            t["item"].as_str().unwrap(),
            t["case"],
            t["subject"],
            t["variant"],
            t["rows"].as_array().unwrap().len(),
            heads.join(" | "),
            t["caption"].as_str().unwrap()
        ));
    }
    for f in d["figures"].as_array().unwrap() {
        let panels: Vec<String> = f["panels"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                format!(
                    "{} ({} series)",
                    p["y_label"].as_str().unwrap(),
                    p["series"].as_array().unwrap().len()
                )
            })
            .collect();
        out.push_str(&format!(
            "figure {} case={} file={} shaded={} x={} panels: {}\n  caption: {}\n",
            f["item"].as_str().unwrap(),
            f["case"],
            f["file"],
            f["shaded"],
            f["x_label"].as_str().unwrap(),
            panels.join("; "),
            f["caption"].as_str().unwrap()
        ));
    }
    let keys: Vec<&str> = d["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["key"].as_str().unwrap())
        .filter(|k| !k.starts_with("part.") && !k.starts_with("kpi."))
        .collect();
    out.push_str(&format!(
        "values: {} (budget keys: {})\n",
        d["values"].as_array().unwrap().len(),
        keys.join(", ")
    ));
    out
}

#[test]
fn default_export_outline_on_the_demo() {
    let r = export(&demo(), &ReportRequest::default());
    assert!(r.unavailable.is_empty(), "{:?}", r.unavailable);
    insta::assert_snapshot!(outline(&data(&r)));
}

#[test]
fn files_are_listed_in_a_fixed_order_and_are_byte_stable() {
    let ws = demo();
    let a = export(&ws, &req(&[], &["hot_imaging", "cold_safe"]));
    let b = export(&ws, &req(&[], &["hot_imaging", "cold_safe"]));
    assert_eq!(a.files, b.files, "two exports of the same design differ");
    let paths: Vec<&str> = a.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "data.json",
            "iwafune.typ",
            "example.typ",
            "figs/power-history-cold_safe.svg",
            "figs/power-history-hot_imaging.svg",
            "figs/thermal-history-cold_safe.svg",
            "figs/thermal-history-hot_imaging.svg",
        ]
    );
    assert!(file(&a, "iwafune.typ").contains("#let mass-table()"));
    assert!(file(&a, "example.typ").contains("#thermal-history(case: \"cold_safe\")"));
    let svg = file(&a, "figs/thermal-history-hot_imaging.svg");
    assert!(svg.starts_with("<svg "), "{}", &svg[..40]);
    assert!(svg.contains("<polyline") && svg.contains("stroke-dasharray"));
    assert!(svg.contains(">Eclipse</text>"));
}

#[test]
fn revision_is_in_the_data_the_notes_and_the_result() {
    let ws = demo();
    let r = export(&ws, &ReportRequest::default());
    let revision = ws.snapshot().unwrap().design.revision.0;
    assert_eq!(r.revision, revision);
    let d = data(&r);
    assert_eq!(d["revision"], revision.as_str());
    for t in d["tables"].as_array().unwrap() {
        let notes = t["notes"].as_array().unwrap();
        assert!(
            notes
                .iter()
                .any(|n| n.as_str().unwrap().contains(&revision)),
            "no revision note in {}",
            t["caption"]
        );
    }
}

#[test]
fn mass_table_agrees_with_the_budget() {
    let ws = demo();
    let r = export(&ws, &req(&[ReportItem::Mass], &[]));
    let d = data(&r);
    let view = budget::budget(&ws, BudgetKind::Mass, &BudgetRequest::default()).unwrap();
    let table = tables(&d, "mass")[0];
    let rows = table["rows"].as_array().unwrap();
    let si = |row: &Value, col: usize| row["cells"][col]["si"].as_f64().unwrap();
    // 合計の行（成長余裕込み）は質量バジェットの合計と同じ
    let total = rows.iter().find(|r| r["style"] == "total").unwrap();
    assert!(close(si(total, 5), view.total.as_ref().unwrap().si));
    // 小計は配分の行の成長余裕込み
    let subtotals: Vec<f64> = rows
        .iter()
        .filter(|r| r["style"] == "subtotal")
        .map(|r| si(r, 5))
        .collect();
    let expected: Vec<f64> = view
        .rows
        .iter()
        .filter(|r| r.current.si > 0.0)
        .map(|r| r.with_growth.si)
        .collect();
    assert_eq!(subtotals.len(), expected.len());
    assert!(subtotals.iter().zip(&expected).all(|(a, b)| close(*a, *b)));
    // 列の見出しは単位つき、表の注記はマージンの扱いを述べる
    assert_eq!(table["columns"][2]["unit"], "g");
    assert!(
        table["notes"][0]
            .as_str()
            .unwrap()
            .contains("growth margin")
    );
    let keys: Vec<&str> = d["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["key"].as_str().unwrap())
        .collect();
    assert!(keys.contains(&"budget.mass.total") && keys.contains(&"budget.mass.margin"));
    let rw = d["values"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["key"] == "part.RW_X.mass")
        .unwrap();
    assert_eq!(rw["text"], "152 g");
}

#[test]
fn mode_power_table_agrees_with_mode_load() {
    let ws = demo();
    let r = export(&ws, &req(&[ReportItem::ModePower], &[]));
    let d = data(&r);
    let table = tables(&d, "mode-power")[0];
    let run =
        analysis::run_analysis(&ws, "power.mode_load", &RunAnalysisRequest::default()).unwrap();
    let modes = run.output.unwrap()["modes"].as_array().unwrap().clone();
    assert_eq!(table["columns"].as_array().unwrap().len(), modes.len() + 1);
    let rows = table["rows"].as_array().unwrap();
    let with = rows
        .iter()
        .find(|r| r["cells"][0]["text"] == "Total (with growth margin)")
        .unwrap();
    let without = rows
        .iter()
        .find(|r| r["cells"][0]["text"] == "Total (without margin)")
        .unwrap();
    for (i, m) in modes.iter().enumerate() {
        assert_close(&with["cells"][i + 1]["si"], &m["total_with_growth"]);
        assert_close(&without["cells"][i + 1]["si"], &m["total"]);
    }
    // 機器の行は状態を添える
    let heater = rows
        .iter()
        .find(|r| {
            r["cells"][0]["text"]
                .as_str()
                .is_some_and(|t| t.contains("(HTR_BAT)"))
        })
        .unwrap();
    assert_eq!(heater["cells"][1]["detail"], "Off 80.0 %, On 20.0 %");
}

#[test]
fn link_budget_has_an_input_and_a_result_table_per_link() {
    let ws = demo();
    let r = export(&ws, &req(&[ReportItem::LinkBudget], &[]));
    let d = data(&r);
    let all = tables(&d, "link-budget");
    assert_eq!(all.len(), 6);
    let run =
        analysis::run_analysis(&ws, "comm.link_budget", &RunAnalysisRequest::default()).unwrap();
    let links = run.output.unwrap()["links"].as_array().unwrap().clone();
    for l in &links {
        let subject = format!(
            "{}@{}",
            l["name"].as_str().unwrap(),
            l["station"].as_str().unwrap()
        );
        let result = all
            .iter()
            .find(|t| t["subject"] == subject.as_str() && t["variant"] == "result")
            .unwrap();
        let margin = result["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["cells"][0]["text"] == "Link margin")
            .unwrap();
        assert_close(&margin["cells"][1]["si"], &l["margin_db"]);
        assert!(
            result["caption"]
                .as_str()
                .unwrap()
                .contains(l["name"].as_str().unwrap())
        );
        assert!(
            all.iter()
                .any(|t| t["subject"] == subject.as_str() && t["variant"] == "input")
        );
    }
    let first = all[0]["caption"].as_str().unwrap();
    assert!(first.starts_with("Link budget inputs: "), "{first}");
}

#[test]
fn thermal_result_agrees_with_the_analysis_per_case() {
    let ws = demo();
    let r = export(
        &ws,
        &req(&[ReportItem::ThermalResult], &["hot_imaging", "cold_safe"]),
    );
    let d = data(&r);
    for case in ["hot_imaging", "cold_safe"] {
        let table = tables(&d, "thermal-result")
            .into_iter()
            .find(|t| t["case"] == case)
            .unwrap();
        let run = analysis::run_analysis(
            &ws,
            "thermal.steady",
            &RunAnalysisRequest {
                case: Some(case.to_owned()),
                ..Default::default()
            },
        )
        .unwrap();
        let envelope = run.output.unwrap()["envelope"].as_array().unwrap().clone();
        let rows = table["rows"].as_array().unwrap();
        assert_eq!(rows.len(), envelope.len());
        let cols = table["columns"].as_array().unwrap().len();
        for (row, e) in rows.iter().zip(&envelope) {
            assert_eq!(row["cells"][0]["text"], e["node"]);
            assert_close(&row["cells"][cols - 4]["si"], &e["min"]);
            assert_close(&row["cells"][cols - 3]["si"], &e["max"]);
        }
        assert!(table["columns"][1]["unit"] == "°C");
    }
}

#[test]
fn figures_carry_the_case_eclipse_shading_and_limits() {
    let r = export(
        &demo(),
        &req(
            &[ReportItem::ThermalHistory, ReportItem::PowerHistory],
            &["eclipse_worst"],
        ),
    );
    let d = data(&r);
    let figs = d["figures"].as_array().unwrap();
    assert_eq!(figs.len(), 2);
    let thermal = figs
        .iter()
        .find(|f| f["item"] == "thermal-history")
        .unwrap();
    assert_eq!(thermal["case"], "eclipse_worst");
    assert!(thermal["shaded"].as_u64().unwrap() >= 4, "eclipse bands");
    assert_eq!(thermal["panels"][0]["series"].as_array().unwrap().len(), 14);
    let power = figs.iter().find(|f| f["item"] == "power-history").unwrap();
    assert_eq!(
        power["panels"].as_array().unwrap().len(),
        2,
        "power and state of charge"
    );
}

#[test]
fn japanese_headings_use_the_label_table() {
    let r = export(
        &demo(),
        &ReportRequest {
            lang: ReportLang::Ja,
            ..req(&[ReportItem::Mass], &[])
        },
    );
    let d = data(&r);
    assert_eq!(d["lang"], "ja");
    assert_eq!(tables(&d, "mass")[0]["caption"], "質量バジェット");
}

#[test]
fn missing_inputs_give_a_marked_item_and_not_a_failure() {
    let (tmp, ws) = demo_copy();
    std::fs::remove_dir_all(tmp.path().join("demo-6u/conditions/ground")).unwrap();
    let r = export(&ws, &req(&[ReportItem::Mass, ReportItem::LinkBudget], &[]));
    assert_eq!(r.unavailable.len(), 1);
    assert_eq!(r.unavailable[0].item, ReportItem::LinkBudget);
    let d = data(&r);
    let link = tables(&d, "link-budget")[0];
    assert!(
        link["unavailable"]
            .as_str()
            .unwrap()
            .starts_with("Not available: ")
    );
    assert!(tables(&d, "mass")[0]["unavailable"].is_null());
}

#[test]
fn unknown_case_and_item_are_input_errors() {
    let err = report::export(&demo(), &req(&[], &["no_such_case"])).unwrap_err();
    assert_eq!(err.kind(), fune_service::ErrorKind::NotFound);
    assert!(report::parse_item("mass").is_ok());
    let err = report::parse_item("pdf").unwrap_err();
    assert_eq!(err.kind(), fune_service::ErrorKind::InvalidInput);
    assert!(report::parse_lang("fr").is_err());
}

#[test]
fn export_does_not_touch_the_design() {
    let (tmp, ws) = demo_copy();
    let before = ws.snapshot().unwrap().design.revision;
    export(&ws, &ReportRequest::default());
    assert_eq!(ws.snapshot().unwrap().design.revision, before);
    assert!(!tmp.path().join("demo-6u/exports").exists());
    assert!(!tmp.path().join("demo-6u/.fune").exists());
}
