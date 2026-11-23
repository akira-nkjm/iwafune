//! 不足入力の直し方の候補（`fune_service::fixes`）。デモの設計から入力を消して、対象の種類ごとに候補を確かめる。

use fune_model::path::ParamPath;
use fune_model::role::Role;
use fune_model::{Design, Target, resolve};
use fune_service::dto::{
    AnalysisInputSource, AnalysisRunResult, Choice, Fix, ListKind, MissingInput, RefKind,
};
use fune_service::{Workspace, analysis, fixes};
use serde_json::{Value, json};
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .expect("demo")
}

fn run(ws: &Workspace, id: &str, edit: Value) -> AnalysisRunResult {
    let req = fune_service::dto::RunAnalysisRequest {
        case: None,
        source: AnalysisInputSource::Design,
        ops: serde_json::from_value(edit).unwrap(),
    };
    analysis::run_analysis(ws, id, &req).unwrap()
}

/// 選べる値の綴り。
fn values(choices: &[Choice]) -> Vec<&str> {
    choices.iter().map(|c| c.value.as_str()).collect()
}

/// 対象だけを与えた不足入力の候補（設計は `design`）。
fn fixes_of(design: &Design, target: Target) -> Vec<Fix> {
    let resolved = resolve(design);
    let m = MissingInput {
        part: target.part().map(str::to_owned),
        path: target.to_string(),
        target: Some(target),
        reason: String::new(),
        fixes: Vec::new(),
    };
    fixes::fixes_for(design, &resolved, &m)
}

/// 部品の属性の値がカタログ品にも無い: 部品の上書きと、品の側の書き先の両方を出す。解析の結果の不足にも付く。
#[test]
fn attribute_value_offers_part_and_catalog_item() {
    let ws = demo();
    let r = run(
        &ws,
        "aocs.rw_unloading",
        json!([{ "op": "unset_catalog_value", "item": "local/rw_10mNms", "attr": "max_momentum", "state": null }]),
    );
    let m = r
        .missing
        .iter()
        .find(|m| m.path == "RW_X.max_momentum")
        .expect("the missing input of RW_X");
    let [
        Fix::SetValue {
            path,
            dimension,
            command,
            catalog: Some(catalog),
            target,
            ..
        },
    ] = m.fixes.as_slice()
    else {
        panic!("{:?}", m.fixes);
    };
    assert_eq!(path, "RW_X.max_momentum");
    assert!(matches!(target, ParamPath::Attr { part, .. } if part == "RW_X"));
    assert_eq!(dimension.as_deref(), Some("AngularMomentum"));
    assert_eq!(command, r#"fune set RW_X.max_momentum "<value>""#);
    assert_eq!(catalog.item, "local/rw_10mNms");
    assert_eq!(catalog.path, "max_momentum");
    assert_eq!(
        catalog.command,
        r#"fune catalog set local/rw_10mNms max_momentum "<value>""#
    );
}

/// 部品がカタログ品を選んでいなければ、品の側の書き先は無い。部品のフィールドは次元なし。
#[test]
fn part_without_catalog_item_has_no_catalog_alternative() {
    let ws = demo();
    let design = ws.snapshot().unwrap().design;
    let fix = fixes_of(&design, Target::attr("EPT", "power"));
    assert!(matches!(
        fix.as_slice(),
        [Fix::SetValue { catalog: None, .. }]
    ));
    let fix = fixes_of(&design, Target::field("EPT", "count"));
    assert!(matches!(
        fix.as_slice(),
        [Fix::SetValue {
            dimension: None,
            catalog: None,
            ..
        }]
    ));
}

/// 設定の値: 次元は設定の次元の表から。
#[test]
fn setting_value_carries_its_dimension() {
    let ws = demo();
    let design = ws.snapshot().unwrap().design;
    let path = fune_model::path::SettingPath::conditions(&["orbit", "altitude"]);
    let fix = fixes_of(&design, path.into());
    let [
        Fix::SetValue {
            dimension, command, ..
        },
    ] = fix.as_slice()
    else {
        panic!("{fix:?}");
    };
    assert_eq!(dimension.as_deref(), Some("Length"));
    assert_eq!(command, r#"fune set conditions.orbit.altitude "<value>""#);
}

/// 役割の部品が 1 つも無い: 部品を足す（その役割の種別とカタログ品）か、既にある部品の種別を替える。
#[test]
fn missing_role_offers_add_and_assign() {
    let ws = demo();
    let mut design = ws.snapshot().unwrap().design;
    design.parts.remove("BAT");
    let fix = fixes_of(&design, Target::any_of_roles(&[Role::Battery], None));
    let [
        Fix::AddPart {
            kinds,
            catalog_items,
            command,
            ..
        },
        Fix::AssignPart {
            parts,
            command: assign,
            ..
        },
    ] = fix.as_slice()
    else {
        panic!("{fix:?}");
    };
    assert_eq!(kinds, &["battery"]);
    assert!(catalog_items.iter().any(|c| c == "local/bat_2s2p"));
    assert_eq!(command, "fune part add <ID> --kind battery");
    assert!(parts.iter().any(|p| p == "OBC") && !parts.iter().any(|p| p == "BAT"));
    assert_eq!(assign, "fune set <part>.kind battery");
}

/// 役割の部品が有るなら、役割の不足には足さない（部品ごとの不足として出る）。有って値だけ無いなら、値を入れる部品を選ぶ。
#[test]
fn existing_role_does_not_offer_adding_a_part() {
    let ws = demo();
    let design = ws.snapshot().unwrap().design;
    let target = Target::any_of_roles(&[Role::Battery], None);
    assert!(fixes_of(&design, target).is_empty());
    let target = Target::any_of_roles(&[Role::Obc], Some("storage_capacity"));
    let fix = fixes_of(&design, target);
    let [
        Fix::SetValueOnPart {
            attr,
            parts,
            command,
            ..
        },
    ] = fix.as_slice()
    else {
        panic!("{fix:?}");
    };
    assert_eq!(attr, "storage_capacity");
    assert!(parts.len() <= 1 && parts.iter().all(|p| p == "OBC"));
    assert_eq!(command, r#"fune set <part>.storage_capacity "<value>""#);
}

/// マヌーバのスラスタ: 選べる部品（推進系のスラスタ）と、マヌーバを丸ごと書くコマンド。
#[test]
fn maneuver_thruster_picks_among_thrusters() {
    let ws = demo();
    let design = ws.snapshot().unwrap().design;
    let fix = fixes_of(&design, Target::maneuver("RAISE", Some("thruster")));
    let [
        Fix::PickRef {
            path,
            what: RefKind::Part,
            roles,
            choices,
            command,
            ..
        },
    ] = fix.as_slice()
    else {
        panic!("{fix:?}");
    };
    assert_eq!(path, "maneuvers.RAISE.thruster");
    assert!(roles.contains(&Role::PropulsionThruster));
    assert_eq!(values(choices), ["EPT"]);
    assert!(
        command.starts_with("fune maneuver set RAISE ")
            && command.contains(r#"--thruster "<part>""#)
    );
}

/// 通信の回線の送受信機・アンテナ: 役割で絞った部品から選ぶ。
#[test]
fn comm_link_parts_are_picked_by_role() {
    let ws = demo();
    let design = ws.snapshot().unwrap().design;
    let path = fune_model::path::SettingPath::analysis(&["comm", "link", "uhf_down", "part"]);
    let fix = fixes_of(&design, path.into());
    let [
        Fix::PickRef {
            roles,
            choices,
            command,
            ..
        },
    ] = fix.as_slice()
    else {
        panic!("{fix:?}");
    };
    assert_eq!(roles, &[Role::Transceiver]);
    let parts = values(choices);
    assert!(parts.contains(&"UHF_TRX") && !parts.contains(&"OBC"));
    assert_eq!(command, "fune set analysis.comm.link.uhf_down.part <part>");
}

/// 一覧の空: 足す先の一覧とコマンド。
#[test]
fn empty_lists_offer_adding_an_item() {
    let ws = demo();
    let design = ws.snapshot().unwrap().design;
    let list = |target: Target| match fixes_of(&design, target).as_slice() {
        [Fix::AddListItem { list, command }] => (*list, command.clone()),
        other => panic!("{other:?}"),
    };
    assert_eq!(
        list(Target::Modes),
        (ListKind::Mode, "fune mode add <ID>".into())
    );
    assert_eq!(list(Target::Maneuvers).0, ListKind::Maneuver);
    let ground = fune_model::path::SettingPath::conditions(&["ground"]);
    assert_eq!(list(ground.into()).0, ListKind::GroundStation);
    let link = fune_model::path::SettingPath::analysis(&["comm", "link"]);
    assert_eq!(list(link.into()).0, ListKind::CommLink);
    assert_eq!(
        list(Target::case("HOT", None)),
        (ListKind::Case, "fune case set HOT".into())
    );
}

/// 直しようのない対象（要求・図など）には候補を付けない。
#[test]
fn other_targets_have_no_fix() {
    let ws = demo();
    let design = ws.snapshot().unwrap().design;
    assert!(fixes_of(&design, Target::criterion("budget/mass")).is_empty());
    assert!(fixes_of(&design, Target::Part { id: "OBC".into() }).is_empty());
}

/// カタログ品を指す設定: 打ち上げ機の欄は打ち上げ機の種別の品だけ、ディスペンサーの欄はディスペンサーの種別の品だけを選べる。
/// 欄が空でも、打ち上げ条件の全体が無くても、同じ候補（打ち上げ機の欄）になる。
#[test]
fn launch_settings_pick_catalog_items_of_their_kind() {
    let ws = demo();
    let mut design = ws.snapshot().unwrap().design;
    if let Some(launch) = design.conditions.launch.as_mut() {
        launch.launcher = None;
    }
    let pick = |design: &Design, keys: &[&str]| match fixes_of(
        design,
        fune_model::path::SettingPath::conditions(keys).into(),
    )
    .as_slice()
    {
        [
            Fix::PickRef {
                path,
                what: RefKind::CatalogItem,
                kinds,
                choices,
                command,
                add_command,
                ..
            },
        ] => (
            path.clone(),
            kinds.clone(),
            choices.clone(),
            command.clone(),
            add_command.clone(),
        ),
        other => panic!("{other:?}"),
    };
    let (path, kinds, choices, command, add) = pick(&design, &["launch", "launcher"]);
    assert_eq!(path, "conditions.launch.launcher");
    assert_eq!(kinds, ["launcher"]);
    assert_eq!(values(&choices), ["local/lv_alpha"]);
    assert_eq!(command, "fune set conditions.launch.launcher <item>");
    assert_eq!(
        add.as_deref(),
        Some("fune catalog add local/<ID> --kind launcher")
    );
    let (_, kinds, choices, ..) = pick(&design, &["launch", "dispenser"]);
    assert_eq!(kinds, ["dispenser"]);
    assert_eq!(values(&choices), ["local/disp_6u_rs"]);
    let (path, kinds, ..) = pick(&design, &["launch"]);
    assert_eq!(
        (path.as_str(), kinds.as_slice()),
        (
            "conditions.launch.launcher",
            ["launcher".to_owned()].as_slice()
        )
    );
}

/// 選べる品が 1 つも無いときは、候補は空で、足すコマンドが残る。
#[test]
fn launch_setting_without_catalog_items_says_how_to_add_one() {
    let ws = demo();
    let mut design = ws.snapshot().unwrap().design;
    design.catalog.clear();
    let path = fune_model::path::SettingPath::conditions(&["launch", "launcher"]);
    let fix = fixes_of(&design, path.into());
    let [
        Fix::PickRef {
            choices,
            add_command,
            ..
        },
    ] = fix.as_slice()
    else {
        panic!("{fix:?}");
    };
    assert!(choices.is_empty());
    assert!(
        add_command
            .as_deref()
            .is_some_and(|c| c.contains("--kind launcher"))
    );
}

/// 通信の `data_downlink`: 下りの回線の名前だけを選べる。回線の設定の全体が無ければ、回線を足す候補。
#[test]
fn data_downlink_picks_a_down_link() {
    let ws = demo();
    let design = ws.snapshot().unwrap().design;
    let path = fune_model::path::SettingPath::analysis(&["comm", "data_downlink"]);
    let fix = fixes_of(&design, path.into());
    let [
        Fix::PickRef {
            what: RefKind::CommLink,
            choices,
            command,
            ..
        },
    ] = fix.as_slice()
    else {
        panic!("{fix:?}");
    };
    let downs: Vec<&str> = design
        .analysis
        .comm
        .iter()
        .flat_map(|c| c.links.iter())
        .filter(|(_, l)| l.direction == fune_model::design::LinkDirection::Down)
        .map(|(n, _)| n.as_str())
        .collect();
    assert!(!downs.is_empty());
    assert_eq!(values(choices), downs);
    assert_eq!(command, "fune set analysis.comm.data_downlink <link>");
    let whole = fune_model::path::SettingPath::analysis(&["comm"]);
    assert!(matches!(
        fixes_of(&design, whole.into()).as_slice(),
        [Fix::AddListItem {
            list: ListKind::CommLink,
            ..
        }]
    ));
}

/// 量の入力には、次元から作った例（`sat_units`）が付く。次元の無い値には付かない。
#[test]
fn quantity_fixes_carry_an_input_example() {
    let ws = demo();
    let design = ws.snapshot().unwrap().design;
    let fix = fixes_of(&design, Target::attr("EPT", "mass"));
    let [
        Fix::SetValue {
            example, choices, ..
        },
    ] = fix.as_slice()
    else {
        panic!("{fix:?}");
    };
    assert_eq!(example.as_deref(), Some("1 kg"));
    assert!(choices.is_empty());
    let path = fune_model::path::SettingPath::conditions(&["orbit", "altitude"]);
    let fix = fixes_of(&design, path.into());
    assert!(
        matches!(fix.as_slice(), [Fix::SetValue { example: Some(e), .. }] if e.starts_with("1 "))
    );
    let fix = fixes_of(&design, Target::field("EPT", "count"));
    assert!(matches!(
        fix.as_slice(),
        [Fix::SetValue { example: None, .. }]
    ));
}

/// 閉じた語彙（列挙・真偽）の属性には、選べる値が付く（属性定義の `symbols`・型から。表を持たない）。
#[test]
fn enum_and_bool_attributes_offer_their_choices() {
    use fune_model::design::AttributeDef;
    let ws = demo();
    let mut design = ws.snapshot().unwrap().design;
    let def = |ty: &str, symbols: &[&str]| -> AttributeDef {
        serde_json::from_value(json!({ "label": "x", "type": ty, "symbols": symbols })).unwrap()
    };
    design
        .attributes
        .insert("polarization".into(), def("enum", &["rhcp", "lhcp"]));
    design
        .attributes
        .insert("deployable".into(), def("bool", &[]));
    let choices = |attr: &str| match fixes_of(&design, Target::attr("EPT", attr)).as_slice() {
        [Fix::SetValue { choices, .. }] => choices.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(values(&choices("polarization")), ["rhcp", "lhcp"]);
    assert_eq!(values(&choices("deployable")), ["true", "false"]);
}

/// 軌道の `maneuver` が足りないときは、日付つきの遷移（`[transfer]`）を持つマヌーバだけを選ぶ候補（`PickRef`）。
/// `source` は書かなくてよい欄で、対象外の理由（`source` を対象にした不足入力）には直す候補を付けない。
#[test]
fn the_orbit_maneuver_picks_among_dated_transfers_and_the_source_has_no_fix() {
    let mut design = demo().snapshot().expect("snapshot").design.clone();
    let mut dated = design.maneuvers["RAISE"].clone();
    dated.id = "MV_MARS".into();
    dated.transfer = Some(fune_model::design::ManeuverTransfer {
        departure: Some("2028-12-10T00:00:00Z".to_owned()),
        arrival: Some("2029-09-25T00:00:00Z".to_owned()),
    });
    design.maneuvers.insert(dated.id.clone(), dated);
    let target =
        |key: &str| Target::from(fune_model::path::SettingPath::conditions(&["orbit", key]));
    let fixes = fixes_of(&design, target("maneuver"));
    let [
        Fix::PickRef {
            what,
            choices,
            command,
            add_command,
            ..
        },
    ] = fixes.as_slice()
    else {
        panic!("expected one PickRef");
    };
    assert_eq!(*what, RefKind::Maneuver);
    // 日付を持たない RAISE・DEORBIT は選べない
    assert_eq!(values(choices), ["MV_MARS"]);
    assert_eq!(command, "fune set conditions.orbit.maneuver <maneuver>");
    assert!(
        add_command
            .as_deref()
            .is_some_and(|c| c.contains("--depart"))
    );
    assert!(fixes_of(&design, target("source")).is_empty());
}

/// 設定の表は、選んで入れる欄（`source`・`maneuver`）の選択肢を API が返す（`maneuver` は日付つきの遷移を持つものだけ）。
/// フェーズの上書きの表（`PhaseView.orbit`）は CLI と HTTP のパリティテスト（`parity_orbit_source.rs`）で見る。
#[test]
fn settings_view_carries_the_choices_of_the_orbit_keys() {
    let ws = demo();
    let design = fune_service::query::design_view(&ws).expect("design view");
    let setting = |key: &str| {
        design
            .settings
            .iter()
            .find(|s| s.key == format!("conditions.orbit.{key}"))
            .unwrap_or_else(|| panic!("no setting {key}"))
    };
    assert_eq!(values(&setting("source").choices), ["elements", "transfer"]);
    assert!(
        setting("maneuver").choices.is_empty(),
        "the demo has no dated transfer"
    );
    assert!(setting("altitude").choices.is_empty());
}
