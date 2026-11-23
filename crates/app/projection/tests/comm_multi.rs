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
    // 回線 × 局の余裕の判定が 6 つ。PFD の限度を書いた回線（デモの s_down）は局ごとに PFD の判定がもう 1 つ
    let keys: Vec<&str> = out["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys.iter().filter(|k| k.starts_with("margin:")).count(), 6);
    let mut pfd: Vec<&str> = keys
        .iter()
        .filter(|k| k.starts_with("pfd:"))
        .copied()
        .collect();
    pfd.sort();
    assert_eq!(pfd, ["pfd:s_down@SECOND_GS", "pfd:s_down@TOKYO_GS"]);
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

/// 下りの帯域に `g_over_t` があれば、地上局の `antenna_gain` と `noise_temperature` は書かなくてよい
/// （余裕は書いたときと同じ）。`g_over_t` が無ければ、どちらも必須（不足入力）。上りは地上局の利得が要る。
#[test]
fn g_over_t_alone_describes_a_downlink_station() {
    let (mut design, resolved) = demo();
    let margin = |design: &fune_model::Design, resolved: &fune_model::Resolved| {
        let input = project(design, resolved, "comm.link_budget");
        let out = sat_analysis::catalog::run_json("comm.link_budget", input).unwrap();
        out["links"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["name"] == "s_down")
            .unwrap()["margin_db"]
            .as_f64()
            .unwrap()
    };
    let full = margin(&design, &resolved);
    let band = design
        .conditions
        .ground
        .get_mut("TOKYO_GS")
        .unwrap()
        .links
        .get_mut("s_band")
        .unwrap();
    band.antenna_gain = None;
    band.noise_temperature = None;
    let resolved = fune_model::resolve(&design);
    assert_eq!(margin(&design, &resolved), full);
    let input = project(&design, &resolved, "comm.link_budget");
    let rf = &input["links"][0]["rf"];
    assert!(rf["rx_gain_db"].is_null() && rf["system_noise_temperature"].is_null());
    // G/T が無ければ利得と雑音温度が要る
    design
        .conditions
        .ground
        .get_mut("TOKYO_GS")
        .unwrap()
        .links
        .get_mut("s_band")
        .unwrap()
        .g_over_t = None;
    let paths: Vec<String> = missing_paths(&design, "comm.link_budget")
        .into_iter()
        .map(|(p, _)| p)
        .collect();
    assert!(
        paths.iter().any(|p| p.ends_with("s_band.antenna_gain")),
        "{paths:?}"
    );
    assert!(
        paths
            .iter()
            .any(|p| p.ends_with("s_band.noise_temperature")),
        "{paths:?}"
    );
}

fn comm_mut(design: &mut fune_model::Design) -> &mut fune_model::design::CommSettings {
    design.analysis.comm.as_mut().unwrap()
}

fn missing_paths(design: &fune_model::Design, id: &str) -> Vec<(String, String)> {
    let resolved = fune_model::resolve(design);
    let ctx = Context::new(design, &resolved, None).unwrap();
    let fune_projection::ProjectionError::Missing(m) = project_json(id, &ctx).unwrap_err() else {
        panic!("{id} should be missing inputs");
    };
    m.into_vec()
        .into_iter()
        .map(|x| (x.path.to_string(), x.reason))
        .collect()
}

/// `data_downlink` が無ければ名前順の先頭（上の `data_throughput_names_stations_and_uncounted_downlinks` と同じ）、
/// 2 本目を選べばその回線が数えられ、1 本目が数えない側に移る。2 局 × 2 下りで何も黙って落ちない。
#[test]
fn data_downlink_selects_the_counted_downlink() {
    let (mut design, _) = with_second_station();
    let input = project(
        &design.clone(),
        &fune_model::resolve(&design),
        "comm.data_throughput",
    );
    assert_eq!(input["downlink"], "s_down");
    comm_mut(&mut design).data_downlink = Some("uhf_down".to_owned());
    let resolved = fune_model::resolve(&design);
    let input = project(&design, &resolved, "comm.data_throughput");
    assert_eq!(input["downlink"], "uhf_down");
    assert_eq!(input["other_downlinks"], serde_json::json!(["s_down"]));
    assert_eq!(input["station_links"].as_array().unwrap().len(), 2);
    assert_eq!(input["stations_without_band"], serde_json::json!([]));
    let out = sat_analysis::catalog::run_json("comm.data_throughput", input).unwrap();
    assert_eq!(out["station_closures"].as_array().unwrap().len(), 2);
    let checks = out["checks"].as_array().unwrap();
    assert!(checks.iter().any(|c| c["key"] == "downlinks_not_counted"));
}

/// 存在しない回線・下りでない回線を選ぶと、不足入力として `data_downlink` が挙がる。
#[test]
fn data_downlink_naming_a_missing_or_up_link_is_missing() {
    for name in ["nope", "uhf_up"] {
        let (mut design, _) = demo();
        comm_mut(&mut design).data_downlink = Some(name.to_owned());
        let missing = missing_paths(&design, "comm.data_throughput");
        assert!(
            missing
                .iter()
                .any(|(p, _)| p == "analysis.comm.data_downlink"),
            "{name}: {missing:?}"
        );
    }
}

fn s_down_tx_gain(design: &fune_model::Design) -> f64 {
    let resolved = fune_model::resolve(design);
    let input = project(design, &resolved, "comm.link_budget");
    let l = input["links"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["name"] == "s_down")
        .unwrap();
    l["rf"]["tx_gain_db"].as_f64().unwrap()
}

/// 回線の `antenna` があればその部品の `antenna_gain`、無ければ送受信機の値。部品に値が無ければその部品を名指して不足入力。
#[test]
fn link_antenna_part_gain_is_used_over_the_transceiver() {
    let (mut design, _) = demo();
    assert!((s_down_tx_gain(&design) - 6.5).abs() < 1e-9);
    // UHF 送受信機（0 dBi）をアンテナに指定すると、S 帯の下りの送信利得が 0 dB になる
    comm_mut(&mut design)
        .links
        .get_mut("s_down")
        .unwrap()
        .antenna = Some("UHF_TRX".to_owned());
    assert!(s_down_tx_gain(&design).abs() < 1e-9);
    // `antenna_gain` を持たない部品を指すと、その部品の属性が不足入力
    let other = design
        .parts
        .keys()
        .find(|id| !matches!(id.as_str(), "UHF_TRX" | "STX"))
        .unwrap()
        .to_string();
    comm_mut(&mut design)
        .links
        .get_mut("s_down")
        .unwrap()
        .antenna = Some(other.clone());
    let missing = missing_paths(&design, "comm.link_budget");
    assert!(
        missing
            .iter()
            .any(|(p, _)| p.contains(&other) && p.contains("antenna_gain")),
        "{missing:?}"
    );
    // 存在しない部品
    comm_mut(&mut design)
        .links
        .get_mut("s_down")
        .unwrap()
        .antenna = Some("NO_SUCH".to_owned());
    let missing = missing_paths(&design, "comm.link_budget");
    assert!(
        missing
            .iter()
            .any(|(p, _)| p == "analysis.comm.link.s_down.antenna"),
        "{missing:?}"
    );
}
