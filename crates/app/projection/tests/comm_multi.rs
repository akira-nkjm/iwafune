//! 通信の射影: 地上局が複数・下り回線が複数でも、どれも黙って落ちないこと（#160）。

use fune_model::design::GroundStationSettings;
use fune_model::value::Quantity;
use fune_projection::{Context, project_json};
use std::path::Path;

fn demo() -> (fune_model::Design, fune_model::Resolved) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    let design = fune_store::load(&root)
        .expect("the demo should load")
        .design;
    let resolved = fune_model::resolve(&design);
    (design, resolved)
}

/// デモの地上局を写して、名前と緯度経度だけ変えた 2 局目を足す（回線の性能は同じ）。
fn with_second_station() -> (fune_model::Design, fune_model::Resolved) {
    let (mut design, _) = demo();
    let mut second: GroundStationSettings = design.conditions.ground["TOKYO_GS"].clone();
    second.id = "SECOND_GS".to_owned();
    second.latitude = Some(Quantity::new(60.0, "deg"));
    second.longitude = Some(Quantity::new(15.0, "deg"));
    design
        .conditions
        .ground
        .insert("SECOND_GS".to_owned(), second);
    if let Some(comm) = design.analysis.comm.as_mut() {
        comm.stations.push("SECOND_GS".to_owned());
    }
    let resolved = fune_model::resolve(&design);
    (design, resolved)
}

fn project(
    design: &fune_model::Design,
    resolved: &fune_model::Resolved,
    id: &str,
) -> serde_json::Value {
    let ctx = Context::new(design, resolved, None).unwrap();
    project_json(id, &ctx).unwrap_or_else(|e| panic!("{id}: {e}"))
}

/// 回線 3 本 × 地上局 2 つ = 6 組がすべて入力に出る。
#[test]
fn link_budget_has_every_link_and_station_pair() {
    let (design, resolved) = with_second_station();
    let input = project(&design, &resolved, "comm.link_budget");
    let mut pairs: Vec<String> = input["links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            format!(
                "{}@{}",
                l["name"].as_str().unwrap(),
                l["station"].as_str().unwrap()
            )
        })
        .collect();
    pairs.sort();
    let mut want = Vec::new();
    for link in ["s_down", "uhf_down", "uhf_up"] {
        for st in ["SECOND_GS", "TOKYO_GS"] {
            want.push(format!("{link}@{st}"));
        }
    }
    assert_eq!(pairs, want);
    // 解析も回り、組ごとの判定が出る
    let out = sat_analysis::catalog::run_json("comm.link_budget", input).unwrap();
    assert_eq!(out["links"].as_array().unwrap().len(), 6);
    assert_eq!(out["checks"].as_array().unwrap().len(), 6);
}

/// 2 局目が S 帯の表を持たなければ、S 帯の回線はその局と組にせず（不足入力にもしない）、UHF の回線だけが組になる。
/// 送信量では、その局は S 帯の下りに数えず `stations_without_band` に残る。
#[test]
fn a_station_without_the_band_is_not_paired_nor_missing() {
    let (mut design, _) = with_second_station();
    design
        .conditions
        .ground
        .get_mut("SECOND_GS")
        .unwrap()
        .links
        .remove("s_band");
    let resolved = fune_model::resolve(&design);
    let input = project(&design, &resolved, "comm.link_budget");
    let mut pairs: Vec<String> = input["links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            format!(
                "{}@{}",
                l["name"].as_str().unwrap(),
                l["station"].as_str().unwrap()
            )
        })
        .collect();
    pairs.sort();
    assert_eq!(
        pairs,
        [
            "s_down@TOKYO_GS",
            "uhf_down@SECOND_GS",
            "uhf_down@TOKYO_GS",
            "uhf_up@SECOND_GS",
            "uhf_up@TOKYO_GS"
        ]
    );
    let input = project(&design, &resolved, "comm.data_throughput");
    assert_eq!(
        input["stations_without_band"],
        serde_json::json!(["SECOND_GS"])
    );
    assert_eq!(input["stations"].as_array().unwrap().len(), 1);
    let out = sat_analysis::catalog::run_json("comm.data_throughput", input).unwrap();
    let closures = out["station_closures"].as_array().unwrap();
    assert_eq!(closures.len(), 2);
    assert_eq!(closures[1]["station"], "SECOND_GS");
    assert_eq!(closures[1]["closure"]["kind"], "no_band_settings");
}

/// どの局にも回線の帯域が無ければ、帯域を名指しして不足入力にする。
#[test]
fn a_band_no_station_has_is_missing() {
    let (mut design, _) = demo();
    design
        .conditions
        .ground
        .get_mut("TOKYO_GS")
        .unwrap()
        .links
        .remove("s_band");
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let err = project_json("comm.link_budget", &ctx).unwrap_err();
    let fune_projection::ProjectionError::Missing(m) = err else {
        panic!("{err}");
    };
    assert!(
        m.into_vec()
            .iter()
            .any(|x| x.path == "analysis.comm.link.s_down.band" && x.reason.contains("s_band"))
    );
}

/// 送信量: 局ごとの回線の条件が全局分あり、数えた下り回線とそうでない回線の名前が入力に出る。
#[test]
fn data_throughput_names_stations_and_uncounted_downlinks() {
    let (design, resolved) = with_second_station();
    let input = project(&design, &resolved, "comm.data_throughput");
    assert_eq!(input["stations"].as_array().unwrap().len(), 2);
    let linked: Vec<&str> = input["station_links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["station"].as_str().unwrap())
        .collect();
    assert_eq!(linked, ["TOKYO_GS", "SECOND_GS"]);
    assert_eq!(input["stations_without_band"], serde_json::json!([]));
    assert_eq!(input["downlink"], "s_down");
    assert_eq!(input["other_downlinks"], serde_json::json!(["uhf_down"]));
    let out = sat_analysis::catalog::run_json("comm.data_throughput", input).unwrap();
    let checks = out["checks"].as_array().unwrap();
    assert!(checks.iter().any(|c| c["key"] == "downlinks_not_counted"));
    assert_eq!(out["station_closures"].as_array().unwrap().len(), 2);
}

/// 地上局の G/T（`g_over_t`）は下りの受信側の G/T として渡り、無い帯域・上りには渡らない。
#[test]
fn ground_g_over_t_reaches_the_downlink_only() {
    let (design, resolved) = demo();
    let input = project(&design, &resolved, "comm.link_budget");
    let g_t = |name: &str| {
        let l = input["links"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["name"] == name)
            .unwrap();
        l["rf"]["rx_g_over_t_dbk"].clone()
    };
    assert_eq!(g_t("s_down").as_f64(), Some(9.0));
    assert!(g_t("uhf_down").is_null() && g_t("uhf_up").is_null());
    let out = sat_analysis::catalog::run_json("comm.link_budget", input).unwrap();
    let r = out["links"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["name"] == "s_down")
        .unwrap();
    assert_eq!(r["g_over_t_dbk"].as_f64(), Some(9.0));
    assert_eq!(r["g_over_t_given"], true);
}
