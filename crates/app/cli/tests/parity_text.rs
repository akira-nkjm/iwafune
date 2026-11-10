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

/// 設定の表（`fune setting set`・`fune setting unset` と `POST /ops/from_text`・`/ops/unset_from_text`）。
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
    let path = "analysis.power.dod_max";
    run_cli(&cli_root, &["setting", "unset", path]);
    let op = http_text(
        &rt,
        &http_root,
        "/api/v1/ops/unset_from_text",
        json!({ "path": path }),
    );
    assert_eq!(op["op"], "set_setting", "{op}");
    assert_same_files(&cli_root, &http_root, "setting unset");
}

/// 部品の値とフィールドの書き込みと消去（`fune set`・`fune unset` と `POST /ops/from_text`・`/ops/unset_from_text`）。
/// GUI の属性の欄は部品のパラメーターパス（`PartRow.paths` の値）と打った文字列をそのまま送る。
/// (CLI の引数, 送り先, 本文)。
fn part_text_edits() -> Vec<(Vec<&'static str>, &'static str, Value)> {
    const SET: &str = "/api/v1/ops/from_text";
    const UNSET: &str = "/api/v1/ops/unset_from_text";
    let set = |path: &'static str, value: &'static str| {
        (
            vec!["set", path, value],
            SET,
            json!({ "path": path, "value": value }),
        )
    };
    let unset = |path: &'static str| (vec!["unset", path], UNSET, json!({ "path": path }));
    vec![
        // スカラー（カタログの値の上書き）
        set("RW_X.mass", "160 g"),
        // 状態ごとの値の 1 セル（部品にまだ上書きの無い属性）
        set("RW_X.power.on", "0.7 W"),
        // 不確かさ（--min / --max）付きの状態のセル
        (
            vec![
                "set",
                "UHF_TRX.power.tx",
                "4.2 W",
                "--min",
                "3.9 W",
                "--max",
                "4.6 W",
            ],
            SET,
            json!({ "path": "UHF_TRX.power.tx", "value": "4.2 W", "min": "3.9 W", "max": "4.6 W" }),
        ),
        // 出典と注記
        (
            vec![
                "set",
                "OBC.mass",
                "0.09 kg",
                "--source",
                "em_test_2026_10",
                "--note",
                "EM 実測",
            ],
            SET,
            json!({ "path": "OBC.mass", "value": "0.09 kg", "source": "em_test_2026_10", "note": "EM 実測" }),
        ),
        // 属性定義の型で読む値（ベクトル・範囲）
        set("OBC.size", "95 95 20 mm"),
        set("OBC.temp_op", "-30 .. 70 degC"),
        // フィールド（文字列・成熟度・員数）
        set("RW_X.label", "X 軸 RW"),
        set("RW_X.maturity", "modified"),
        set("RW_X.count", "2"),
        // 消す: 状態のセル・属性・フィールド
        unset("RW_X.power.on"),
        unset("UHF_TRX.power.tx"),
        unset("OBC.size"),
        unset("RW_X.label"),
        unset("RW_X.maturity"),
    ]
}

#[test]
fn gui_part_text_edits_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    for (cli, uri, body) in part_text_edits() {
        run_cli(&cli_root, &cli);
        let op = http_text(&rt, &http_root, uri, body);
        match cli[1] {
            "RW_X.power.on" if cli[0] == "set" => {
                assert_eq!(op["op"], "set_value", "{op}");
                assert_eq!(op["state"], "on", "{op}");
            }
            "OBC.size" if cli[0] == "set" => assert_eq!(op["value"]["type"], "vec3", "{op}"),
            "OBC.temp_op" => assert_eq!(op["value"]["type"], "range", "{op}"),
            _ => {}
        }
        assert_same_files(&cli_root, &http_root, &cli.join(" "));
    }
    let trx = std::fs::read_to_string(http_root.join("parts/COM/UHF_TRX.toml")).unwrap();
    assert!(!trx.contains("4.2"), "消したセルは残さない:\n{trx}");
    let rw = std::fs::read_to_string(http_root.join("parts/AOCS/RW_X.toml")).unwrap();
    assert!(rw.contains("count") && !rw.contains("label"), "{rw}");
}

/// GUI の部品の基本の欄（カタログの候補の切り替え・候補の一覧・員数・成熟度・説明）。パスは API の
/// `PartRow.field_paths` の値（web はつながない）、値は打った文字列のまま（候補の一覧のカンマ区切りも service が読む）。
#[test]
fn gui_part_field_edits_use_api_paths_and_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let view = post_json(&rt, &http_root, "/api/v1/parts/STX", json!({}));
    let path = |field: &str| {
        view["part"]["field_paths"][field]
            .as_str()
            .unwrap_or_else(|| panic!("field_paths に {field} がありません: {view}"))
            .to_owned()
    };
    let edits = [
        (
            "candidates",
            Some(" local/sband_tx_2m,local/uhf_trx_9k6 , "),
        ),
        ("catalog", Some("local/uhf_trx_9k6")),
        ("count", Some("2")),
        ("maturity", Some("new")),
        ("description", Some("S 帯の送信機")),
        ("catalog", Some("local/sband_tx_2m")),
        ("candidates", None),
        ("count", None),
        ("maturity", None),
        ("description", None),
    ];
    for (field, value) in edits {
        let p = path(field);
        let op = match value {
            Some(v) => {
                run_cli(&cli_root, &["set", &p, v]);
                let body = json!({ "path": p, "value": v });
                http_text(&rt, &http_root, "/api/v1/ops/from_text", body)
            }
            None => {
                run_cli(&cli_root, &["unset", &p]);
                let body = json!({ "path": p });
                http_text(&rt, &http_root, "/api/v1/ops/unset_from_text", body)
            }
        };
        assert_eq!(op["op"], "set_field", "{op}");
        if field == "candidates" && value.is_some() {
            // 一覧の読み方は service の `edit::list_from_text`（カンマで分け、空白を除き、空の語を捨てる）
            let list = &op["field"]["value"];
            assert_eq!(
                list,
                &json!(["local/sband_tx_2m", "local/uhf_trx_9k6"]),
                "{op}"
            );
        }
        assert_same_files(&cli_root, &http_root, &format!("{p} {value:?}"));
    }
    let stx = std::fs::read_to_string(http_root.join("parts/COM/STX.toml")).unwrap();
    assert!(
        !stx.contains("candidates") && !stx.contains("count"),
        "{stx}"
    );
}

/// GUI のパラメーター追加: 属性の定義と値を 1 回の編集で作る。定義は `fune attr add` と同じ打った引数を
/// `POST /ops/from_command` で Op にする。値は定義を書き込まずに当てた設計（`ops`）で読む。
/// パスは、その定義を書き込まずに当てた部品（`POST /parts/{id}` の `ops`）の `paths` から引く（web はつながない）。
/// CLI では `fune attr add` と `fune set` の 2 回。
#[test]
fn gui_new_attribute_with_value_matches_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    run_cli(
        &cli_root,
        &[
            "attr",
            "add",
            "tank_pressure",
            "--label",
            "タンク圧",
            "--dimension",
            "Pressure",
            "--units",
            "Pa,kPa",
            "--by-state",
        ],
    );
    run_cli(&cli_root, &["set", "UHF_TRX.tank_pressure.tx", "120 kPa"]);
    let made = post_json(
        &rt,
        &http_root,
        "/api/v1/ops/from_command",
        json!({ "command": "attr_add", "id": "tank_pressure", "label": "タンク圧",
            "dimension": "Pressure", "units": ["Pa", "kPa"], "by_state": true }),
    );
    let create = made[0].clone();
    assert_eq!(create["op"], "create_attribute", "{made}");
    let draft = post_json(
        &rt,
        &http_root,
        "/api/v1/parts/UHF_TRX",
        json!({ "ops": [create.clone()] }),
    );
    let path = draft["part"]["paths"]["tank_pressure"]["tx"].clone();
    assert_eq!(path, "UHF_TRX.tank_pressure.tx", "{draft}");
    let body = json!({ "path": path, "value": "120 kPa", "ops": [create.clone()] });
    let op = post_json(&rt, &http_root, "/api/v1/ops/from_text", body);
    assert_eq!(op["op"], "set_value", "{op}");
    post_json(
        &rt,
        &http_root,
        "/api/v1/ops",
        json!({ "ops": [create, op] }),
    );
    assert_same_files(&cli_root, &http_root, "attr add + set");
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
