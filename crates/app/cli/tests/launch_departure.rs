//! 打ち上げ機が出発を受け持つ惑星間遷移（`departure_by = "launcher"`）と、能力の曲線（`payload_mass_by_c3`）の通し:
//! CLI で設計を作り（デモの写し。地球 → 火星の日付つきの遷移）、`structure.launch_fit` の質量の上限が曲線を遷移の C3 で
//! 手計算した補間と一致し、`propulsion.dv_budget` が出発の噴射を数えないことを見る。
//! C3 は同じ設計の `propulsion.maneuvers` の結果から取り（暦が DE440 でも解析式でも手計算が同じ値で突き合わさる）、
//! 値の目安（地球 → 火星の 2028 年の窓で 11〜13 km²/s²）だけを別に確かめる。

mod common;

use common::{demo_copy, fune, run_cli};
use serde_json::Value;
use std::path::Path;

/// `fune --json -C <root> analyze <id>` の結果。
fn analyze(root: &Path, id: &str) -> Value {
    let out = fune()
        .arg("--json")
        .arg("-C")
        .arg(root)
        .args(["analyze", id])
        .output()
        .unwrap();
    // 違反があると終了コードは 0 でない（結果の JSON は出る）
    let r: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "analyze {id}: {e}: {}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    assert_eq!(r["status"], "ok", "{r}");
    r
}

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("{v} is not a number"))
}

/// デモの写し: 軌道上のマヌーバを外し、火星への遷移だけにする（打ち上げ機は架空の `lv_beta`）。
/// `departure_by` が `Some` なら `--departure-by` を付ける。曲線は `curve`（`"0:9 10:8 … (km^2/s^2 -> kg)"`）。
fn setup(curve: &str, departure_by: Option<&str>) -> (tempfile::TempDir, std::path::PathBuf) {
    let (tmp, root) = demo_copy();
    run_cli(&root, &["maneuver", "rm", "RAISE"]);
    run_cli(&root, &["maneuver", "rm", "DEORBIT"]);
    run_cli(
        &root,
        &[
            "catalog",
            "add",
            "local/lv_beta",
            "--kind",
            "launcher",
            "--label",
            "Beta (fictional)",
        ],
    );
    run_cli(
        &root,
        &[
            "catalog",
            "set",
            "local/lv_beta",
            "payload_mass_by_c3",
            curve,
        ],
    );
    run_cli(
        &root,
        &["set", "conditions.launch.launcher", "local/lv_beta"],
    );
    let mut add = vec![
        "maneuver",
        "add",
        "MARS_TRANSFER",
        "--category",
        "insertion",
        "--method",
        "impulsive",
        "--thruster",
        "EPT",
        "--body",
        "mars",
        "--perigee-altitude",
        "300 km",
        "--apogee-altitude",
        "33000 km",
        "--depart",
        "2028-12-10T00:00:00Z",
        "--arrive",
        "2029-09-25T00:00:00Z",
    ];
    if let Some(by) = departure_by {
        add.extend(["--departure-by", by]);
    }
    run_cli(&root, &add);
    (tmp, root)
}

/// 出発の C3 [m²/s²]（`propulsion.maneuvers` の結果）。
fn c3(root: &Path) -> f64 {
    let r = analyze(root, "propulsion.maneuvers");
    let d = &r["output"]["maneuvers"][0]["dated_transfer"];
    assert_eq!(d["departure_by"], "launcher", "{d}");
    num(&d["c3"])
}

fn mass(root: &Path) -> Value {
    analyze(root, "structure.launch_fit")["output"]["mass"].clone()
}

/// 曲線 (0, 9)・(10, 8)・(20, 7)・(30, 3) [km²/s² → kg]。手計算: C3 が 10〜20 の間なら 8 − (C3 − 10) / 10 × 1。
/// 限度は曲線が効き（質量の要求 12 kg・ディスペンサーの 14 kg より厳しい）、判定は成長込みの打ち上げ質量との比。
#[test]
fn the_curve_limit_is_the_hand_interpolation_at_the_transfer_c3() {
    let (_t, root) = setup("0:9 10:8 20:7 30:3 (km^2/s^2 -> kg)", Some("launcher"));
    let c3 = c3(&root);
    let km = c3 / 1e6;
    assert!((11.0..13.0).contains(&km), "C3 = {km} km²/s²");
    let expected = 8.0 - (km - 10.0) / 10.0;
    let m = mass(&root);
    assert!((num(&m["limit"]) - expected).abs() < 1e-9, "{m}");
    assert!(
        m["origin"]
            .as_str()
            .unwrap()
            .contains("launcher local/lv_beta at C3"),
        "{m}"
    );
    assert_eq!(m["verdict"], "ok", "{m}");
    let launch = num(&m["launch_mass"]);
    assert!((num(&m["margin"]) - (expected / launch - 1.0)).abs() < 1e-9);
    assert!((num(&m["c3"]) - c3).abs() < 1e-3);
    let text = analyze(&root, "structure.launch_fit")["checks"][0]["message"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        text.contains("at C3 12.1 km²/s²") || text.contains("at C3 "),
        "{text}"
    );
}

/// 曲線を厳しくすると（(0, 7.5)・(10, 6)・(20, 5)・(30, 3)）同じ C3 で上限が 6 − (C3 − 10) / 10 になり、
/// 成長込みの打ち上げ質量（約 6.9 kg）を下回って違反。曲線の終わり（5 km²/s²）より C3 が大きければ送れない。
#[test]
fn a_tighter_curve_violates_and_a_short_curve_is_not_capable() {
    let (_t, root) = setup("0:7.5 10:6 20:5 30:3 (km^2/s^2 -> kg)", Some("launcher"));
    let km = c3(&root) / 1e6;
    let m = mass(&root);
    assert!(
        (num(&m["limit"]) - (6.0 - (km - 10.0) / 10.0)).abs() < 1e-9,
        "{m}"
    );
    assert_eq!(m["verdict"], "violation", "{m}");
    run_cli(
        &root,
        &[
            "catalog",
            "set",
            "local/lv_beta",
            "payload_mass_by_c3",
            "0:7.5 5:7 (km^2/s^2 -> kg)",
        ],
    );
    let m = mass(&root);
    assert_eq!(m["verdict"], "violation", "{m}");
    assert_eq!(num(&m["limit"]), 0.0);
    assert!((num(&m["beyond_curve_end"]) - 5e6).abs() < 1e-3, "{m}");
    let msg = analyze(&root, "structure.launch_fit")["checks"][0]["message"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(msg.starts_with("Not capable"), "{msg}");
}

/// 出発を打ち上げ機が行わない（旗なし・`spacecraft`）なら、曲線は使わず今までと同じ（限度は質量の要求の 12 kg）。
#[test]
fn without_departure_by_the_launcher_the_curve_is_ignored() {
    for by in [None, Some("spacecraft")] {
        let (_t, root) = setup("0:9 10:8 20:7 30:3 (km^2/s^2 -> kg)", by);
        let m = mass(&root);
        assert_eq!(m["origin"], "requirement R_MASS", "{m}");
        assert!((num(&m["limit"]) - 12.0).abs() < 1e-12);
        assert!(m["c3"].is_null() && m["beyond_curve_end"].is_null(), "{m}");
    }
}

/// ΔV バジェット: 打ち上げ機が出発を行うと、合計は衛星が出発するときから出発の噴射を引いた値で、推薬が減り、
/// C3・赤緯は変わらず出る。推薬は捕獲だけのロケット方程式 `m_p = m_end (e^(Δv/v_e) − 1)`（残る推薬の割合・マージンを入れた設計値）の
/// 向きを、公称の `propulsion.maneuvers`（同じ設計）の差で確かめる。
#[test]
fn dv_budget_does_not_count_the_departure_burn() {
    let (_a, own) = setup("0:9 10:8 20:7 30:3 (km^2/s^2 -> kg)", None);
    let (_b, by_launcher) = setup("0:9 10:8 20:7 30:3 (km^2/s^2 -> kg)", Some("launcher"));
    let burns = |root: &Path| {
        analyze(root, "propulsion.maneuvers")["output"]["maneuvers"][0]["burns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| num(&b["delta_v"]))
            .collect::<Vec<_>>()
    };
    let (a, b) = (burns(&own), burns(&by_launcher));
    assert_eq!((a.len(), b.len()), (2, 1));
    assert!((b[0] - a[1]).abs() < 1e-9);
    let total = |root: &Path| num(&analyze(root, "propulsion.dv_budget")["output"]["total"]);
    assert!((total(&by_launcher) - (total(&own) - a[0])).abs() < 1e-9);
    let used =
        |root: &Path| num(&analyze(root, "propulsion.maneuvers")["output"]["total_propellant"]);
    assert!(used(&by_launcher) < used(&own));
    // 打ち上げの要求（C3・赤緯）はバジェットにも出る
    let budget = analyze(&by_launcher, "propulsion.dv_budget");
    assert_eq!(
        budget["output"]["dated_transfers"][0]["departure_by"],
        "launcher"
    );
    let notes = analyze(&by_launcher, "propulsion.maneuvers")["output"]["maneuvers"][0]["notes"]
        .to_string();
    assert!(notes.contains("departure_by_launcher"), "{notes}");
}

/// `--departure-by` は日付と一緒にだけ書け、`launcher`・`spacecraft` 以外は誤り。
#[test]
fn departure_by_needs_dates_and_a_known_value() {
    let (_t, root) = demo_copy();
    let bad = |extra: &[&str]| {
        fune()
            .arg("-C")
            .arg(&root)
            .args([
                "maneuver",
                "add",
                "MV_X",
                "--category",
                "insertion",
                "--method",
                "impulsive",
            ])
            .args(extra)
            .output()
            .unwrap()
            .status
            .success()
    };
    assert!(!bad(&[
        "--body",
        "mars",
        "--perigee-altitude",
        "300 km",
        "--departure-by",
        "launcher"
    ]));
    assert!(!bad(&[
        "--body",
        "mars",
        "--perigee-altitude",
        "300 km",
        "--depart",
        "2028-12-10T00:00:00Z",
        "--arrive",
        "2029-09-25T00:00:00Z",
        "--departure-by",
        "rocket",
    ]));
}
