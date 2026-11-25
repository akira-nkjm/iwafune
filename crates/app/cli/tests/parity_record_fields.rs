//! パリティテスト（CI 必須）: マヌーバ・要求・ハーネス・出典・フェーズの見出しの欄を 1 つずつ書く
//! （`fune set|unset <集まり>.<ID>.<欄>` と `POST /ops/from_text`・`/ops/unset_from_text`）。
//! 解析ケースの欄は `parity_case_fields.rs`。

mod common;

use common::{assert_same_files, demo_copy, fune, post_json, post_json_response, run_cli};
use serde_json::json;

/// (パス, 値。None なら unset)。順に当てる。
const EDITS: &[(&str, Option<&str>)] = &[
    ("maneuvers.RAISE.label", Some("Raise the orbit")),
    ("maneuvers.RAISE.target.altitude", Some("650 km")),
    ("maneuvers.RAISE.constraints.duty", Some("0.8")),
    ("maneuvers.RAISE.constraints.sunlit_only", None),
    ("maneuvers.RAISE.after", None),
    ("maneuvers.DEORBIT.method", Some("finite_burn")),
    ("requirements.R_MASS.constraint.value", Some("11 kg")),
    (
        "requirements.R_MASS.statement_en",
        Some("Mass at most 11 kg"),
    ),
    ("requirements.R_MASS.quantify.all_cases", Some("true")),
    ("requirements.R_MASS.quantify.all_cases", None),
    ("requirements.R_MASS.quantify.cases", Some("eclipse_worst")),
    ("requirements.R_MASS.label", None),
    ("requirements.R_BAT_TEMP.verify.check", None),
    ("harness.H_OBC_CAM.cores", Some("4")),
    ("harness.H_OBC_CAM.length", Some("0.5 m")),
    ("harness.H_OBC_CAM.nets", Some("SPW_PL, P5V")),
    ("harness.H_OBC_CAM.wire", None),
    ("sources.ds_rw10.revision", Some("Rev. B")),
    ("sources.ds_rw10.distribution", Some("public")),
    (
        "sources.ds_rw10.note",
        Some("Checked against the order form"),
    ),
    ("sources.ds_rw10.by", None),
    ("phases.LEOP.label", Some("Early operations")),
    ("phases.LEOP.order", Some("3")),
    ("phases.LEOP.duration", Some("20 d")),
    ("phases.LEOP.modes", Some("SAFE")),
    ("phases.LEOP.start", None),
];

/// 欄ごとの書き込みと消去。CLI と HTTP で書かれるファイルがバイト単位で一致し、書式（`fune fmt`）も保たれる。
#[test]
fn record_field_edits_match_cli_and_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    for (path, value) in EDITS {
        let op = match value {
            Some(v) => {
                run_cli(&cli_root, &["set", path, v]);
                post_json(
                    &rt,
                    &http_root,
                    "/api/v1/ops/from_text",
                    json!({ "path": path, "value": v }),
                )
            }
            None => {
                run_cli(&cli_root, &["unset", path]);
                post_json(
                    &rt,
                    &http_root,
                    "/api/v1/ops/unset_from_text",
                    json!({ "path": path }),
                )
            }
        };
        assert_eq!(op["op"], "set_record_field", "{path}: {op}");
        let applied = post_json(&rt, &http_root, "/api/v1/ops", json!({ "ops": [op] }));
        assert_eq!(applied["removed"], json!([]), "{path}");
        assert_same_files(&cli_root, &http_root, path);
    }
    run_cli(&cli_root, &["fmt", "--check"]);
    let read = |p: &str| std::fs::read_to_string(http_root.join(p)).unwrap();
    // 入れ子の表の欄が正規の位置に入り、外した欄・空になった表は無くなる
    let raise = read("maneuvers/RAISE.toml");
    assert!(
        raise.contains("altitude = { value = 650.0, unit = \"km\" }"),
        "{raise}"
    );
    assert!(raise.contains("duty"), "{raise}");
    assert!(!raise.contains("sunlit_only"), "{raise}");
    let mass = read("requirements/R_MASS.toml");
    assert!(
        mass.contains("value = { value = 11.0, unit = \"kg\" }"),
        "{mass}"
    );
    assert!(mass.contains("[quantify]"), "{mass}");
    assert!(!mass.contains("label"), "{mass}");
    let bat = read("requirements/R_BAT_TEMP.toml");
    assert!(bat.contains("analysis") && !bat.contains("check"), "{bat}");
    let cam = read("harness/H_OBC_CAM.toml");
    assert!(cam.contains("cores  = 4") && !cam.contains("wire"), "{cam}");
    let src = read("sources/ds_rw10.toml");
    assert!(src.contains("Rev. B") && !src.contains("by "), "{src}");
    let leop = read("phases/LEOP.toml");
    assert!(
        leop.contains("order         = 3") && !leop.contains("start"),
        "{leop}"
    );
}

/// 欄の名前・値・レコードが違えば、CLI も HTTP も書かずに断る。欄の名前の誤りは書ける欄を並べる。
#[test]
fn bad_record_field_edits_are_refused_by_both() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t, root) = demo_copy();
    for (path, value) in [
        ("maneuvers.RAISE.colour", "red"),
        ("maneuvers.RAISE.method", "teleport"),
        ("requirements.R_MASS.constraint.kind", "sideways"),
        ("requirements.R_MASS.quantify.all_cases", "maybe"),
        ("requirements.R_MASS.statement", "  "),
        ("harness.H_OBC_CAM.cores", "-1"),
        ("harness.H_OBC_CAM.from", "OBC"),
        ("sources.ds_rw10.kind", "rumour"),
        ("phases.LEOP.order", "first"),
        ("phases.LEOP.orbit", "x"),
        ("phases.LEOP", "x"),
        ("requirements.NO_SUCH.label", "x"),
    ] {
        let out = fune()
            .arg("-C")
            .arg(&root)
            .args(["set", path, value])
            .output()
            .unwrap();
        assert!(!out.status.success(), "{path}");
        let (status, _) = post_json_response(
            &rt,
            &root,
            "/api/v1/ops/from_text",
            json!({ "path": path, "value": value }),
        );
        // 書けない値は Op にする前に断るか、適用を断る
        if status == axum::http::StatusCode::OK {
            let op = post_json(
                &rt,
                &root,
                "/api/v1/ops/from_text",
                json!({ "path": path, "value": value }),
            );
            let (applied, _) =
                post_json_response(&rt, &root, "/api/v1/ops", json!({ "ops": [op] }));
            assert_ne!(applied, axum::http::StatusCode::OK, "{path}");
        }
    }
    let out = fune()
        .arg("-C")
        .arg(&root)
        .args(["set", "harness.H_OBC_CAM.colour", "red"])
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("label, from, to, nets, wire, cores, length"),
        "{err}"
    );
    // 何も書かれていない
    assert_same_files(&root, &root, "refused edits");
}

/// 集まりの名前と同じ ID の部品があれば部品が優先: `harness.mass`（2 区切り）は常に部品のパス、`harness.<X>.<Y>` は
/// 部品があれば部品のパス・無ければレコードの欄。CLI と HTTP でファイルが一致する。
#[test]
fn a_part_named_like_a_collection_keeps_its_own_paths() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    // 部品が無い間は、`harness.<ID>.<欄>` はハーネスの欄
    run_cli(&cli_root, &["set", "harness.H_OBC_CAM.cores", "5"]);
    run_cli(&http_root, &["set", "harness.H_OBC_CAM.cores", "5"]);
    // 部品 `harness` を足す（CLI だけでなく HTTP の側も同じ Op で）
    run_cli(
        &cli_root,
        &["part", "add", "harness", "--catalog", "local/gps_l1"],
    );
    run_cli(
        &http_root,
        &["part", "add", "harness", "--catalog", "local/gps_l1"],
    );
    for (path, value) in [("harness.label", "Harness part"), ("harness.mass", "1 kg")] {
        run_cli(&cli_root, &["set", path, value]);
        let op = post_json(
            &rt,
            &http_root,
            "/api/v1/ops/from_text",
            json!({ "path": path, "value": value }),
        );
        assert_ne!(op["op"], "set_record_field", "{path}");
        post_json(&rt, &http_root, "/api/v1/ops", json!({ "ops": [op] }));
    }
    // 部品があるので 3 区切りも部品のパス（`harness.power.on` は部品 `harness` の属性 power の状態 on）
    let op = post_json(
        &rt,
        &http_root,
        "/api/v1/ops/from_text",
        json!({ "path": "harness.power.on", "value": "1 W" }),
    );
    assert_eq!(op["op"], "set_value", "{op}");
    assert_same_files(&cli_root, &http_root, "part named harness");
    let cam = std::fs::read_to_string(cli_root.join("harness/H_OBC_CAM.toml")).unwrap();
    assert!(cam.contains("cores = 5"), "{cam}");
    // レコードの欄としては書けなくなる（部品のパスとして読み、断られる）
    let out = fune()
        .arg("-C")
        .arg(&cli_root)
        .args(["set", "harness.H_OBC_CAM.cores", "6"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    // 部品の無い集まりは今までどおりレコードの欄
    run_cli(&cli_root, &["set", "sources.ds_rw10.note", "x"]);
}
