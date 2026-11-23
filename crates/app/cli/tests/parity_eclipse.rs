//! 季節解析の条件編集とケース作成が CLI / HTTP で同じファイルを書くこと。
mod common;
use common::{assert_same_files, demo_copy, get_json, post_json, run_cli};
use serde_json::json;

#[test]
fn eclipse_settings_and_representative_cases_match_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_cli_temp, cli_root) = demo_copy();
    let (_http_temp, http_root) = demo_copy();
    let (path, value) = ("conditions.orbit.eclipse_season.step", "1 day");
    run_cli(&cli_root, &["setting", "set", path, value]);
    let op = post_json(
        &rt,
        &http_root,
        "/api/v1/ops/from_text",
        json!({"path": path, "value": value}),
    );
    post_json(&rt, &http_root, "/api/v1/ops", json!({"ops": [op]}));
    assert_same_files(&cli_root, &http_root, path);
    for (id, at, mode) in [
        ("season_longest", "longest", Some("NOMINAL")),
        ("season_beta_min", "beta_min", None),
    ] {
        let mut args = vec!["case", "from-eclipse", id, "--at", at];
        if let Some(mode) = mode {
            args.extend(["--mode", mode]);
        }
        run_cli(&cli_root, &args);
        let op = post_json(
            &rt,
            &http_root,
            "/api/v1/ops/case_from_eclipse",
            json!({"id": id, "at": at, "mode": mode}),
        );
        assert_eq!(op["op"], "set_case");
        assert!(op["case"]["beta"]["value"].is_number());
        assert!(op["case"]["description"].as_str().unwrap().contains('Z'));
        post_json(&rt, &http_root, "/api/v1/ops", json!({"ops": [op]}));
        assert_same_files(&cli_root, &http_root, at);
    }
    let design = get_json(&rt, &http_root, "/api/v1/design");
    assert!(
        design["settings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["key"] == "conditions.orbit.eclipse_season.step")
    );
    let run = post_json(
        &rt,
        &http_root,
        "/api/v1/analyses/orbit.eclipse_season",
        json!({}),
    );
    assert_eq!(run["status"], "ok");
    assert_eq!(run["checks"], json!([]));
    assert!(run["verdict"].is_null());
    assert_eq!(run["kpis"].as_array().unwrap().len(), 3);
    assert!(
        run["kpis"]
            .as_array()
            .unwrap()
            .iter()
            .all(|k| k["verdict"].is_null())
    );
}

#[test]
fn missing_duration_is_exposed_by_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_temp, root) = demo_copy();
    // ミッション期間は要求が与える（`input = "mission_duration"`）。その要求が無ければ入力不足
    run_cli(&root, &["requirement", "rm", "R_MISSION"]);
    let run = post_json(
        &rt,
        &root,
        "/api/v1/analyses/orbit.eclipse_season",
        json!({}),
    );
    assert_eq!(run["status"], "missing_inputs");
    assert!(
        run["missing"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["path"] == "requirement[input/mission_duration]")
    );
}

#[test]
fn case_creation_rejects_invalid_id_existing_case_and_unknown_mode() {
    use fune_service::dto::{CaseFromEclipseRequest, EclipseCaseAt};
    use fune_service::edit::case_from_eclipse_op;
    let (_temp, root) = demo_copy();
    let ws = fune_service::Workspace::open(root).unwrap();
    for (id, mode) in [
        ("_invalid", None),
        ("HOT_IMAGING", None),
        ("season_new", Some("UNKNOWN")),
    ] {
        assert!(
            case_from_eclipse_op(
                &ws,
                &CaseFromEclipseRequest {
                    id: id.into(),
                    at: EclipseCaseAt::Longest,
                    mode: mode.map(str::to_owned),
                }
            )
            .is_err()
        );
    }
}
