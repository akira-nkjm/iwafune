//! 不足入力の直し方の候補（`fune_service::fixes`）。デモの設計から入力を消して、対象の種類ごとに候補を確かめる。

use fune_model::path::ParamPath;
use fune_model::role::Role;
use fune_model::{Design, Target, resolve};
use fune_service::dto::{AnalysisInputSource, AnalysisRunResult, Fix, ListKind, MissingInput};
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
        Fix::PickPart {
            path,
            roles,
            parts,
            command,
            ..
        },
    ] = fix.as_slice()
    else {
        panic!("{fix:?}");
    };
    assert_eq!(path, "maneuvers.RAISE.thruster");
    assert!(roles.contains(&Role::PropulsionThruster));
    assert_eq!(parts, &["EPT"]);
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
        Fix::PickPart {
            roles,
            parts,
            command,
            ..
        },
    ] = fix.as_slice()
    else {
        panic!("{fix:?}");
    };
    assert_eq!(roles, &[Role::Transceiver]);
    assert!(parts.iter().any(|p| p == "UHF_TRX") && !parts.iter().any(|p| p == "OBC"));
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
