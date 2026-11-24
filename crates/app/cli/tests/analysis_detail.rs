//! 重い任意の出力（`fune analyze <id> --detail` と `POST /analyses/{id}` の `detail`）: 既定の結果には出ず、頼むと
//! CLI と HTTP が同じものを返す。解析 `structure.modal_fem` のモードの形と、`thermal.steady`・`thermal.transient` の板の温度の分布で確かめる。
mod common;
use common::{demo_copy, fune, post_json, run_cli};
use serde_json::{Value, json};
use std::path::Path;

fn analyze(root: &Path, id: &str, detail: bool) -> Value {
    let mut args = vec!["-C", root.to_str().unwrap(), "--json", "analyze", id];
    if detail {
        args.push("--detail");
    }
    let out = fune().args(args).output().unwrap();
    assert!(
        out.status.success() || out.status.code() == Some(1),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn detail_is_opt_in_and_the_same_over_cli_and_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_temp, root) = demo_copy();
    let id = "structure.modal_fem";

    let plain = analyze(&root, id, false);
    assert_eq!(plain["status"], "ok");
    assert!(plain.get("detail").is_none());

    let cli = analyze(&root, id, true);
    let http = post_json(
        &rt,
        &root,
        &format!("/api/v1/analyses/{id}"),
        json!({ "detail": true }),
    );
    let http_plain = post_json(&rt, &root, &format!("/api/v1/analyses/{id}"), json!({}));
    assert!(http_plain.get("detail").is_none());
    assert_eq!(http_plain, plain, "the default result is the same");

    let detail = &cli["detail"];
    assert_eq!(
        detail, &http["detail"],
        "CLI and HTTP return the same detail"
    );
    assert_eq!(detail["normalization"], "max_translation");
    let nodes = detail["mesh"]["nodes"].as_array().unwrap().len() as u64;
    assert_eq!(nodes, plain["output"]["model"]["nodes"].as_u64().unwrap());
    assert_eq!(
        detail["modes"].as_array().unwrap().len(),
        plain["output"]["modes"].as_array().unwrap().len()
    );
    // 詳細を頼んでも、ほかの欄は変わらない
    let mut without = cli.clone();
    without.as_object_mut().unwrap().remove("detail");
    assert_eq!(without, plain);
}

#[test]
fn detail_is_ignored_by_analyses_without_one() {
    let (_temp, root) = demo_copy();
    let id = "structure.mass_properties";
    assert_eq!(analyze(&root, id, true), analyze(&root, id, false));
}

/// デモの 2 枚のパネルを升に分ける（まとめた節点から外して、3 × 5 と 4 × 4 に分ける）。
fn subdivide_two_panels(root: &Path) {
    for part in ["PANEL_PX", "PANEL_MZ"] {
        run_cli(
            root,
            &[
                "setting",
                "unset",
                &format!("analysis.thermal.nodes.{part}"),
            ],
        );
    }
    for (part, u, v) in [("PANEL_PX", "3", "5"), ("PANEL_MZ", "4", "4")] {
        for (axis, n) in [("u", u), ("v", v)] {
            run_cli(
                root,
                &[
                    "setting",
                    "set",
                    &format!("analysis.thermal.subdivide.{part}.{axis}"),
                    n,
                ],
            );
        }
    }
}

#[test]
fn thermal_plate_detail_is_the_same_over_cli_and_http_and_matches_the_nodes() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_temp, root) = demo_copy();
    subdivide_two_panels(&root);
    for id in ["thermal.steady", "thermal.transient"] {
        let plain = analyze(&root, id, false);
        assert_eq!(plain["status"], "ok", "{id}");
        assert!(plain.get("detail").is_none());
        let cli = analyze(&root, id, true);
        let http = post_json(
            &rt,
            &root,
            &format!("/api/v1/analyses/{id}"),
            json!({ "detail": true }),
        );
        let detail = &cli["detail"];
        assert_eq!(
            detail, &http["detail"],
            "{id}: CLI and HTTP return the same detail"
        );
        // 升の数 3 × 5 + 4 × 4 = 31。メッシュの頂点は (3+1)(5+1) + (4+1)(4+1) = 49、四角形は板ごとに升の数
        assert_eq!(detail["cells"].as_array().unwrap().len(), 31);
        assert_eq!(detail["mesh"]["nodes"].as_array().unwrap().len(), 49);
        let groups = detail["mesh"]["groups"].as_array().unwrap();
        let quads: Vec<_> = groups
            .iter()
            .map(|g| {
                (
                    g["id"].as_str().unwrap(),
                    g["quads"].as_array().unwrap().len(),
                )
            })
            .collect();
        assert_eq!(quads, [("PANEL_MZ", 16), ("PANEL_PX", 15)], "{id}");
        // 場の値は、結果の節点（升の ID）の値と同じ
        let output = &plain["output"];
        let value = |node: &str, field: &str, case: Option<&str>| -> f64 {
            if id == "thermal.steady" {
                let c = output["cases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|c| c["id"] == case.unwrap())
                    .unwrap();
                c["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|n| n["node"] == node)
                    .unwrap()["temperature"]
                    .as_f64()
                    .unwrap()
            } else if field == "end" {
                let ch = output["series"]["channels"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|c| c["key"] == node)
                    .unwrap();
                ch["values"]
                    .as_array()
                    .unwrap()
                    .last()
                    .unwrap()
                    .as_f64()
                    .unwrap()
            } else {
                output["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|n| n["node"] == node)
                    .unwrap()[field]
                    .as_f64()
                    .unwrap()
            }
        };
        let fields = detail["fields"].as_array().unwrap();
        for f in fields {
            let quantity = f["quantity"].as_str().unwrap();
            let temps = f["temperature"].as_array().unwrap();
            assert_eq!(temps.len(), 31);
            for (cell, t) in detail["cells"].as_array().unwrap().iter().zip(temps) {
                let node = cell["node"].as_str().unwrap();
                let expected = value(node, quantity, f["case"].as_str());
                assert_eq!(t.as_f64().unwrap(), expected, "{id} {quantity} {node}");
            }
        }
        // 升の温度は一様ではない（機器の取付の位置で伝導が配られる）
        let first = fields[0]["temperature"].as_array().unwrap();
        let (lo, hi) = first
            .iter()
            .map(|t| t.as_f64().unwrap())
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), t| {
                (lo.min(t), hi.max(t))
            });
        assert!(hi - lo > 1.0, "{id}: cells differ ({lo} to {hi})");
        // 詳細を頼んでも、ほかの欄は変わらない
        let mut without = cli.clone();
        without.as_object_mut().unwrap().remove("detail");
        assert_eq!(without, plain, "{id}");
    }
    let steady = analyze(&root, "thermal.steady", true);
    let quantities: Vec<_> = steady["detail"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["quantity"].as_str().unwrap())
        .collect();
    assert!(quantities.iter().all(|q| *q == "steady"));
    assert_eq!(
        quantities.len(),
        steady["output"]["cases"].as_array().unwrap().len()
    );
    let transient = analyze(&root, "thermal.transient", true);
    let quantities: Vec<_> = transient["detail"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["quantity"].as_str().unwrap())
        .collect();
    assert_eq!(quantities, ["min", "max", "end"]);
}

#[test]
fn thermal_detail_is_absent_without_subdivision() {
    let (_temp, root) = demo_copy();
    for id in ["thermal.steady", "thermal.transient"] {
        let with = analyze(&root, id, true);
        assert!(with.get("detail").is_none(), "{id}");
        assert_eq!(with, analyze(&root, id, false), "{id}");
    }
}
