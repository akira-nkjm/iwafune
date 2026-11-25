//! パリティテスト（CI 必須）: 解析ケースの欄を 1 つずつ書く（`fune set|unset cases.<ID>.<欄>` と `POST /ops/from_text`・
//! `/ops/unset_from_text`）と、ケースを丸ごと書く `fune case set` が落とした欄の知らせ（CLI の出力と `POST /ops` の結果の `removed`）。

mod common;

use common::{assert_same_files, demo_copy, fune, post_json, run_cli};
use serde_json::json;

/// 欄ごとの書き込みと消去。CLI と HTTP で書かれるファイルがバイト単位で一致し、ほかの欄は残る。
#[test]
fn case_field_edits_match_cli_and_keep_the_other_fields() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let edits: [(&str, Option<&str>, &str); 12] = [
        ("label", Some("Worst case"), "set_record_field"),
        ("description", None, "set_record_field"),
        ("mode", Some("SAFE"), "set_record_field"),
        ("side", Some("min"), "set_record_field"),
        ("thermal_case", Some("hot"), "set_record_field"),
        ("beta", Some("-45 deg"), "set_record_field"),
        ("orbits", Some("20"), "set_record_field"),
        ("sun_distance", Some("0.9 AU"), "set_record_field"),
        ("phase", None, "set_case_phase"),
        ("epoch", Some("2027-06-01T00:00:00Z"), "set_case_epoch"),
        ("orbits", None, "set_record_field"),
        ("epoch", None, "set_case_epoch"),
    ];
    for (field, value, op_name) in edits {
        let path = format!("cases.eclipse_worst.{field}");
        let op = match value {
            Some(v) => {
                run_cli(&cli_root, &["set", &path, v]);
                post_json(
                    &rt,
                    &http_root,
                    "/api/v1/ops/from_text",
                    json!({ "path": path, "value": v }),
                )
            }
            None => {
                run_cli(&cli_root, &["unset", &path]);
                post_json(
                    &rt,
                    &http_root,
                    "/api/v1/ops/unset_from_text",
                    json!({ "path": path }),
                )
            }
        };
        assert_eq!(op["op"], op_name, "{field}: {op}");
        let applied = post_json(&rt, &http_root, "/api/v1/ops", json!({ "ops": [op] }));
        assert_eq!(applied["removed"], json!([]), "{field}");
        assert_same_files(&cli_root, &http_root, &path);
    }
    // 書いた欄だけが変わった（beta・side・mode は書いた値、書かなかった label は元のまま、外した欄は無い）
    let text = std::fs::read_to_string(http_root.join("cases/eclipse_worst.toml")).unwrap();
    assert!(text.contains("label        = \"Worst case\""), "{text}");
    assert!(
        text.contains("beta         = { value = -45.0, unit = \"deg\" }"),
        "{text}"
    );
    assert!(
        text.contains("sun_distance = { value = 0.9, unit = \"AU\" }"),
        "{text}"
    );
    for gone in ["description", "orbits", "phase", "epoch"] {
        assert!(!text.contains(gone), "{gone} should be gone:\n{text}");
    }
}

/// 欄の名前・値・ケースが違えば、CLI も HTTP も書かずに断る。
#[test]
fn bad_case_field_edits_are_refused_by_both() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t, root) = demo_copy();
    for (path, value) in [
        ("cases.eclipse_worst.colour", "red"),
        ("cases.eclipse_worst.orbits", "1.5"),
        ("cases.eclipse_worst.side", "high"),
        ("cases.eclipse_worst", "x"),
    ] {
        let out = fune()
            .arg("-C")
            .arg(&root)
            .args(["set", path, value])
            .output()
            .unwrap();
        assert!(!out.status.success(), "{path}");
        let (status, _) = common::post_json_response(
            &rt,
            &root,
            "/api/v1/ops/from_text",
            json!({ "path": path, "value": value }),
        );
        assert_ne!(status, axum::http::StatusCode::OK, "{path}");
    }
    // ケースが無い（Op にはなるが、適用は拒む）
    let out = fune()
        .arg("-C")
        .arg(&root)
        .args(["set", "cases.no_such_case.beta", "1 deg"])
        .output()
        .unwrap();
    assert!(!out.status.success());
}

/// `fune case set` が書かなかった欄を落とすとき、CLI は書く前の dry run にも書いた後にも落とす欄を警告で並べ、
/// `POST /ops` の結果（dry run も）の `removed` は同じ一覧を返す。ファイルは CLI と HTTP で一致する。
#[test]
fn case_set_lists_the_fields_it_removes_on_both_surfaces() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let notice = "Removed from cases/eclipse_worst.toml: label, description, side, beta, orbits, thermal_case, phase (not given to set)";
    // dry run: 書かずに知らせる
    let dry = fune()
        .arg("-C")
        .arg(&cli_root)
        .args([
            "case",
            "set",
            "eclipse_worst",
            "--mode",
            "NOMINAL",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(dry.status.success());
    let err = String::from_utf8_lossy(&dry.stderr);
    assert!(
        err.contains(&notice.replace("Removed from", "Would remove from")),
        "{err}"
    );
    assert!(
        std::fs::read_to_string(cli_root.join("cases/eclipse_worst.toml"))
            .unwrap()
            .contains("beta"),
        "a dry run must not write"
    );
    // 書く
    let out = fune()
        .arg("-C")
        .arg(&cli_root)
        .args(["case", "set", "eclipse_worst", "--mode", "NOMINAL"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains(notice), "{err}");
    // 同じ編集を HTTP で（`set_case` の Op）
    let op = json!({ "op": "set_case", "case": { "id": "eclipse_worst", "mode": "NOMINAL" } });
    let dry = post_json(
        &rt,
        &http_root,
        "/api/v1/ops",
        json!({ "ops": [op.clone()], "dry_run": true }),
    );
    let expected = json!([{ "path": "cases/eclipse_worst.toml",
        "keys": ["label", "description", "side", "beta", "orbits", "thermal_case", "phase"] }]);
    assert_eq!(dry["removed"], expected);
    let written = post_json(&rt, &http_root, "/api/v1/ops", json!({ "ops": [op] }));
    assert_eq!(written["removed"], expected);
    assert_same_files(&cli_root, &http_root, "case set");
    // 欄を足すだけの編集や、1 欄の編集は何も知らせない
    let ok = fune()
        .arg("-C")
        .arg(&cli_root)
        .args(["set", "cases.eclipse_worst.beta", "10 deg"])
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&ok.stderr).contains("Removed"));
}
