//! 判定の基準は要求が持つ（`fune_model::design::criteria`）: 要求の値が解析の入力に届くこと、同じ基準を持つ要求が複数なら
//! 最も厳しいものが効くこと、要求が無いときは設定が無かったときと同じ振る舞いになること。
//! 宣言の表（`CRITERIA`）の解析 ID が実在し、判定のキーの書式が解析の出力と合っていること。

use fune_model::Design;
use fune_model::design::{CRITERIA, Requirement};
use fune_projection::{Context, project_json};
use serde_json::{Value, json};
use std::path::Path;

fn demo() -> Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root)
        .expect("the demo should load")
        .design
}

fn requirement(id: &str, quantity: &str, kind: &str, value: Value, verify: Value) -> Requirement {
    serde_json::from_value(json!({
        "id": id, "statement": "s", "quantity": quantity,
        "constraint": {"kind": kind, "value": value}, "verify": verify
    }))
    .unwrap()
}

fn with(design: &mut Design, r: Requirement) {
    design.requirements.insert(r.id.clone(), r);
}

/// 解析 `id` の入力（JSON）。足りない入力があれば、そのパスを返す。
fn input(design: &Design, id: &str) -> Result<Value, Vec<String>> {
    let resolved = fune_model::resolve(design);
    let ctx = Context::new(design, &resolved, None).unwrap();
    project_json(id, &ctx).map_err(|e| match e {
        fune_projection::ProjectionError::Missing(m) => {
            m.into_vec().into_iter().map(|m| m.path).collect()
        }
        other => panic!("{other}"),
    })
}

fn close(a: &Value, want: f64) {
    let a = a.as_f64().unwrap_or_else(|| panic!("{a} is not a number"));
    assert!(
        (a - want).abs() < 1e-9 * want.abs().max(1.0),
        "{a} != {want}"
    );
}

/// デモの要求の値が、それぞれの解析の入力に届く。
#[test]
fn the_demo_requirements_reach_the_analysis_inputs() {
    let d = demo();
    // DoD の上限 30 %（R_DOD）→ 電池の入力
    let eb = input(&d, "power.energy_balance").unwrap();
    close(&eb["batteries"][0]["dod_limit"], 0.3);
    // 姿勢変更の時間 120 s（R_SLEW）
    close(
        &input(&d, "aocs.slew_time").unwrap()["required_time"],
        120.0,
    );
    // 寿命 25 年（R_LIFETIME）とミッション期間 1 年（R_MISSION）
    let decay = input(&d, "orbit.decay").unwrap();
    close(&decay["max_lifetime"], 25.0 * 365.25 * 86400.0);
    close(&decay["mission_duration"], 365.25 * 86400.0);
    // 回線余裕 3 dB（R_LINK_MARGIN。回線 × 局ごと）と PFD の限度 −135 dB（R_PFD。下りの s_down だけ）
    let links = input(&d, "comm.link_budget").unwrap();
    for l in links["links"].as_array().unwrap() {
        close(&l["required_margin_db"], 3.0);
        let pfd = &l["pfd_limit_db"];
        if l["name"] == "s_down" {
            close(pfd, -135.0);
        } else {
            assert!(pfd.is_null(), "{l}");
        }
    }
    // 最低固有振動数（R_FREQ_*）
    let modal = input(&d, "structure.modal_estimate").unwrap();
    close(&modal["requirements"]["axial"], 100.0);
    close(&modal["requirements"]["lateral"], 50.0);
    // 質量の上限 12 kg（R_MASS）→ 構造の質量特性
    close(
        &input(&d, "structure.mass_properties").unwrap()["max_mass"],
        12.0,
    );
    // 温度の要求（R_BAT_TEMP）は節点 BAT の動作範囲の下限に重なる
    let thermal = input(&d, "thermal.steady").unwrap();
    let bat = thermal["network"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "BAT")
        .unwrap();
    close(&bat["required"]["operating"]["min"]["limit"], 273.15);
    assert_eq!(
        bat["required"]["operating"]["min"]["requirement"],
        "R_BAT_TEMP"
    );
}

/// 要求の値を変えると入力が変わる（設定ではなく要求が基準の置き場所）。
#[test]
fn editing_the_requirement_changes_the_input() {
    let mut d = demo();
    let dod = requirement(
        "R_DOD",
        "Dimensionless",
        "at_most",
        json!({"value": 25.0, "unit": "%"}),
        json!({"analysis": "power.energy_balance", "check": "dod"}),
    );
    with(&mut d, dod);
    close(
        &input(&d, "power.energy_balance").unwrap()["batteries"][0]["dod_limit"],
        0.25,
    );
}

/// 同じ基準に要求が複数あれば最も厳しいものが効く（`at_least` は最大）。回線 × 局の個別のキーは族より優先して効くのではなく、
/// 厳しい方が効く。
#[test]
fn the_tightest_requirement_wins_and_keys_cover_families() {
    let mut d = demo();
    let margin = |id: &str, db: f64, check: &str| {
        requirement(
            id,
            "Dimensionless",
            "at_least",
            json!({"value": db, "unit": "dB"}),
            json!({"analysis": "comm.link_budget", "check": check}),
        )
    };
    // 族（すべて 3 dB）に、s_down だけ 6 dB と、uhf_up だけ 1 dB（緩いので効かない）
    with(&mut d, margin("R_S_DOWN", 6.0, "margin:s_down"));
    with(&mut d, margin("R_UHF_UP", 1.0, "margin:uhf_up@TOKYO_GS"));
    let links = input(&d, "comm.link_budget").unwrap();
    let required = |name: &str| {
        let l = links["links"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["name"] == name)
            .unwrap();
        l["required_margin_db"].as_f64().unwrap()
    };
    assert!((required("s_down") - 6.0).abs() < 1e-9);
    assert!((required("uhf_up") - 3.0).abs() < 1e-9);
    assert!((required("uhf_down") - 3.0).abs() < 1e-9);
    // 要求の兄弟（同じ基準に効くほかの要求）は、適合表が要求ごとに判定し直すときに外す
    let r6 = d.requirements.get("R_S_DOWN").unwrap();
    let mut siblings = d.criterion_siblings(r6);
    siblings.sort();
    assert_eq!(siblings, ["R_LINK_MARGIN", "R_LINK_S"]);
}

/// 要求が無いときは、設定が無かったときと同じ。DoD は必須（入力不足）、ほかは判定を作らない（解析の既定があれば既定）。
#[test]
fn without_a_requirement_the_analysis_behaves_as_without_the_setting() {
    let mut d = demo();
    for id in [
        "R_DOD",
        "R_SLEW",
        "R_LIFETIME",
        "R_LINK_MARGIN",
        "R_MISSION",
    ] {
        d.requirements.remove(id);
    }
    let missing = input(&d, "power.energy_balance").unwrap_err();
    assert!(
        missing.contains(&"requirement[power.energy_balance/dod]".to_owned()),
        "{missing:?}"
    );
    assert!(input(&d, "aocs.slew_time").unwrap()["required_time"].is_null());
    assert!(input(&d, "orbit.decay").unwrap()["max_lifetime"].is_null());
    assert!(input(&d, "orbit.decay").unwrap()["mission_duration"].is_null());
    // 回線余裕は要求が無ければ入力は空で、解析の既定（3 dB）が使われる
    let links = input(&d, "comm.link_budget").unwrap();
    let uhf = links["links"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["name"] == "uhf_down")
        .unwrap();
    assert!(uhf["required_margin_db"].is_null());
    // ミッション期間は季節の解析に必要
    let missing = input(&d, "orbit.eclipse_season").unwrap_err();
    assert!(missing.contains(&"requirement[input/mission_duration]".to_owned()));
}

/// 向きか次元が合わない要求は基準にならない（`fune check` が知らせる）。
#[test]
fn a_requirement_of_the_wrong_direction_is_not_a_criterion() {
    let mut d = demo();
    let bad = requirement(
        "R_SLEW",
        "Time",
        "at_least",
        json!({"value": 10.0, "unit": "s"}),
        json!({"analysis": "aocs.slew_time", "check": "slew_time"}),
    );
    with(&mut d, bad);
    assert!(input(&d, "aocs.slew_time").unwrap()["required_time"].is_null());
    assert!(
        fune_model::check(&d)
            .iter()
            .any(|x| x.code == "requirement.criterion_kind")
    );
}

/// 宣言の表の解析 ID は解析の一覧にあり、判定のキーの族は解析が実際に作る判定のキーに当たる（デモを実行して確かめる）。
#[test]
fn declared_criteria_name_real_analyses_and_produced_keys() {
    let ids: Vec<String> = sat_analysis::catalog::catalog()
        .into_iter()
        .map(|i| i.id)
        .collect();
    for c in CRITERIA {
        assert!(ids.iter().any(|i| i == c.analysis), "{}", c.analysis);
    }
    let d = demo();
    let resolved = fune_model::resolve(&d);
    let ctx = Context::new(&d, &resolved, None).unwrap();
    let produced = |analysis: &str| -> Vec<String> {
        let input = project_json(analysis, &ctx).unwrap();
        let out = sat_analysis::catalog::run_json(analysis, input).unwrap();
        let mut keys = Vec::new();
        collect_keys(&out, &mut keys);
        keys
    };
    // 判定が出るデモの解析: 宣言した族のどれかに当たる判定が少なくとも 1 つある
    for (analysis, family) in [
        ("power.energy_balance", "dod"),
        ("comm.link_budget", "margin"),
        ("comm.link_budget", "pfd"),
        ("aocs.slew_time", "slew_time"),
        ("orbit.decay", "max_lifetime"),
        ("thermal.steady", "range_low"),
        ("thermal.steady", "range_high"),
    ] {
        assert!(
            CRITERIA
                .iter()
                .any(|c| c.analysis == analysis && c.check == family),
            "{analysis}/{family} is not declared"
        );
        let keys = produced(analysis);
        assert!(
            keys.iter()
                .any(|k| fune_model::design::key_covers(family, k)),
            "{analysis} produced {keys:?}, none under `{family}`"
        );
    }
}

/// 出力のどこにあっても、`checks` の配列にある判定のキーを集める（電力収支はケースごとに判定を持つ）。
fn collect_keys(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            for (k, child) in map {
                match (k.as_str(), child) {
                    ("checks", Value::Array(checks)) => out.extend(
                        checks
                            .iter()
                            .filter_map(|c| c["key"].as_str().map(str::to_owned)),
                    ),
                    _ => collect_keys(child, out),
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|c| collect_keys(c, out)),
        _ => {}
    }
}

/// 温度の要求は部品の範囲を厳しくするだけで、広げない（BAT の動作範囲は 0〜45 °C）。厳しい要求（5 °C 以上）は判定の限界になり、
/// 緩い要求（−5 °C 以上）は効かず、判定文にそう書く。部品の範囲と同じ値（0 °C）は何も足さない。
#[test]
fn a_temperature_requirement_only_tightens_the_part_range() {
    let message = |degc: f64| -> String {
        let mut d = demo();
        let r = requirement(
            "R_BAT_TEMP",
            "Temperature",
            "at_least",
            json!({"value": degc, "unit": "degC"}),
            json!({"analysis": "thermal.steady", "check": "range_low:BAT:operating"}),
        );
        with(&mut d, r);
        let resolved = fune_model::resolve(&d);
        let ctx = Context::new(&d, &resolved, None).unwrap();
        let input = project_json("thermal.steady", &ctx).unwrap();
        let out = sat_analysis::catalog::run_json("thermal.steady", input).unwrap();
        let mut found = Vec::new();
        fn walk(v: &Value, found: &mut Vec<String>) {
            match v {
                Value::Object(m) => {
                    if m.get("key").and_then(Value::as_str) == Some("range_low:BAT:operating")
                        && let Some(msg) = m.get("message").and_then(Value::as_str)
                    {
                        found.push(msg.to_owned());
                    }
                    m.values().for_each(|c| walk(c, found));
                }
                Value::Array(a) => a.iter().for_each(|c| walk(c, found)),
                _ => {}
            }
        }
        walk(&out, &mut found);
        found.pop().expect("the check")
    };
    let tight = message(5.0);
    assert!(
        tight.contains("operating range lower limit 5.00 °C"),
        "{tight}"
    );
    assert!(
        tight.contains("tightened by requirement R_BAT_TEMP"),
        "{tight}"
    );
    let loose = message(-5.0);
    assert!(loose.contains("operating range lower limit 0"), "{loose}");
    assert!(loose.contains("Requirement R_BAT_TEMP"), "{loose}");
    assert!(loose.contains("has no effect"), "{loose}");
    let same = message(0.0);
    assert!(!same.contains("R_BAT_TEMP"), "{same}");
}
