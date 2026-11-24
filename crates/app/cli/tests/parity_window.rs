//! 日付の探索（`propulsion.launch_window`）の最小の点をマヌーバの日付にする操作が、CLI（`fune maneuver set-window`）と
//! HTTP（`POST /ops/maneuver_window` と `POST /ops`）で同じファイルを書き、そのファイルが `fune maneuver set … --depart … --arrive …`
//! にその日付を打ったものとも同じになること。書いたあとの `propulsion.maneuvers` が探索の最小の点の値と一致すること。
mod common;
use common::{assert_same_files, demo_copy, fune, post_json, run_cli, snapshot_files};
use serde_json::json;

/// 火星への天体間マヌーバ（日付なし）を足す引数。
const MARS: [&str; 15] = [
    "maneuver",
    "add",
    "MARS",
    "--category",
    "insertion",
    "--method",
    "impulsive",
    "--thruster",
    "EPT",
    "--after",
    "DEORBIT",
    "--body",
    "mars",
    "--perigee-altitude",
    "300 km",
];

fn with_mars(root: &std::path::Path) {
    let mut args = MARS.to_vec();
    args.extend(["--apogee-altitude", "33000 km"]);
    run_cli(root, &args);
}

#[test]
fn set_window_writes_the_same_file_over_cli_and_http_and_as_maneuver_set() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    for min in ["dv", "c3"] {
        let (_a, cli) = demo_copy();
        let (_b, http) = demo_copy();
        let (_c, by_hand) = demo_copy();
        for root in [&cli, &http, &by_hand] {
            with_mars(root);
        }
        run_cli(&cli, &["maneuver", "set-window", "MARS", "--min", min]);
        // 適用の前の探索の最小の点
        let before = post_json(
            &rt,
            &http,
            "/api/v1/analyses/propulsion.launch_window",
            json!({}),
        );
        assert_eq!(before["status"], "ok", "{before}");
        let best = before["output"]["windows"][0][if min == "dv" {
            "minimum_delta_v"
        } else {
            "minimum_c3"
        }]
        .clone();
        assert!(before["output"]["windows"][0]["written"].is_null());
        let op = post_json(
            &rt,
            &http,
            "/api/v1/ops/maneuver_window",
            json!({"id": "MARS", "min": min}),
        );
        assert_eq!(op["op"], "set_maneuver");
        post_json(&rt, &http, "/api/v1/ops", json!({"ops": [op.clone()]}));
        assert_same_files(&cli, &http, min);
        // 同じ日付を `maneuver set … --depart … --arrive …` に打つと、同じファイルになる
        let tr = &op["maneuver"]["transfer"];
        let (dep, arr) = (
            tr["departure"].as_str().unwrap(),
            tr["arrival"].as_str().unwrap(),
        );
        run_cli(
            &by_hand,
            &[
                "maneuver",
                "set",
                "MARS",
                "--category",
                "insertion",
                "--method",
                "impulsive",
                "--thruster",
                "EPT",
                "--after",
                "DEORBIT",
                "--body",
                "mars",
                "--perigee-altitude",
                "300 km",
                "--apogee-altitude",
                "33000 km",
                "--depart",
                dep,
                "--arrive",
                arr,
            ],
        );
        let (a, c) = (snapshot_files(&cli), snapshot_files(&by_hand));
        assert_eq!(
            a["maneuvers/MARS.toml"],
            c["maneuvers/MARS.toml"],
            "set-window and maneuver set differ\n{}\n---\n{}",
            String::from_utf8_lossy(&a["maneuvers/MARS.toml"]),
            String::from_utf8_lossy(&c["maneuvers/MARS.toml"])
        );
        assert!(String::from_utf8_lossy(&a["maneuvers/MARS.toml"]).contains("[transfer]"));
        // 書いたあとの propulsion.maneuvers は探索の最小の点と同じ値（探索の格子の点は同じ経路で解く）
        let ms = post_json(
            &rt,
            &http,
            "/api/v1/analyses/propulsion.maneuvers",
            json!({}),
        );
        let m = ms["output"]["maneuvers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == "MARS")
            .unwrap();
        assert_eq!(m["dated_transfer"]["departure"], best["departure"]);
        assert_eq!(m["dated_transfer"]["arrival"], best["arrival"]);
        assert_eq!(m["ideal_delta_v"], best["delta_v"]);
        assert_eq!(m["dated_transfer"]["c3"], best["c3"]);
        // 書いたあとの探索は、書かれた日付の点（窓は書かれた出発の日が真ん中になる）が適用の前の最小の点と同じ値
        let after = post_json(
            &rt,
            &http,
            "/api/v1/analyses/propulsion.launch_window",
            json!({}),
        );
        let written = &after["output"]["windows"][0]["written"]["point"];
        assert_eq!(written["departure"], best["departure"]);
        assert_eq!(written["delta_v"], best["delta_v"]);
        assert_eq!(written["c3"], best["c3"]);
        assert!(after["output"]["points"].as_array().unwrap().len() >= 3);
    }
}

#[test]
fn set_window_refuses_a_maneuver_that_is_not_a_transfer_between_bodies() {
    let (_t, root) = demo_copy();
    let before = snapshot_files(&root);
    for (id, why) in [
        ("RAISE", "not a transfer between bodies"),
        ("NOPE", "not found"),
    ] {
        let out = fune()
            .arg("-C")
            .arg(&root)
            .args(["maneuver", "set-window", id, "--min", "dv"])
            .output()
            .unwrap();
        assert!(!out.status.success(), "{id}");
        let text = String::from_utf8_lossy(&out.stderr).to_lowercase();
        assert!(text.contains(why), "{id}: {text}");
    }
    assert_eq!(before, snapshot_files(&root), "nothing is written on error");
}
