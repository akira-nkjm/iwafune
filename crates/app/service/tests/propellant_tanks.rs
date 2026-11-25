//! スラスタが推薬を取るタンク（属性 `tank`）のデモでの確かめ: 唯一のタンクを名指しても結果は変わらず、
//! 食い違い・部品が無い・役割が違う場合は検査が言い、指す先がタンクでなければ不足の入力にタンクを選ぶ候補が付く。

use fune_model::design::refs::TANK_ATTR;
use fune_model::value::{Authored, Value};
use fune_model::{Design, Id, resolve};
use fune_service::dto::{AnalysisInputSource, Fix, RefKind, RunAnalysisRequest};
use fune_service::{Workspace, analysis};
use serde_json::{Value as Json, json};
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .expect("demo")
}

fn run(ws: &Workspace, edit: Json) -> fune_service::dto::AnalysisRunResult {
    let req = RunAnalysisRequest {
        source: AnalysisInputSource::Design,
        ops: serde_json::from_value(edit).unwrap(),
        ..RunAnalysisRequest::default()
    };
    analysis::run_analysis(ws, "propulsion.propellant", &req).unwrap()
}

fn name_tank(tank: &str) -> Json {
    json!([{ "op": "set_value", "part": "EPT", "attr": TANK_ATTR,
        "value": { "type": "text", "data": tank } }])
}

/// デモの唯一のタンクを名指しても、積む量・ウェット質量・タンクの行の数値は名指さないときと同じ（ビットまで）。
#[test]
fn naming_the_only_tank_changes_no_number() {
    let ws = demo();
    let plain = run(&ws, json!([])).output.unwrap();
    let named = run(&ws, name_tank("XE_TANK"));
    assert!(named.missing.is_empty(), "{:?}", named.missing);
    let named = named.output.unwrap();
    for key in ["loaded_propellant", "wet_mass", "usable_propellant"] {
        assert_eq!(plain[key], named[key], "{key}");
    }
    let (a, b) = (&plain["tanks"][0], &named["tanks"][0]);
    assert_eq!(a["id"], "XE_TANK");
    // 組の積む量（タンクの行の元）は `loaded_propellant` と丸めの差だけ違う（これまでと同じ）
    let (loaded, total) = (
        a["loaded"].as_f64().unwrap(),
        plain["loaded_propellant"].as_f64(),
    );
    assert!((loaded - total.unwrap()).abs() < 1e-9);
    assert_eq!(a["loaded"], b["loaded"]);
    assert_eq!(a["volume_margin"], b["volume_margin"]);
    assert_eq!(a["named"], false);
    assert_eq!(b["named"], true);
}

/// 指す先がタンクでなければ不足の入力（`EPT.tank`）で、直し方の候補は役割 `propellant_tank` の部品。
#[test]
fn a_named_part_that_is_not_a_tank_is_a_missing_input_with_the_tanks_as_choices() {
    let ws = demo();
    let r = run(&ws, name_tank("BUS_FRAME"));
    let m = r
        .missing
        .iter()
        .find(|m| m.path == "EPT.tank")
        .expect("EPT.tank is missing");
    let [
        Fix::PickRef {
            what: RefKind::Part,
            choices,
            ..
        },
    ] = m.fixes.as_slice()
    else {
        panic!("{:?}", m.fixes);
    };
    assert_eq!(
        choices.iter().map(|c| c.value.as_str()).collect::<Vec<_>>(),
        ["XE_TANK"]
    );
}

fn with_tank_attrs(tank: &str, propellant: Option<&str>) -> Design {
    let mut d = demo().snapshot().unwrap().design;
    let part = d.parts.get_mut("EPT").unwrap();
    part.attr.insert(
        Id::new(TANK_ATTR),
        Authored::bare(Value::Text(tank.to_owned())),
    );
    if let Some(p) = propellant {
        part.attr.insert(
            Id::new("propellant"),
            Authored::bare(Value::Enum(p.to_owned())),
        );
    }
    d
}

fn codes(d: &Design) -> Vec<String> {
    let r = resolve(d);
    fune_model::check(d)
        .into_iter()
        .chain(fune_projection::propulsion::tank_refs::check(d, &r))
        .map(|x| x.code)
        .collect()
}

/// 検査: 部品が無ければ `ref.part`、タンクでなければ `propulsion.tank_role`、推薬の名前が違えば `propulsion.tank_propellant`。
#[test]
fn checks_report_a_missing_part_a_wrong_role_and_a_propellant_mismatch() {
    let ok = codes(&with_tank_attrs("XE_TANK", None));
    assert!(
        !ok.iter()
            .any(|c| c.starts_with("propulsion.tank") || c == "ref.part"),
        "{ok:?}"
    );
    assert!(codes(&with_tank_attrs("NOPE", None)).contains(&"ref.part".to_owned()));
    assert!(
        codes(&with_tank_attrs("BUS_FRAME", None)).contains(&"propulsion.tank_role".to_owned())
    );
    assert!(
        codes(&with_tank_attrs("XE_TANK", Some("krypton")))
            .contains(&"propulsion.tank_propellant".to_owned())
    );
}
