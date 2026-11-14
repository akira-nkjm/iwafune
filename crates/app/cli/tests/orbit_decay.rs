//! 高度低下の解析（`orbit.decay`）: 設定の編集が CLI / HTTP で同じファイルを書くことと、宇宙天気の表の有無で
//! 日付で引いた条件と表の版が結果に入る・入らないこと。
mod common;
use common::{assert_same_files, demo_copy, fune, post_json, run_cli};
use serde_json::{Value, json};
use std::path::Path;

const HEADER: &str = "DATE,BSRN,ND,KP1,KP2,KP3,KP4,KP5,KP6,KP7,KP8,KP_SUM,AP1,AP2,AP3,AP4,AP5,AP6,AP7,AP8,AP_AVG,CP,C9,ISN,F10.7_OBS,F10.7_ADJ,F10.7_DATA_TYPE,F10.7_OBS_CENTER81,F10.7_OBS_LAST81,F10.7_ADJ_CENTER81,F10.7_ADJ_LAST81";

/// 試験用の作った表（デモの元期 2027-04-01 の前後の日ごとの予測と、2 か月分の月ごとの予測。その後は表の外）。
const ROWS: &str = "\
2027-03-31,2640,1,13,13,13,13,13,13,13,13,104,5,5,5,5,5,5,5,5,5,0.2,1,90,120.0,118.0,PRD,121.0,119.0,119.0,117.0
2027-04-01,2640,2,13,13,13,13,13,13,13,13,104,5,5,5,5,5,5,5,5,5,0.2,1,90,121.0,119.0,PRD,121.0,119.0,119.0,117.0
2027-05-01,2641,4,,,,,,,,,,,,,,,,,,,,,80,118.0,116.0,PRM,118.0,117.0,116.0,115.0
2027-06-01,2642,5,,,,,,,,,,,,,,,,,,,,,75,115.0,113.0,PRM,115.0,114.0,113.0,112.0
";

fn analyze(root: &Path, data_dir: &Path) -> Value {
    let out = fune()
        .env("IWAFUNE_DATA_DIR", data_dir)
        .args([
            "-C",
            root.to_str().unwrap(),
            "--json",
            "analyze",
            "orbit.decay",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn decay_settings_match_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_cli_temp, cli_root) = demo_copy();
    let (_http_temp, http_root) = demo_copy();
    for (path, value) in [
        ("conditions.orbit.decay.max_lifetime", "25 yr"),
        ("conditions.orbit.decay.band", "2 km"),
        ("conditions.orbit.decay.span", "10 yr"),
    ] {
        run_cli(&cli_root, &["setting", "set", path, value]);
        let op = post_json(
            &rt,
            &http_root,
            "/api/v1/ops/from_text",
            json!({"path": path, "value": value}),
        );
        post_json(&rt, &http_root, "/api/v1/ops", json!({"ops": [op]}));
        assert_same_files(&cli_root, &http_root, path);
    }
    let run = post_json(&rt, &http_root, "/api/v1/analyses/orbit.decay", json!({}));
    assert_eq!(run["status"], "ok");
    assert_eq!(run["kpis"].as_array().unwrap().len(), 3);
    let used = &run["output"]["used"];
    assert_eq!(used["band"], json!(2000.0));
    assert!(
        run["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["key"] == "max_lifetime")
    );
    let burns = &run["output"]["cases"][1]["station_keeping"]["burns"];
    assert!(burns.as_u64().unwrap() > 0, "{burns}");
}

#[test]
fn dated_condition_needs_the_space_weather_table() {
    let (_temp, root) = demo_copy();
    let empty = tempfile::tempdir().unwrap();
    let r = analyze(&root, empty.path());
    assert_eq!(r["status"], "ok");
    assert!(r.get("space_weather").is_none());
    let ids: Vec<_> = r["output"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].clone())
        .collect();
    assert_eq!(ids, [json!("low"), json!("moderate"), json!("high")]);

    let data = tempfile::tempdir().unwrap();
    std::fs::write(
        data.path().join("SW-All.csv"),
        format!("{HEADER}\r\n{ROWS}"),
    )
    .unwrap();
    let r = analyze(&root, data.path());
    assert_eq!(r["status"], "ok");
    let version = &r["space_weather"];
    assert_eq!(version["first_date"], "2027-03-31");
    assert_eq!(version["last_date"], "2027-06-30");
    assert_eq!(version["sha256"].as_str().unwrap().len(), 64);
    let dated = &r["output"]["cases"][3];
    assert_eq!(dated["id"], "dated");
    assert_eq!(
        dated["condition"],
        json!({"kind": "dated", "fallback": "moderate"})
    );
    assert_eq!(dated["initial_weather"]["f107"], json!(120.0));
    assert_eq!(dated["initial_weather"]["f107_source"], "daily_predicted");
    let f107 = &dated["weather_sources"]["f107"];
    assert!(f107["predicted"].as_u64().unwrap() >= 2);
    assert!(f107["preset"].as_u64().unwrap() > 0);
    assert!(f107["preset_from"].as_str().unwrap().starts_with("2027-07"));
    // 月ごとの予測は Ap を持たないので、その月からは Ap だけプリセット
    assert!(
        dated["weather_sources"]["ap"]["preset_from"]
            .as_str()
            .unwrap()
            .starts_with("2027-05")
    );
    assert!(
        r["labels"]
            .as_object()
            .unwrap()
            .values()
            .any(|l| l["label_en"] == "Dated values from the space-weather table")
    );
}
