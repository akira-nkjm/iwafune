//! 要求の判定のキー（`verify.check`）に入る名前（部品・地上局・通信の回線・熱の節点）は、名前の変更で追従する（#367）。
//! 追従しないと、適合表で「確かめられない」になり、温度の要求の限度は温度の解析に届かなくなる。
use fune_service::dto::{ApplyRequest, NotVerifiedReason, Op};
use fune_service::requirements::compliance;
use fune_service::{Workspace, edit};
use sat_analysis::spec::Verdict;
use std::path::Path;

fn demo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// デモの写しに、判定のキーが名前を持つ要求（`X_*`）を足したワークスペース。
fn workspace(tmp: &tempfile::TempDir) -> Workspace {
    let root = tmp.path().join("demo-6u");
    copy_dir(&demo_root(), &root);
    let head = "statement = \"s\"\nquantity = \"Dimensionless\"\nsource = \"sys_judgement\"\n[constraint]\nkind = \"at_most\"\nvalue = { value = 1.0, unit = \"\" }\n";
    for (id, analysis, key) in [
        ("X_HEATER", "thermal.steady", "heater:HTR_BAT"),
        ("X_IFACE", "structure.static_load", "interface:BUS_FRAME"),
        ("X_LINK_UP", "comm.link_budget", "margin:uhf_up@TOKYO_GS"),
    ] {
        let body = format!("{head}[verify]\nanalysis = \"{analysis}\"\ncheck = \"{key}\"\n");
        std::fs::write(root.join("requirements").join(format!("{id}.toml")), body).unwrap();
    }
    Workspace::open(&root).unwrap()
}

fn rename_part(id: &str, to: &str) -> Op {
    Op::RenamePart {
        id: id.into(),
        to: to.into(),
    }
}

fn apply(ws: &Workspace, ops: Vec<Op>) -> Result<(), String> {
    edit::apply(
        ws,
        &ApplyRequest {
            ops,
            base_revision: None,
            dry_run: false,
        },
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn check_of(ws: &Workspace, id: &str) -> String {
    let design = ws.snapshot().unwrap().design;
    design.requirements[id].verify.check.clone().unwrap()
}

/// 適合表の行の判定（確かめられなければ `None`）。確かめられない理由が「キーが出ない」なら、名前の追従の漏れ。
fn verdict_of(ws: &Workspace, id: &str) -> Option<Verdict> {
    let report = compliance(ws).unwrap();
    let row = report.rows.iter().find(|r| r.id == id).unwrap();
    assert_ne!(row.reason, Some(NotVerifiedReason::KeyNotProduced), "{id}");
    row.verdict
}

/// 名前を変えたあとも、キーが追従して同じ行が同じ判定で確かめられる。
fn assert_follows(ws: &Workspace, requirement: &str, op: Op, expected_key: &str) {
    let before = verdict_of(ws, requirement);
    assert!(before.is_some(), "{requirement} must be verified before");
    apply(ws, vec![op]).unwrap();
    assert_eq!(check_of(ws, requirement), expected_key);
    assert_eq!(verdict_of(ws, requirement), before, "{requirement}");
}

#[test]
fn part_names_in_check_keys_follow_a_part_rename() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = workspace(&tmp);
    // `intrusion:<センサ>`（デモの R_STT_SUN）
    assert_follows(
        &ws,
        "R_STT_SUN",
        rename_part("STT", "STT_A"),
        "intrusion:STT_A",
    );
    // `heater:<ヒータ>`
    assert_follows(
        &ws,
        "X_HEATER",
        rename_part("HTR_BAT", "HTR_B"),
        "heater:HTR_B",
    );
    // `interface:<取付先>`
    assert_follows(
        &ws,
        "X_IFACE",
        rename_part("BUS_FRAME", "BUS_STRUCT"),
        "interface:BUS_STRUCT",
    );
}

#[test]
fn station_and_link_names_in_check_keys_follow_their_renames() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = workspace(&tmp);
    // 地上局の名前の変更は、これまでは拒まれた。キーを書き換える
    assert_follows(
        &ws,
        "R_LINK_S",
        Op::RenameGroundStation {
            id: "TOKYO_GS".into(),
            to: "TOKYO_2".into(),
        },
        "margin:s_down@TOKYO_2",
    );
    // 同じ局を名指す別のキーも
    assert_eq!(check_of(&ws, "X_LINK_UP"), "margin:uhf_up@TOKYO_2");
    assert_follows(
        &ws,
        "R_PFD",
        Op::RenameCommLink {
            id: "s_down".into(),
            to: "s_dl".into(),
        },
        "pfd:s_dl",
    );
    assert_eq!(check_of(&ws, "R_LINK_S"), "margin:s_dl@TOKYO_2");
}

/// 温度の要求は、部品の名前を変えても節点の限度として温度の解析の入力に届く（節点の名前は設定のキーで、部品の名前では変わらない）。
#[test]
fn a_temperature_requirement_still_reaches_the_thermal_input_after_a_part_rename() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = workspace(&tmp);
    let limit = |ws: &Workspace| {
        let result = fune_service::analysis::run_analysis(
            ws,
            "thermal.steady",
            &fune_service::dto::RunAnalysisRequest::default(),
        )
        .unwrap();
        let input = result.input.expect("the input is shown");
        let node = input["network"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == "BAT")
            .unwrap()
            .clone();
        node["required"].clone()
    };
    let before = limit(&ws);
    assert!(
        before.as_object().is_some_and(|o| !o.is_empty()),
        "{before}"
    );
    apply(&ws, vec![rename_part("BAT", "BAT_CELL")]).unwrap();
    assert_eq!(check_of(&ws, "R_BAT_TEMP"), "range_low:BAT:operating");
    assert_eq!(limit(&ws), before);
    assert_eq!(verdict_of(&ws, "R_BAT_TEMP"), Some(Verdict::Violation));
}

/// 削除は、キーを書き換えられないので拒む（部品・熱の節点・地上局・回線）。
#[test]
fn removing_an_entity_named_by_a_check_key_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = workspace(&tmp);
    let refused = |ops: Vec<Op>, what: &str| {
        let err = apply(&ws, ops).unwrap_err();
        assert!(err.contains(what) && err.contains("verify.check"), "{err}");
    };
    refused(
        vec![Op::RemovePart {
            id: "HTR_BAT".into(),
        }],
        "Part `HTR_BAT`",
    );
    refused(
        vec![Op::RemoveGroundStation {
            id: "TOKYO_GS".into(),
        }],
        "Ground station `TOKYO_GS`",
    );
    refused(
        vec![Op::RemoveCommLink {
            id: "s_down".into(),
        }],
        "Link `s_down`",
    );
    // 節点は設定のキー。消す書き込みを拒む（`range_low:BAT:operating` を書いた R_BAT_TEMP がある）
    refused(
        vec![Op::SetSetting {
            path: "analysis.thermal.nodes.BAT".into(),
            value: None,
        }],
        "Thermal node `BAT`",
    );
}

/// 既定の 1 部品 1 節点では、部品の ID が節点の名前（EPT はデモの `[nodes]` に載っていない）。その節点を名指す判定のキー
/// （`range_high:EPT:operating`）と、節点の名前を書いた設定（明示の結合の両端・`[heat_node]` の値）は、部品の名前の変更に追従する。
/// 節点を名指すキーがある間は、その部品の削除を拒む。
#[test]
fn per_part_node_names_follow_a_part_rename() {
    use fune_model::value::{Quantity, SettingValue};
    let tmp = tempfile::tempdir().unwrap();
    let ws = workspace(&tmp);
    let body = "statement = \"s\"\nquantity = \"Temperature\"\nsource = \"sys_judgement\"\n[constraint]\nkind = \"at_most\"\nvalue = { value = 40.0, unit = \"degC\" }\n[verify]\nanalysis = \"thermal.steady\"\ncheck = \"range_high:EPT:operating\"\n";
    std::fs::write(tmp.path().join("demo-6u/requirements/X_NODE.toml"), body).unwrap();
    let ws = Workspace::open(ws.root()).unwrap();
    let set = |path: &str, value: SettingValue| Op::SetSetting {
        path: path.into(),
        value: Some(value),
    };
    apply(
        &ws,
        vec![
            set(
                "analysis.thermal.conductive.BUS.EPT",
                SettingValue::Quantity(Quantity::new(0.3, "W/K")),
            ),
            set(
                "analysis.thermal.heat_node.HTR_CAM",
                SettingValue::Text("EPT".into()),
            ),
        ],
    )
    .unwrap();
    let refused = apply(&ws, vec![Op::RemovePart { id: "EPT".into() }]).unwrap_err();
    assert!(refused.contains("verify.check"), "{refused}");
    apply(&ws, vec![rename_part("EPT", "EPT_A")]).unwrap();
    assert_eq!(check_of(&ws, "X_NODE"), "range_high:EPT_A:operating");
    let design = ws.snapshot().unwrap().design;
    let t = design.analysis.thermal.as_ref().unwrap();
    assert!(t.conductive["BUS"].contains_key("EPT_A") && !t.conductive["BUS"].contains_key("EPT"));
    assert_eq!(t.heat_node["HTR_CAM"], "EPT_A");
    let problems: Vec<_> = fune_model::check(&design)
        .into_iter()
        .filter(|d| d.is_error() && (d.code.starts_with("ref.") || d.code.starts_with("thermal.")))
        .collect();
    assert!(problems.is_empty(), "{problems:?}");
}
