//! 編集のコマンドの引数 → Op の列（実行はしない）。実行は `edit::run_ops`、往復の検査は `roundtrip` のテストが使う。

use super::{defs, nets, nouns, records};
use crate::args::{Command, EditOpts};
use fune_service::Workspace;
use fune_service::dto::Op;
use fune_service::edit::{self, SetText, UnsetText};

/// 編集のコマンドから作った Op の列とオプション。
pub struct Edit<'a> {
    pub ops: Vec<Op>,
    pub opts: &'a EditOpts,
}

fn one<'a>(built: Option<(Op, &'a EditOpts)>) -> Option<Edit<'a>> {
    built.map(|(op, opts)| Edit {
        ops: vec![op],
        opts,
    })
}

/// 編集のコマンドなら Op の列を作る（読み取りのコマンドなら None）。
pub fn edit_of<'a>(ws: &Workspace, command: &'a Command) -> anyhow::Result<Option<Edit<'a>>> {
    Ok(match command {
        Command::Part(crate::args::PartCmd::Place(p)) if p.body_pos.is_some() => Some(Edit {
            ops: vec![body_place_op(ws, p)?],
            opts: &p.edit,
        }),
        Command::Part(c) => one(nouns::part(c)?),
        Command::Mode(c) => one(nouns::mode(c)?),
        Command::Net(c) => nets::net(c)?.map(|(ops, opts)| Edit { ops, opts }),
        Command::Attr(c) => one(defs::attr(c)?),
        Command::Kind(c) => one(defs::kind(c)?),
        Command::Setting(c) => one(nouns::setting(ws, c)?),
        Command::View(c) => one(nouns::view(c)?),
        Command::Case(crate::args::CaseCmd::FromEclipse {
            id,
            at,
            mode,
            edit: opts,
        }) => Some(Edit {
            ops: vec![edit::case_from_eclipse_op(
                ws,
                &fune_service::dto::CaseFromEclipseRequest {
                    id: id.clone(),
                    at: serde_json::from_value(serde_json::Value::String(at.clone()))?,
                    mode: mode.clone(),
                },
            )?],
            opts,
        }),
        Command::Case(c) => one(records::case(c)?),
        Command::Source(c) => one(records::source(c)?),
        Command::Harness(c) => one(records::harness(c)?),
        Command::Maneuver(c) => one(records::maneuver(c)?),
        Command::Catalog(c) => one(records::catalog(ws, c)?),
        Command::Set(a) => {
            let text = SetText {
                item: None,
                path: a.path.clone(),
                value: a.value.clone(),
                source: a.source.clone(),
                at: a.at.clone(),
                note: a.note.clone(),
                min: a.min.clone(),
                max: a.max.clone(),
                ops: Vec::new(),
            };
            Some(Edit {
                ops: vec![edit::set_op(ws, &text)?],
                opts: &a.edit,
            })
        }
        Command::Unset { path, edit: opts } => Some(Edit {
            ops: vec![edit::unset_op(&UnsetText {
                item: None,
                path: path.clone(),
            })?],
            opts,
        }),
        Command::Apply { file, edit: opts } => Some(Edit {
            ops: ops_from_file(file)?,
            opts,
        }),
        _ => None,
    })
}

/// `fune apply ops.json`（GUI と同じ Op の配列の JSON。`-` で標準入力）。
pub fn ops_from_file(file: &std::path::Path) -> anyhow::Result<Vec<Op>> {
    let text = if file.as_os_str() == "-" {
        std::io::read_to_string(std::io::stdin())?
    } else {
        std::fs::read_to_string(file)?
    };
    serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("{}: Op の配列として読めません: {e}", file.display()))
}

/// `fune part place <ID> --body-pos "x y z mm" [--body-rot "w x y z"]` → 機体座標から直した `Place`。
fn body_place_op(ws: &Workspace, p: &crate::args::PlaceArgs) -> anyhow::Result<Op> {
    let position = edit::vector_from_text(p.body_pos.as_deref().unwrap_or_default())?;
    let orientation = p
        .body_rot
        .as_deref()
        .map(edit::quaternion_from_text)
        .transpose()?;
    Ok(edit::place_body_op(
        ws,
        &p.id,
        &position,
        orientation.as_ref(),
    )?)
}
