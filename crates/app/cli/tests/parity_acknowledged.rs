//! パリティテスト（CI 必須）: 承知済みの組の編集。CLI の `fune acknowledged add|rm` と、HTTP の `POST /ops`（`acknowledge`・`unacknowledge`）は
//! 同じ Op になり、書かれるファイルはバイト単位で一致する。GUI は所見の欄から Op をそのまま送る。

mod common;

use common::{assert_same_files, demo_copy, post_json, run_cli};
use serde_json::{Value, json};

#[test]
fn acknowledged_edits_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    // (CLI の引数, HTTP の Op)。足す・理由の書き換え・形態を指した承知・円錐・取り消し
    let edits: Vec<(Vec<&str>, Value)> = vec![
        (
            vec![
                "acknowledged",
                "add",
                "interference",
                "SAP_PY",
                "SP_PY",
                "--reason",
                "Hinge edge",
            ],
            json!({ "op": "acknowledge", "finding": "interference", "first": "SAP_PY",
                "second": "SP_PY", "reason": "Hinge edge" }),
        ),
        (
            vec![
                "acknowledged",
                "add",
                "interference",
                "SP_PY",
                "SAP_PY",
                "--reason",
                "Hinge edge: the boxes meet at the hinge line",
            ],
            json!({ "op": "acknowledge", "finding": "interference", "first": "SP_PY",
                "second": "SAP_PY", "reason": "Hinge edge: the boxes meet at the hinge line" }),
        ),
        (
            vec![
                "acknowledged",
                "add",
                "cone_clearance",
                "SS_PX",
                "SAP_PY",
                "--configuration",
                "STOWED",
                "--reason",
                "Folded paddle",
            ],
            json!({ "op": "acknowledge", "finding": "cone_clearance", "first": "SS_PX",
                "second": "SAP_PY", "configuration": "STOWED", "reason": "Folded paddle" }),
        ),
        (
            vec![
                "acknowledged",
                "rm",
                "cone_clearance",
                "SS_PX",
                "SAP_PY",
                "--configuration",
                "STOWED",
            ],
            json!({ "op": "unacknowledge", "finding": "cone_clearance", "first": "SS_PX",
                "second": "SAP_PY", "configuration": "STOWED" }),
        ),
    ];
    for (cli, op) in edits {
        run_cli(&cli_root, &cli);
        post_json(&rt, &http_root, "/api/v1/ops", json!({ "ops": [op] }));
        assert_same_files(&cli_root, &http_root, &cli.join(" "));
    }
    let text = std::fs::read_to_string(http_root.join("analysis/structure.toml")).unwrap();
    // デモは 13 組を承知済みにしている。同じ組への承知は行を足さず、理由を書き換える
    assert_eq!(text.matches("[[acknowledged]]").count(), 13, "{text}");
    assert!(text.contains("the boxes meet at the hinge line"), "{text}");
}
