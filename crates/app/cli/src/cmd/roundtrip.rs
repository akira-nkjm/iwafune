//! 往復の検査: `to_cli(op)` の文字列を引数に分け、clap で読み、Op に戻すと元の Op と一致する。
//! GUI の「CLI としてコピー」が必ずそのまま使えることを保証する。Op の種類が増えたら `covers` の match が
//! コンパイルエラーになるので、ここに見本を足す。

use super::ops::edit_of;
use crate::args::Cli;
use clap::Parser;
use fune_service::Workspace;
mod catalog;
mod samples;

use catalog::catalog_ops;
use fune_service::dto::Op;
use fune_service::edit::to_cli;
use samples::{def_ops, part_ops, record_ops, system_ops};
use std::path::Path;

/// `quote` の逆（空白区切り。`"…"` の中は `\` で `" \ $ \`` を逃がす）。
fn split(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut quoted, mut escaped, mut started) = (false, false, false);
    for c in line.chars() {
        match (quoted, escaped, c) {
            (true, true, c) => {
                cur.push(c);
                escaped = false;
            }
            (true, false, '\\') => escaped = true,
            (true, false, '"') => quoted = false,
            (true, false, c) => cur.push(c),
            (false, _, '"') => {
                quoted = true;
                started = true;
            }
            (false, _, ' ') => {
                if started || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                started = false;
            }
            (false, _, c) => cur.push(c),
        }
    }
    if started || !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn workspace() -> Workspace {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    Workspace::open(root).unwrap()
}

/// 隣り合うモード表・図の配置の Op を 1 つにまとめる（`to_cli` は 1 セル 1 行に分けるため）。
fn merge(ops: Vec<Op>) -> Vec<Op> {
    let mut out: Vec<Op> = Vec::new();
    for op in ops {
        match (out.last_mut(), op) {
            (Some(Op::SetModeStates { cells }), Op::SetModeStates { cells: more }) => {
                cells.extend(more)
            }
            (
                Some(Op::SetViewLayout {
                    view,
                    nodes,
                    remove,
                }),
                Op::SetViewLayout {
                    view: v2,
                    nodes: n2,
                    remove: r2,
                },
            ) if *view == v2 => {
                nodes.extend(n2);
                remove.extend(r2);
            }
            (_, op) => out.push(op),
        }
    }
    out
}

fn roundtrip(ws: &Workspace, op: &Op) {
    let mut back = Vec::new();
    for line in to_cli(op) {
        let argv = split(&line);
        let cli = Cli::try_parse_from(&argv).unwrap_or_else(|e| panic!("{line}\n{e}"));
        let edit = edit_of(ws, &cli.command)
            .unwrap_or_else(|e| panic!("{line}\n{e:#}"))
            .unwrap_or_else(|| panic!("{line} does not become an edit command"));
        back.extend(edit.ops);
    }
    assert_eq!(merge(back), vec![op.clone()], "{:?}", to_cli(op));
}

/// すべての Op の種類に見本があることをコンパイル時に確かめる（ワイルドカードを使わない）。
fn covers(op: &Op) -> &'static str {
    match op {
        Op::CreatePart { .. } => "CreatePart",
        Op::RemovePart { .. } => "RemovePart",
        Op::RenamePart { .. } => "RenamePart",
        Op::RenameGroup { .. } => "RenameGroup",
        Op::MovePart { .. } => "MovePart",
        Op::SetValue { .. } => "SetValue",
        Op::UnsetValue { .. } => "UnsetValue",
        Op::SetField { .. } => "SetField",
        Op::Place { .. } => "Place",
        Op::SetPort { .. } => "SetPort",
        Op::SetMountPoint { .. } => "SetMountPoint",
        Op::CreateAttribute { .. } => "CreateAttribute",
        Op::CreateKind { .. } => "CreateKind",
        Op::UpdateAttribute { .. } => "UpdateAttribute",
        Op::UpdateKind { .. } => "UpdateKind",
        Op::DeleteAttribute { .. } => "DeleteAttribute",
        Op::DeleteKind { .. } => "DeleteKind",
        Op::RenameAttribute { .. } => "RenameAttribute",
        Op::RenameKind { .. } => "RenameKind",
        Op::CreateMode { .. } => "CreateMode",
        Op::RemoveMode { .. } => "RemoveMode",
        Op::RenameMode { .. } => "RenameMode",
        Op::MoveMode { .. } => "MoveMode",
        Op::SetModeDuration { .. } => "SetModeDuration",
        Op::SetModeTiming { .. } => "SetModeTiming",
        Op::SetSequence { .. } => "SetSequence",
        Op::RemoveSequence { .. } => "RemoveSequence",
        Op::SetModeStates { .. } => "SetModeStates",
        Op::CreateNet { .. } => "CreateNet",
        Op::UpdateNet { .. } => "UpdateNet",
        Op::SetNetField { .. } => "SetNetField",
        Op::RemoveNet { .. } => "RemoveNet",
        Op::SetCase { .. } => "SetCase",
        Op::RemoveCase { .. } => "RemoveCase",
        Op::SetSource { .. } => "SetSource",
        Op::RemoveSource { .. } => "RemoveSource",
        Op::MoveSource { .. } => "MoveSource",
        Op::SetHarness { .. } => "SetHarness",
        Op::RemoveHarness { .. } => "RemoveHarness",
        Op::CreateManeuver { .. } => "CreateManeuver",
        Op::SetManeuver { .. } => "SetManeuver",
        Op::RemoveManeuver { .. } => "RemoveManeuver",
        Op::CreateGroundStation { .. } => "CreateGroundStation",
        Op::RemoveGroundStation { .. } => "RemoveGroundStation",
        Op::RenameGroundStation { .. } => "RenameGroundStation",
        Op::CreatePhase { .. } => "CreatePhase",
        Op::SetPhase { .. } => "SetPhase",
        Op::RemovePhase { .. } => "RemovePhase",
        Op::RenamePhase { .. } => "RenamePhase",
        Op::SetPhaseField { .. } => "SetPhaseField",
        Op::CopyPhaseOrbit { .. } => "CopyPhaseOrbit",
        Op::UnsetPhaseOrbit { .. } => "UnsetPhaseOrbit",
        Op::UnsetPhaseLaw { .. } => "UnsetPhaseLaw",
        Op::SetCasePhase { .. } => "SetCasePhase",
        Op::RenameNet { .. } => "RenameNet",
        Op::RenameHarness { .. } => "RenameHarness",
        Op::RenameCase { .. } => "RenameCase",
        Op::RenameSource { .. } => "RenameSource",
        Op::RenameManeuver { .. } => "RenameManeuver",
        Op::CreateCommLink { .. } => "CreateCommLink",
        Op::RemoveCommLink { .. } => "RemoveCommLink",
        Op::RenameCommLink { .. } => "RenameCommLink",
        Op::SetProjectField { .. } => "SetProjectField",
        Op::AddLocalCatalog { .. } => "AddLocalCatalog",
        Op::CreateRequirement { .. } => "CreateRequirement",
        Op::SetRequirement { .. } => "SetRequirement",
        Op::RemoveRequirement { .. } => "RemoveRequirement",
        Op::SetViewLayout { .. } => "SetViewLayout",
        Op::SetViewEdge { .. } => "SetViewEdge",
        Op::UnsetViewEdge { .. } => "UnsetViewEdge",
        Op::ResetView { .. } => "ResetView",
        Op::SetSetting { .. } => "SetSetting",
        Op::CreateCatalogItem { .. } => "CreateCatalogItem",
        Op::SetCatalogValue { .. } => "SetCatalogValue",
        Op::UnsetCatalogValue { .. } => "UnsetCatalogValue",
        Op::SetCatalogField { .. } => "SetCatalogField",
        Op::RemoveCatalogItem { .. } => "RemoveCatalogItem",
        Op::RenameCatalogItem { .. } => "RenameCatalogItem",
        Op::MoveCatalogItem { .. } => "MoveCatalogItem",
        Op::AddCatalogSource { .. } => "AddCatalogSource",
        Op::RemoveCatalogSource { .. } => "RemoveCatalogSource",
    }
}

#[test]
fn split_undoes_quote() {
    assert_eq!(
        split(r#"fune set RW_X.mass "620 g" --note "a \"b\" c""#),
        ["fune", "set", "RW_X.mass", "620 g", "--note", "a \"b\" c"]
    );
    assert_eq!(split(r#"fune x """#), ["fune", "x", ""]);
}

#[test]
fn every_op_survives_to_cli_and_back() {
    let ws = workspace();
    let all: Vec<Op> = [
        samples::view_ops(),
        samples::eclipse_cases(),
        part_ops(),
        def_ops(),
        system_ops(),
        record_ops(),
        catalog_ops(),
    ]
    .concat();
    let mut seen: Vec<&str> = all.iter().map(covers).collect();
    for op in &all {
        roundtrip(&ws, op);
    }
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), 81, "some Op kinds have no sample: {seen:?}");
}
