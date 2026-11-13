//! CLI の表示のスナップショット（色なし）。表示を変えたら `cargo insta review` で確かめて更新する。

mod common;

use common::{demo_copy, fune};

fn stdout(args: &[&str]) -> String {
    let out = fune().args(args).output().unwrap();
    assert!(
        out.status.success(),
        "fune {args:?} が失敗しました: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn units_parse() {
    insta::assert_snapshot!(stdout(&[
        "units",
        "parse",
        "18 mN*m*s",
        "--dimension",
        "AngularMomentum"
    ]));
}

#[test]
fn units_ls_power() {
    insta::assert_snapshot!(stdout(&["units", "ls", "--dimension", "Power"]));
}

#[test]
fn analyze_ls() {
    // 表の行末の空白は表示の内容と分け、スナップショットでは正規化する。
    let text = stdout(&["analyze", "ls"])
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(text);
}

#[test]
fn part_ls() {
    let (_tmp, root) = demo_copy();
    insta::assert_snapshot!(stdout(&["-C", root.to_str().unwrap(), "part", "ls"]));
}

#[test]
fn mode_table() {
    let (_tmp, root) = demo_copy();
    insta::assert_snapshot!(stdout(&["-C", root.to_str().unwrap(), "mode", "table"]));
}

#[test]
fn set_dry_run_shows_diff() {
    let (_tmp, root) = demo_copy();
    let text = stdout(&[
        "-C",
        root.to_str().unwrap(),
        "set",
        "RW_X.mass",
        "650 g",
        "--dry-run",
    ]);
    insta::assert_snapshot!(text);
}

#[test]
fn bad_unit_exits_with_2() {
    let out = fune()
        .args(["units", "parse", "3 furlongs"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("✖"), "{err}");
}

#[test]
fn json_error_envelope() {
    let out = fune()
        .args(["--json", "units", "parse", "1 kg", "--dimension", "Power"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["error"]["code"], "unit_error");
}

#[test]
fn check_points_at_the_toml_line() {
    let (_tmp, root) = demo_copy();
    let file = root.join("parts/aocs/RW_X.toml");
    let text = std::fs::read_to_string(&file).unwrap();
    std::fs::write(&file, text.replace("unit = \"kg\"", "unit = \"furlong\"")).unwrap();
    let out = fune()
        .args(["-C", root.to_str().unwrap(), "check"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("parts/aocs/RW_X.toml"), "{err}");
    assert!(err.contains("furlong"), "{err}");
}

#[test]
fn units_format_parity_with_http_shape() {
    let out = stdout(&[
        "--json",
        "units",
        "format",
        "0.0123",
        "--dimension",
        "Power",
    ]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    // ほかの API と同じ量の表示（`QuantityView`）
    assert_eq!(v["items"][0]["display"], "12.3 mW");
    assert_eq!(v["items"][0]["dimension"], "Power");
}

#[test]
fn catalog_case_harness_ls() {
    let (_tmp, root) = demo_copy();
    let r = root.to_str().unwrap();
    let text = [["catalog", "ls"], ["case", "ls"], ["harness", "ls"]]
        .iter()
        .map(|args| stdout(&["-C", r, args[0], args[1]]))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(text);
}

#[test]
fn place_by_body_position() {
    let (_tmp, root) = demo_copy();
    let r = root.to_str().unwrap();
    let out = stdout(&[
        "-C",
        r,
        "--json",
        "part",
        "place",
        "RW_X",
        "--body-pos",
        "10 -20 30 mm",
    ]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["cli"][0].as_str().unwrap().contains("--on"), "{v}");
    let part = stdout(&["-C", r, "--json", "part", "show", "RW_X"]);
    let p: serde_json::Value = serde_json::from_str(&part).unwrap();
    let pos = p["part"]["mount"]["body_position"].as_array().unwrap();
    for (got, want) in pos.iter().zip([0.010, -0.020, 0.030]) {
        assert!((got.as_f64().unwrap() - want).abs() < 1e-6, "{pos:?}");
    }
}
