//! `fune flow` の表示と、`--json` が HTTP（`GET /flow/{system}`）と同じ `FlowView` であること。
mod common;

use common::{fixture, fune, get_json};
use serde_json::Value;

fn flow_json(args: &[&str]) -> Value {
    let out = fune()
        .arg("-C")
        .arg(fixture())
        .arg("--json")
        .arg("flow")
        .args(args)
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
fn json_matches_http_with_and_without_case() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    for (args, uri) in [
        (vec!["power"], "/api/v1/flow/power"),
        (
            vec!["power", "--case", "hot_imaging"],
            "/api/v1/flow/power?case=hot_imaging",
        ),
    ] {
        let cli = flow_json(&args);
        assert!(!cli["nodes"].as_array().unwrap().is_empty());
        assert_eq!(cli, get_json(&rt, &fixture(), uri), "{uri}");
    }
}

#[test]
fn text_lists_cards_rows_and_edges() {
    let out = fune()
        .arg("-C")
        .arg(fixture())
        .args(["flow", "power"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    for want in [
        "部品: 太陽電池パネル",
        "CELLS, SP_MX, SP_MY, SP_PX, SP_PY",
        "設定: 軌道",
        "J2 永年項",
        "解析: 電力収支と DoD（時系列）",
        "最大 DoD",
        "つながり",
        "（結果）",
    ] {
        assert!(text.contains(want), "{want} が無い:\n{text}");
    }
    // 範囲は「〜」
    assert!(text.contains("cm²〜"), "{text}");
}

#[test]
fn systems_without_a_flow_are_an_error() {
    let out = fune()
        .arg("-C")
        .arg(fixture())
        .args(["flow", "aocs"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("フロー図はまだありません"), "{err}");
    assert!(err.contains("power"), "{err}");
}
