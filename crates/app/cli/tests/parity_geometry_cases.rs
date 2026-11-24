//! 最悪の日のケース作成（`fune case from-geometry` / `POST /ops/cases_from_geometry`）が CLI / HTTP で同じファイルを書くこと、
//! 欲しいケースと日付（導ける値）ができること、何度実行しても同じファイルになること、既にあるケースのほかの欄を残すこと。
mod common;
use common::{
    assert_same_files, demo_copy, fune, post_json, post_json_response, run_cli, snapshot_files,
};
use sat_astro::constants::SECONDS_PER_DAY;
use sat_astro::ephem::CentralBody;
use sat_astro::time::Epoch;
use serde_json::json;
use std::path::Path;

/// 太陽を回る軌道（高度 1.64e8 km + 太陽の半径 = 長半径、離心率 0.3636364、元期の平均近点角 0 = 近日点）にし、
/// 通常運用フェーズの期間を 400 日にする（周期 ≈ 422 日なので、期間の中に遠日点が 1 回入る）。
fn cruise_orbit(root: &Path) {
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
    ] {
        run_cli(root, &["setting", "set", path, value]);
    }
    run_cli(root, &["setting", "unset", "conditions.orbit.ltdn"]);
    run_cli(
        root,
        &[
            "phase",
            "set",
            "NOMINAL_OPS",
            "--order",
            "2",
            "--start",
            "Commissioning complete",
            "--end",
            "End of the mission life",
            "--modes",
            "NOMINAL,MISSION,DOWNLINK,SAFE",
            "--duration",
            "400 d",
        ],
    );
}

fn http_ops(
    rt: &tokio::runtime::Runtime,
    root: &Path,
    body: serde_json::Value,
) -> serde_json::Value {
    let ops = post_json(rt, root, "/api/v1/ops/cases_from_geometry", body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops.clone() }));
    ops
}

fn case_epoch(root: &Path, id: &str) -> Epoch {
    let text = std::fs::read_to_string(root.join("cases").join(format!("{id}.toml"))).unwrap();
    let line = text
        .lines()
        .find(|l| l.starts_with("epoch"))
        .expect("epoch line");
    Epoch::parse(line.split('"').nth(1).expect("quoted epoch")).unwrap()
}

#[test]
fn worst_day_cases_match_http_and_are_idempotent() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    cruise_orbit(&cli_root);
    cruise_orbit(&http_root);
    let args = [
        "case",
        "from-geometry",
        "--phase",
        "NOMINAL_OPS",
        "--mode",
        "NOMINAL",
    ];
    let body = json!({ "phase": "NOMINAL_OPS", "mode": "NOMINAL" });
    run_cli(&cli_root, &args);
    let ops = http_ops(&rt, &http_root, body.clone());
    assert_eq!(ops.as_array().unwrap().len(), 3);
    assert_same_files(&cli_root, &http_root, "from-geometry");
    // 遠日点は元期（近日点）の半周後: t = π√(a³/GM)（手計算）。詰め方の限界（値の丸めで決まる時刻のぶれは 1 s 未満）に ISO の 1 ms を足して 10 s
    let a: f64 = 164.0e9 + CentralBody::Sun.radius_m();
    let half = std::f64::consts::PI * (a.powi(3) / CentralBody::Sun.gm()).sqrt();
    let start = Epoch::parse("2027-04-01T00:00:00Z").unwrap();
    let far = case_epoch(&cli_root, "nominal_ops_sun_far");
    assert!(
        (far.seconds_since(start) - half).abs() < 10.0,
        "{}",
        far.seconds_since(start)
    );
    // 近日点は元期そのもの（期間の始め）、地球距離の最大は期間の中
    let near = case_epoch(&cli_root, "nominal_ops_sun_near");
    assert!(near.seconds_since(start).abs() < 10.0);
    let earth = case_epoch(&cli_root, "nominal_ops_earth_far").seconds_since(start);
    assert!((0.0..=400.0 * SECONDS_PER_DAY).contains(&earth), "{earth}");
    // 何度実行しても同じファイル（CLI も HTTP も）
    let before = snapshot_files(&cli_root);
    run_cli(&cli_root, &args);
    http_ops(&rt, &http_root, body.clone());
    assert_eq!(snapshot_files(&cli_root), before);
    assert_same_files(&cli_root, &http_root, "second run");
    // 人が足した欄（ラベル）は残り、書き換えた日付は直る
    for root in [&cli_root, &http_root] {
        run_cli(
            root,
            &[
                "case",
                "set",
                "nominal_ops_sun_far",
                "--label",
                "Aphelion",
                "--mode",
                "SAFE",
                "--phase",
                "NOMINAL_OPS",
                "--epoch",
                "2027-05-01T00:00:00Z",
            ],
        );
    }
    run_cli(
        &cli_root,
        &["case", "from-geometry", "--phase", "NOMINAL_OPS"],
    );
    http_ops(&rt, &http_root, json!({ "phase": "NOMINAL_OPS" }));
    assert_same_files(&cli_root, &http_root, "update");
    let text = std::fs::read_to_string(cli_root.join("cases/nominal_ops_sun_far.toml")).unwrap();
    assert!(
        text.contains("label") && text.contains("Aphelion") && text.contains("SAFE"),
        "{text}"
    );
    assert!((case_epoch(&cli_root, "nominal_ops_sun_far").seconds_since(far)).abs() < 1.0);
}

#[test]
fn request_errors_are_reported() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t, root) = demo_copy();
    cruise_orbit(&root);
    // フェーズがある設計はフェーズを選ぶ。無いフェーズ・無いモード・形式の悪い前置きは拒否する
    for (body, why) in [
        (json!({}), "Choose the phase"),
        (json!({ "phase": "NO_SUCH" }), "phase not found"),
        (
            json!({ "phase": "NOMINAL_OPS", "mode": "NO_SUCH" }),
            "mode not found",
        ),
        (
            json!({ "phase": "NOMINAL_OPS", "prefix": "_bad" }),
            "Invalid input",
        ),
    ] {
        let (status, text) =
            post_json_response(&rt, &root, "/api/v1/ops/cases_from_geometry", body.clone());
        assert_ne!(status, axum::http::StatusCode::OK, "{body}");
        assert!(text.to_string().contains(why), "{body}: {text}");
    }
    let out = fune()
        .arg("-C")
        .arg(&root)
        .args(["case", "from-geometry"])
        .output()
        .unwrap();
    assert!(!out.status.success());
}
