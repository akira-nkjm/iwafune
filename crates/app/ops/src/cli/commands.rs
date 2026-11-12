//! Op ごとのコマンド。値の文字列は `fune_model::value` の書き方（`value_to_text`・`setting_to_text` など）で作る。

use super::text::Cmd;
use crate::op::{ModeCell, Op, ViewNodePos};
use fune_model::design::{Net, NetField, NetFieldName, PartField};
use fune_model::value::{
    Provenance, Scalar, Value, list_to_text, number_to_text, number_unit_to_text, quantity_to_text,
    quaternion_to_text, setting_to_text, value_to_text, vector_to_text,
};

/// 部品の Op のコマンド。部品の Op でなければ None。
pub(super) fn part_op(op: &Op) -> Option<Vec<String>> {
    Some(match op {
        Op::CreatePart { .. } => vec![create_part(op)],
        Op::RemovePart { id } => vec![Cmd::new(&["fune", "part", "rm", id]).done()],
        Op::RenamePart { id, to } => vec![Cmd::new(&["fune", "part", "rename", id, to]).done()],
        Op::RenameGroup { id, to } => {
            vec![Cmd::new(&["fune", "part", "rename-group", id, to]).done()]
        }
        Op::MovePart { id, to } => vec![match to {
            Some(to) => Cmd::new(&["fune", "part", "mv", id, to]).done(),
            None => Cmd::new(&["fune", "part", "mv", id, "--root"]).done(),
        }],
        Op::SetValue {
            part,
            attr,
            state,
            value,
            from,
        } => {
            let base = format!("{part}.{attr}");
            value_cmds(
                &["fune", "set"],
                &base,
                state.as_deref(),
                value,
                from.as_ref(),
            )
        }
        Op::UnsetValue { part, attr, state } => {
            let path = [Some(part.as_str()), Some(attr.as_str()), state.as_deref()];
            let path: Vec<&str> = path.into_iter().flatten().collect();
            vec![Cmd::new(&["fune", "unset", &path.join(".")]).done()]
        }
        Op::SetField { part, field } => vec![set_field(part, field)],
        Op::Place {
            part,
            on,
            point,
            position,
            orientation,
        } => vec![place(
            part,
            on.as_deref(),
            point.as_deref(),
            position.as_ref(),
            orientation.as_ref(),
        )],
        Op::SetPort { part, port, net } => vec![set_port(part, port, net.as_deref())],
        _ => return None,
    })
}

fn create_part(op: &Op) -> String {
    let Op::CreatePart {
        id,
        group,
        catalog,
        kind,
        count,
        label,
        mode_state,
    } = op
    else {
        return String::new();
    };
    Cmd::new(&["fune", "part", "add", id])
        .flag("catalog", catalog.as_deref())
        .flag("kind", kind.as_deref())
        .flag("group", group.as_deref())
        .flag("count", count.map(|c| c.to_string()).as_deref())
        .flag("label", label.as_deref())
        .flag("mode-state", mode_state.as_deref())
        .done()
}

fn set_port(part: &str, port: &str, net: Option<&str>) -> String {
    let target = format!("{part}.{port}");
    match net {
        Some(net) => Cmd::new(&["fune", "net", "connect", &target, net]).done(),
        None => Cmd::new(&["fune", "net", "disconnect", &target]).done(),
    }
}

fn place(
    part: &str,
    on: Option<&str>,
    point: Option<&str>,
    position: Option<&fune_model::value::Vector3>,
    orientation: Option<&fune_model::value::Quaternion>,
) -> String {
    let cmd = Cmd::new(&["fune", "part", "place", part]);
    let cmd = match on {
        Some(on) => cmd.flag("on", Some(on)).flag("point", point),
        None => cmd.switch("root", true),
    };
    let pos = position.map(vector_to_text);
    let ori = orientation.map(quaternion_to_text);
    cmd.flag("position", pos.as_deref())
        .flag("orientation", ori.as_deref())
        .done()
}

/// 定義・モード・ネット・図・設定の Op のコマンド。
pub(super) fn other_op(op: &Op) -> Vec<String> {
    match op {
        Op::CreateAttribute { def } => vec![super::defs::attr_add(def)],
        Op::CreateKind { def } => vec![super::defs::kind_add(def)],
        Op::CreateMode {
            id,
            label,
            description,
            copy_from,
        } => vec![
            Cmd::new(&["fune", "mode", "add", id])
                .flag("label", label.as_deref())
                .flag("description", description.as_deref())
                .flag("from", copy_from.as_deref())
                .done(),
        ],
        Op::RemoveMode { id } => vec![Cmd::new(&["fune", "mode", "rm", id]).done()],
        Op::RenameMode { id, to } => vec![Cmd::new(&["fune", "mode", "rename", id, to]).done()],
        Op::SetModeDuration { mode, duration } => vec![
            Cmd::new(&["fune", "mode", "duration", mode])
                .flag(
                    "max-duration",
                    duration.as_ref().map(quantity_to_text).as_deref(),
                )
                .done(),
        ],
        Op::SetModeStates { cells } => cells.iter().map(mode_cell).collect(),
        Op::CreateNet { net } => vec![net_cmd("add", net)],
        Op::UpdateNet { net } => vec![net_cmd("replace", net)],
        Op::SetNetField { net, field } => vec![net_field_cmd(net, field)],
        Op::RemoveNet { id } => vec![Cmd::new(&["fune", "net", "rm", id]).done()],
        Op::SetViewEdge { view, key, edge } => {
            let points = edge
                .points
                .iter()
                .map(|[x, y]| format!("{x},{y}"))
                .collect::<Vec<_>>()
                .join(" ");
            let mut cmd =
                Cmd::new(&["fune", "view", "edge-set", view, key]).flag("points", Some(&points));
            for (flag, side) in [
                ("source-side", edge.source_side),
                ("target-side", edge.target_side),
            ] {
                if let Some(side) = side {
                    cmd = cmd.flag(flag, Some(side.as_str()));
                }
            }
            for (flag, offset) in [
                ("source-offset", edge.source_offset),
                ("target-offset", edge.target_offset),
            ] {
                if let Some(offset) = offset {
                    cmd = cmd.flag(flag, Some(&offset.to_string()));
                }
            }
            vec![cmd.done()]
        }
        Op::UnsetViewEdge { view, key } => {
            vec![Cmd::new(&["fune", "view", "edge-reset", view, key]).done()]
        }
        Op::ResetView { view } => vec![Cmd::new(&["fune", "view", "reset", view]).done()],
        Op::SetViewLayout {
            view,
            nodes,
            remove,
        } => view_cmds(view, nodes, remove),
        Op::SetSetting { path, value } => vec![match value {
            Some(v) => Cmd::new(&["fune", "setting", "set", path])
                .arg(&setting_to_text(v))
                .done(),
            None => Cmd::new(&["fune", "setting", "unset", path]).done(),
        }],
        _ => Vec::new(),
    }
}

fn scalar_cmd(head: &[&str], path: &str, s: &Scalar, from: Option<&Provenance>) -> String {
    let min = s.min.map(|m| number_unit_to_text(m, &s.unit));
    let max = s.max.map(|m| number_unit_to_text(m, &s.unit));
    Cmd::new(head)
        .arg(path)
        .arg(&number_unit_to_text(s.value, &s.unit))
        .flag("min", min.as_deref())
        .flag("max", max.as_deref())
        .from(from)
        .done()
}

/// 値を書くコマンド。`head` はコマンドの頭（`["fune", "set"]`）、`base` は状態を除いたパス（`RW_X.power`）。
/// 状態ごとの値を丸ごと書くときは 1 状態 1 行。
pub(super) fn value_cmds(
    head: &[&str],
    base: &str,
    state: Option<&str>,
    value: &Value,
    from: Option<&Provenance>,
) -> Vec<String> {
    let path = match state {
        Some(s) => format!("{base}.{s}"),
        None => base.to_string(),
    };
    let text = match value {
        Value::Scalar(s) => return vec![scalar_cmd(head, &path, s, from)],
        Value::ByState(cells) => {
            return cells
                .iter()
                .map(|(s, c)| {
                    scalar_cmd(
                        head,
                        &format!("{base}.{s}"),
                        &c.value,
                        c.from.as_ref().or(from),
                    )
                })
                .collect();
        }
        other => match value_to_text(other) {
            Some(text) => text,
            // 1 つの文字列にならない値（テンソル）は値の JSON で渡す
            None => {
                let json = serde_json::to_string(value).unwrap_or_default();
                return vec![Cmd::new(head).arg(&path).flag("json", Some(&json)).done()];
            }
        },
    };
    vec![Cmd::new(head).arg(&path).arg(&text).from(from).done()]
}

fn set_field(part: &str, field: &PartField) -> String {
    let path = format!("{part}.{}", field.name());
    let text: Option<String> = match field {
        PartField::Label(v)
        | PartField::Description(v)
        | PartField::Catalog(v)
        | PartField::Kind(v) => v.clone(),
        PartField::Count(n) => Some(n.to_string()),
        PartField::Redundancy(r) => Some(r.as_str().to_string()),
        PartField::Maturity(m) => m.map(|m| m.as_str().to_string()),
        PartField::Candidates(c) => (!c.is_empty()).then(|| list_to_text(c)),
    };
    match text {
        Some(t) => Cmd::new(&["fune", "set", &path]).arg(&t).done(),
        None => Cmd::new(&["fune", "unset", &path]).done(),
    }
}

fn mode_cell(cell: &ModeCell) -> String {
    match &cell.state {
        Some(state) => {
            let duty = (!cell.duty.is_empty()).then(|| {
                list_to_text(
                    cell.duty
                        .iter()
                        .map(|(s, d)| format!("{s}={}", number_to_text(*d))),
                )
            });
            Cmd::new(&["fune", "mode", "set", &cell.mode, &cell.part, state])
                .flag("duty", duty.as_deref())
                .flag("active", cell.active.map(|n| n.to_string()).as_deref())
                .flag("spare-state", cell.spare_state.as_deref())
                .flag("eclipse", cell.eclipse.then_some("true"))
                .done()
        }
        None => Cmd::new(&["fune", "mode", "unset", &cell.mode, &cell.part])
            .flag("eclipse", cell.eclipse.then_some("true"))
            .done(),
    }
}

/// ネットのフィールドの値の文字列（`--<フィールド>` の値。None は消す）。1 フィールドの旗と丸ごとの旗で共通。
fn net_field_text(field: &NetField) -> Option<String> {
    match field {
        NetField::Kind(k) => Some(k.as_str().to_owned()),
        NetField::Label(v) | NetField::Protocol(v) => v.clone(),
        NetField::Source(p) | NetField::Master(p) => p.as_ref().map(ToString::to_string),
        NetField::Rail(q) | NetField::Limit(q) | NetField::Bitrate(q) => {
            q.as_ref().map(quantity_to_text)
        }
    }
}

/// `fune net set <ID> --<フィールド> <値>` / `fune net unset <ID> <フィールド>`（1 フィールド 1 行）。
fn net_field_cmd(net: &str, field: &NetField) -> String {
    match net_field_text(field) {
        Some(t) => Cmd::new(&["fune", "net", "set", net])
            .flag(field.name(), Some(&t))
            .done(),
        None => Cmd::new(&["fune", "net", "unset", net, field.name()]).done(),
    }
}

/// `fune net add|replace <ID> --kind … --<フィールド> …`（旗の順は `NetFieldName` の順）。
fn net_cmd(verb: &str, net: &Net) -> String {
    NetFieldName::ALL
        .iter()
        .fold(
            Cmd::new(&["fune", "net", verb, net.id.as_str()]),
            |cmd, &key| {
                let text = net_field_text(&NetField::of(net, key));
                cmd.flag(key.as_str(), text.as_deref())
            },
        )
        .done()
}

fn view_cmds(view: &str, nodes: &[ViewNodePos], remove: &[String]) -> Vec<String> {
    let set = nodes.iter().map(|n| {
        let size = n.w.zip(n.h).map(|(w, h)| format!("{w},{h}"));
        Cmd::new(&["fune", "view", "set", view, &n.id])
            .flag("pos", Some(&format!("{},{}", n.x, n.y)))
            .flag("size", size.as_deref())
            .done()
    });
    let rm = remove
        .iter()
        .map(|id| Cmd::new(&["fune", "view", "rm", view, id]).done());
    set.chain(rm).collect()
}
