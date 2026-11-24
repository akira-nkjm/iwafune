//! 解析が使う形態の選択と、保存済みの選択に対する診断。

mod common;

use common::{cli_json, demo_copy, fune, post_json, post_json_response, run_cli};
use serde_json::{Value, json};
use std::path::Path;

fn write_design(root: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
}

fn mass_fixture(root: &Path) {
    write_design(
        root,
        &[
            (
                "fune.toml",
                "[project]\nname = \"Test\"\n[[catalog]]\nname = \"local\"\npath = \"catalog\"\n",
            ),
            (
                "parts/A.toml",
                "kind = \"structure\"\n[attr]\nmass = { value = 4.0, unit = \"kg\" }\n",
            ),
            (
                "parts/B.toml",
                "kind = \"structure\"\n[mount]\non = \"A\"\n[pose]\nposition = { x = 0.0, y = 0.2, z = 0.0, unit = \"m\" }\n[attr]\nmass = { value = 1.0, unit = \"kg\" }\n",
            ),
            (
                "configurations/STOWED.toml",
                "[pose.B]\nposition = { x = 0.05, y = 0.0, z = 0.0, unit = \"m\" }\n",
            ),
            ("configurations/DEPLOYED.toml", "label = \"Deployed\"\n"),
            ("phases/EARLY.toml", "configuration = \"STOWED\"\n"),
            ("phases/LATER.toml", "label = \"Later\"\n"),
            ("cases/early.toml", "phase = \"EARLY\"\n"),
            ("cases/later.toml", "phase = \"LATER\"\n"),
            (
                "catalog/vehicle.toml",
                "kind = \"launcher\"\n[attr]\nmax_payload_mass = { value = 10.0, unit = \"kg\" }\n",
            ),
            (
                "conditions/launch.toml",
                "configuration = \"STOWED\"\nlauncher = \"local/vehicle\"\nload_axial = { value = 6.0, unit = \"g0\" }\nload_lateral = { value = 2.0, unit = \"g0\" }\n",
            ),
        ],
    );
}

fn assert_vector(actual: &Value, expected: [f64; 3]) {
    for (axis, expected) in ["x", "y", "z"].into_iter().zip(expected) {
        let value = actual[axis].as_f64().unwrap_or_else(|| panic!("{actual}"));
        assert!(
            (value - expected).abs() < 1e-9,
            "{axis}: {value} != {expected}"
        );
    }
}

/// 重心は 0.2 / 5 = 0.04 m（基準 y）と 0.05 / 5 = 0.01 m（収納 x）。
#[test]
fn mass_properties_use_phase_or_explicit_configuration() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    mass_fixture(root);
    let id = "structure.mass_properties";
    let base = cli_json(root, &["analyze", id]);
    let later = cli_json(root, &["analyze", id, "--case", "later"]);
    for run in [base, later] {
        assert!(run.get("configuration").is_none());
        assert!(run.get("configuration_source").is_none());
        assert_vector(
            &run["output"]["summary"]["center_of_mass"],
            [0.0, 0.04, 0.0],
        );
    }
    let early = cli_json(root, &["analyze", id, "--case", "early"]);
    assert_eq!(early["configuration"], "STOWED");
    assert_eq!(early["configuration_source"], "phase");
    assert_vector(
        &early["output"]["summary"]["center_of_mass"],
        [0.01, 0.0, 0.0],
    );

    let cli = cli_json(
        root,
        &[
            "analyze",
            id,
            "--case",
            "early",
            "--configuration",
            "DEPLOYED",
        ],
    );
    let http = post_json(
        &rt,
        root,
        "/api/v1/analyses/structure.mass_properties",
        json!({ "case": "early", "configuration": "DEPLOYED" }),
    );
    assert_eq!(cli, http);
    assert_eq!(cli["configuration"], "DEPLOYED");
    assert_eq!(cli["configuration_source"], "explicit");
    assert_vector(
        &cli["output"]["summary"]["center_of_mass"],
        [0.0, 0.04, 0.0],
    );

    let unknown = fune()
        .arg("-C")
        .arg(root)
        .args(["analyze", id, "--configuration", "UNKNOWN"])
        .output()
        .unwrap();
    assert!(!unknown.status.success());
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("configuration not found: UNKNOWN"));
    let (status, error) = post_json_response(
        &rt,
        root,
        "/api/v1/analyses/structure.mass_properties",
        json!({ "configuration": "UNKNOWN" }),
    );
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "not_found");
}

/// 打ち上げ向けの射影はフェーズの選択を読まない。
#[test]
fn launch_analyses_use_launch_selection_or_base() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    mass_fixture(root);
    run_cli(
        root,
        &["phase", "set", "EARLY", "--configuration", "DEPLOYED"],
    );
    for id in ["structure.launch_fit", "structure.static_load"] {
        let launch = cli_json(root, &["analyze", id, "--case", "early"]);
        assert_eq!(launch["configuration"], "STOWED");
        assert_eq!(launch["configuration_source"], "launch");
        let explicit = cli_json(root, &["analyze", id, "--configuration", "DEPLOYED"]);
        assert_eq!(explicit["configuration"], "DEPLOYED");
        assert_eq!(explicit["configuration_source"], "explicit");
    }
    run_cli(root, &["unset", "conditions.launch.configuration"]);
    for id in ["structure.launch_fit", "structure.static_load"] {
        let base = cli_json(root, &["analyze", id, "--case", "early"]);
        assert!(base.get("configuration").is_none());
        assert!(base.get("configuration_source").is_none());
    }
}

/// 既定ケースを姿勢のモードに借りるときも、上段のパネルの配置は実行全体の形態に従う。
#[test]
fn energy_balance_uses_the_runs_configuration_with_attitude_cases() {
    let (_tmp, root) = demo_copy();
    run_cli(&root, &["configuration", "add", "PACKED"]);
    run_cli(
        &root,
        &[
            "part",
            "place",
            "SAP_PY",
            "--configuration",
            "PACKED",
            "--rotation",
            "0 0 -90 deg",
        ],
    );
    run_cli(
        &root,
        &["phase", "set", "NOMINAL_OPS", "--configuration", "PACKED"],
    );
    run_cli(
        &root,
        &["set", "analysis.common.default_case", "hot_imaging"],
    );
    let normal = |run: &Value| {
        run["input"]["array"]["panels"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == "CELL_MOD_1")
            .unwrap()["normal"]
            .clone()
    };
    let id = "power.energy_balance";
    let base = cli_json(&root, &["analyze", id]);
    assert_eq!(base["case_source"], "default_attitude");
    assert!(base.get("configuration").is_none());
    assert_vector(&normal(&base), [1.0, 0.0, 0.0]);
    let phased = cli_json(&root, &["analyze", id, "--case", "hot_imaging"]);
    assert_eq!(phased["configuration_source"], "phase");
    assert_vector(&normal(&phased), [0.0, -1.0, 0.0]);
    let explicit = cli_json(&root, &["analyze", id, "--configuration", "PACKED"]);
    assert_eq!(explicit["configuration_source"], "explicit");
    assert_vector(&normal(&explicit), [0.0, -1.0, 0.0]);
}

/// 無効な保存済みの選択は検査のエラーになり、射影から両方の選択の候補を返せる。
#[test]
fn invalid_saved_selections_point_to_their_fields_and_offer_choices() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    mass_fixture(root);
    run_cli(root, &["unset", "conditions.launch.configuration"]);
    let report = cli_json(root, &["check"]);
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| { d["code"] == "launch.configuration_unset" && d["severity"] == "info" })
    );
    write_design(
        root,
        &[
            ("conditions/launch.toml", "configuration = \"UNKNOWN\"\n"),
            (
                "phases/EARLY.toml",
                "label = \"Early\"\nconfiguration = \"UNKNOWN\"\n",
            ),
        ],
    );
    let out = fune()
        .arg("-C")
        .arg(root)
        .args(["check", "--json"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    for (id, path, line) in [
        (
            "structure.static_load",
            "conditions.launch.configuration",
            1,
        ),
        ("structure.mass_properties", "phases.EARLY.configuration", 2),
    ] {
        assert!(
            report["diagnostics"].as_array().unwrap().iter().any(|d| {
                d["code"] == "ref.configuration"
                    && d["severity"] == "error"
                    && d["path"] == path
                    && d["line"] == line
            }),
            "{report}"
        );
        let result = post_json(
            &rt,
            root,
            &format!("/api/v1/analyses/{id}"),
            json!({ "case": "early" }),
        );
        assert_eq!(result["status"], "missing_inputs");
        assert_eq!(result["missing"][0]["path"], path);
        let fix = &result["missing"][0]["fixes"][0];
        assert_eq!(fix["kind"], "pick_ref");
        assert_eq!(fix["what"], "configuration");
        assert_eq!(
            fix["choices"],
            json!([
                { "value": "DEPLOYED", "label": "Deployed", "label_en": null },
                { "value": "STOWED", "label": "STOWED", "label_en": null },
            ])
        );
    }
}
