//! パリティテスト（CI 必須）: 根（取付の無い部品）の配置。`fune part place <根> --root --position/--rotation` は根の `[pose]`
//! （機体座標での姿勢）を書き、GUI の配置の欄は同じ文字列を `POST /ops/from_command`（`part_place`・取付先は空）に送る。
//! 機体座標の配置（`--body-pos`・`POST /parts/{id}/place_body`）は根ではそのまま根の `[pose]` になる。形態の中の配置は形態の
//! `[pose.<根>]`。どれも読み方は service の同じ関数なので、書かれるファイルはバイト単位で一致する。

mod common;

use common::{assert_same_files, demo_copy, post_json, run_cli};
use serde_json::{Value, json};
use std::path::Path;

fn http_command(rt: &tokio::runtime::Runtime, root: &Path, body: Value) {
    let ops = post_json(rt, root, "/api/v1/ops/from_command", body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops }));
}

#[test]
fn root_placement_matches_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    // 取付の根の姿勢（機体座標）: 位置と角度
    run_cli(
        &cli_root,
        &[
            "part",
            "place",
            "BUS_FRAME",
            "--root",
            "--position",
            "-45 0 0 mm",
            "--rotation",
            "0 0 90 deg",
        ],
    );
    http_command(
        &rt,
        &http_root,
        json!({ "command": "part_place", "part": "BUS_FRAME", "on": "",
            "position": "-45 0 0 mm", "rotation": "0 0 90 deg" }),
    );
    assert_same_files(&cli_root, &http_root, "part place BUS_FRAME --root");
    let text = std::fs::read_to_string(http_root.join("parts/structure/BUS_FRAME.toml")).unwrap();
    assert!(
        text.contains("[pose]") && !text.contains("[mount]"),
        "{text}"
    );
    // 機体座標の配置（根では取付点の座標系が機体座標: 要求した位置がそのまま `[pose]` に書かれる）
    run_cli(
        &cli_root,
        &[
            "part",
            "place",
            "BUS_FRAME",
            "--body-pos",
            "10 0 5 mm",
            "--body-rot",
            "1 0 0 0",
        ],
    );
    post_json(
        &rt,
        &http_root,
        "/api/v1/parts/BUS_FRAME/place_body",
        json!({ "position": { "x": 10.0, "y": 0.0, "z": 5.0, "unit": "mm" },
            "orientation": { "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 } }),
    );
    assert_same_files(&cli_root, &http_root, "part place BUS_FRAME --body-pos");
    // 形態の中の配置（形態の `[pose.<根>]`）
    run_cli(&cli_root, &["configuration", "add", "SHIFTED"]);
    http_command(
        &rt,
        &http_root,
        json!({ "command": "configuration_add", "id": "SHIFTED" }),
    );
    run_cli(
        &cli_root,
        &[
            "part",
            "place",
            "BUS_FRAME",
            "--configuration",
            "SHIFTED",
            "--position",
            "20 0 0 mm",
        ],
    );
    http_command(
        &rt,
        &http_root,
        json!({ "command": "part_place", "part": "BUS_FRAME", "configuration": "SHIFTED",
            "position": "20 0 0 mm" }),
    );
    assert_same_files(
        &cli_root,
        &http_root,
        "part place BUS_FRAME --configuration",
    );
    let cfg = std::fs::read_to_string(http_root.join("configurations/SHIFTED.toml")).unwrap();
    assert!(cfg.contains("[pose.BUS_FRAME]"), "{cfg}");
}
