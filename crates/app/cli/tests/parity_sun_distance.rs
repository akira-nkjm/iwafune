//! 太陽距離（近日点・遠日点）のケース作成が CLI / HTTP で同じファイルを書くこと。
mod common;
use common::{assert_same_files, demo_copy, post_json, run_cli};
use serde_json::json;

/// 太陽を回る軌道（長半径 ≈ 1.1 AU・離心率 0.3636）に直す。設定は CLI で両方に同じく書く。
fn cruise_orbit(root: &std::path::Path) {
    run_cli(
        root,
        &[
            "setting",
            "set",
            "conditions.orbit.sun_synchronous",
            "false",
        ],
    );
    for (path, value) in [
        ("conditions.orbit.central", "sun"),
        ("conditions.orbit.altitude", "164000000 km"),
        ("conditions.orbit.eccentricity", "0.3636364"),
        ("conditions.orbit.inclination", "1 deg"),
        ("conditions.orbit.duration", "1 yr"),
    ] {
        run_cli(root, &["setting", "set", path, value]);
    }
    run_cli(root, &["setting", "unset", "conditions.orbit.ltdn"]);
}

#[test]
fn sun_distance_cases_match_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_cli_temp, cli_root) = demo_copy();
    let (_http_temp, http_root) = demo_copy();
    cruise_orbit(&cli_root);
    cruise_orbit(&http_root);
    for (id, at, mode, au) in [
        ("perihelion", "min", Some("NOMINAL"), 0.70),
        ("aphelion", "max", None, 1.50),
    ] {
        let mut args = vec!["case", "from-sun-distance", id, "--at", at];
        if let Some(mode) = mode {
            args.extend(["--mode", mode]);
        }
        run_cli(&cli_root, &args);
        let op = post_json(
            &rt,
            &http_root,
            "/api/v1/ops/case_from_sun_distance",
            json!({"id": id, "at": at, "mode": mode}),
        );
        assert_eq!(op["op"], "set_case");
        assert_eq!(op["case"]["sun_distance"]["unit"], "AU");
        let value = op["case"]["sun_distance"]["value"].as_f64().unwrap();
        assert!((value - au).abs() < 0.01, "{at}: {value} AU");
        assert!(op["case"]["description"].as_str().unwrap().contains('Z'));
        post_json(&rt, &http_root, "/api/v1/ops", json!({"ops": [op]}));
        assert_same_files(&cli_root, &http_root, at);
    }
}

#[test]
fn earth_orbit_is_rejected() {
    use fune_service::dto::{CaseFromSunDistanceRequest, SunDistanceCaseAt};
    use fune_service::edit::case_from_sun_distance_op;
    let (_temp, root) = demo_copy();
    let ws = fune_service::Workspace::open(root).unwrap();
    let req = |id: &str| CaseFromSunDistanceRequest {
        id: id.into(),
        at: SunDistanceCaseAt::Min,
        mode: None,
    };
    // 地球周回（デモの軌道）には近日点が無い
    assert!(case_from_sun_distance_op(&ws, &req("cruise_min")).is_err());
    // ID の形式・既存のケース
    for id in ["_invalid", "HOT_IMAGING"] {
        assert!(case_from_sun_distance_op(&ws, &req(id)).is_err());
    }
}
