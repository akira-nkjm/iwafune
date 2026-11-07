//! パリティテスト（CI 必須）: 人が打った文字列の編集。GUI は打った文字列を `POST /ops/from_text`
//! （消すときは `POST /ops/unset_from_text`）で Op にしてから `POST /ops` で送る。値の型（整数・小数・量・ベクトル・範囲・
//! 状態ごとのセル・不確かさ・フィールド）を決めるのは CLI と同じ service の関数なので、書かれるファイルもバイト単位で一致する。

mod common;

use common::{assert_same_files, demo_copy, post_json, run_cli};
use serde_json::{Value, json};
use std::path::Path;

/// 打った文字列 → Op（HTTP）→ 適用（HTTP）。Op を返す。
fn http_text(rt: &tokio::runtime::Runtime, root: &Path, uri: &str, body: Value) -> Value {
    let op = post_json(rt, root, uri, body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": [op.clone()] }));
    op
}

/// 設定の表（`fune setting set` と `POST /ops/from_text`）。
#[test]
fn gui_text_edits_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let cases = [
        ("analysis.power.orbits", "12"),
        ("analysis.power.dod_max", "0.3"),
        ("mission.orbit.altitude", "560 km"),
        ("mission.orbit.ltdn", "11:00"),
    ];
    for (path, value) in cases {
        run_cli(&cli_root, &["setting", "set", path, value]);
        http_text(
            &rt,
            &http_root,
            "/api/v1/ops/from_text",
            json!({ "path": path, "value": value }),
        );
        assert_same_files(&cli_root, &http_root, path);
    }
    let power = std::fs::read_to_string(http_root.join("analysis/power.toml")).unwrap();
    assert!(power.contains("= 12\n"), "周回数は整数のまま書く:\n{power}");
}

/// カタログ品の値とフィールドの書き込み（`fune catalog set` と `POST /ops/from_text` の `item`）。
/// (CLI の引数, `/ops/from_text` の本文)。
fn catalog_sets() -> Vec<(Vec<&'static str>, Value)> {
    const ITEM: &str = "local/rw_10mNms";
    let set = |path: &'static str, value: &'static str| {
        (
            vec!["catalog", "set", ITEM, path, value],
            json!({ "item": ITEM, "path": path, "value": value }),
        )
    };
    vec![
        // 属性の値（定数のスカラー）
        set("mass", "160 g"),
        // 状態ごとの値の 1 セル。GUI は品のパラメーターパス（`CatalogItemView.paths` の値）を送る
        (
            vec!["catalog", "set", ITEM, "power.idle", "0.2 W"],
            json!({ "item": ITEM, "path": "local/rw_10mNms.power.idle", "value": "0.2 W" }),
        ),
        // 不確かさ（--min / --max）
        (
            vec![
                "catalog", "set", ITEM, "power.on", "0.65 W", "--min", "0.55 W", "--max", "0.8 W",
            ],
            json!({ "item": ITEM, "path": "power.on", "value": "0.65 W", "min": "0.55 W", "max": "0.8 W" }),
        ),
        // 属性定義の型で読む値（ベクトル・範囲）
        set("size", "52 50 30 mm"),
        set("temp_op", "-25 .. 60 degC"),
        // 品にまだ無い属性
        set("heat_fraction", "0.9"),
        // フィールド（文字列・成熟度）
        set("part_number", "RW-10B"),
        set("label_en", "RW 10 mNms class"),
        set("maturity", "modified"),
    ]
}

/// カタログ品の値とフィールドを消す（`fune catalog unset` と `POST /ops/unset_from_text` の `item`）、
/// と部品の値を消す（`fune unset`）。
fn unsets() -> Vec<(Vec<&'static str>, Value)> {
    const ITEM: &str = "local/rw_10mNms";
    let unset = |path: &'static str| {
        (
            vec!["catalog", "unset", ITEM, path],
            json!({ "item": ITEM, "path": path }),
        )
    };
    vec![
        unset("hk_rate"),
        (
            vec!["catalog", "unset", ITEM, "power.peak"],
            json!({ "item": ITEM, "path": "local/rw_10mNms.power.peak" }),
        ),
        unset("datasheet"),
        (vec!["unset", "RW_X.mass"], json!({ "path": "RW_X.mass" })),
    ]
}

#[test]
fn gui_catalog_text_edits_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    for (cli, body) in catalog_sets() {
        run_cli(&cli_root, &cli);
        let op = http_text(&rt, &http_root, "/api/v1/ops/from_text", body);
        if cli[3] == "size" {
            // web が量として組み立てていた不具合の再発防止: 属性定義（vec3）の型で読む
            assert_eq!(op["value"]["type"], "vec3", "{op}");
        }
        assert_same_files(&cli_root, &http_root, &cli.join(" "));
    }
    for (cli, body) in unsets() {
        run_cli(&cli_root, &cli);
        http_text(&rt, &http_root, "/api/v1/ops/unset_from_text", body);
        assert_same_files(&cli_root, &http_root, &cli.join(" "));
    }
    let item = std::fs::read_to_string(http_root.join("catalog/rw_10mNms.toml")).unwrap();
    assert!(
        item.contains("min = 0.55") && item.contains("max = 0.8"),
        "不確かさも書く:\n{item}"
    );
    assert!(!item.contains("hk_rate"), "消した値は残さない:\n{item}");
}
