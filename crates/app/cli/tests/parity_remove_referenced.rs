//! パリティテスト（CI 必須）: まだ指されているフェーズ・モード・形態・出典を消そうとすると、CLI も HTTP も同じ理由
//! （指している場所を種類ごとに挙げた 1 つの文）で拒み、ファイルは変わらない。

mod common;

use axum::body::Body;
use axum::http::Request;
use common::{demo_copy, fune, snapshot_files};
use serde_json::{Value, json};
use tower::ServiceExt;

/// HTTP で Op を 1 つ当てて、拒まれたときの本文（JSON）を返す。
fn http_refusal(rt: &tokio::runtime::Runtime, root: &std::path::Path, op: Value) -> String {
    let ws = fune_service::Workspace::open(root).unwrap();
    let router = fune_server::router(std::sync::Arc::new(ws));
    let body = json!({ "ops": [op] }).to_string();
    let req = Request::post("/api/v1/ops")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    rt.block_on(async {
        let res = router.oneshot(req).await.unwrap();
        assert!(res.status().is_client_error(), "{}", res.status());
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

#[test]
fn referenced_records_are_refused_with_their_referrers() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let cases = [
        (
            ["phase", "rm", "NOMINAL_OPS"],
            json!({ "op": "remove_phase", "id": "NOMINAL_OPS" }),
            "Phase `NOMINAL_OPS` cannot be removed: referred to by `phase` of case(s)",
        ),
        (
            ["mode", "rm", "NOMINAL"],
            json!({ "op": "remove_mode", "id": "NOMINAL" }),
            "Mode `NOMINAL` cannot be removed: referred to by `mode` of case(s)",
        ),
        (
            ["configuration", "rm", "STOWED"],
            json!({ "op": "remove_configuration", "id": "STOWED" }),
            "Configuration `STOWED` cannot be removed: referred to by",
        ),
    ];
    for (args, op, expected) in cases {
        let (_t1, cli_root) = demo_copy();
        let (_t2, http_root) = demo_copy();
        let before = snapshot_files(&cli_root);
        let out = fune().arg("-C").arg(&cli_root).args(args).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains(expected), "{stderr}");
        assert!(!stderr.contains("is not defined"), "{stderr}");
        let body = http_refusal(&rt, &http_root, op);
        assert!(body.contains(expected), "{body}");
        assert!(body.contains("Change those first"), "{body}");
        assert_eq!(snapshot_files(&cli_root), before);
        assert_eq!(snapshot_files(&http_root), before);
    }
}
