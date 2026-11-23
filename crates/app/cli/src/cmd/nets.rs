//! `fune net add|set|unset|replace|rm|connect|disconnect` の引数 → Op。綴りは `fune_ops::to_cli` と対になる。
//! `set` は指定した旗ごとに `SetNetField` を 1 つ（`unset` とともに、読み方は GUI と同じ `fune_service::edit::net_set_ops`・
//! `net_unset_op`）、`replace` は丸ごとの `UpdateNet`。

use crate::args::{EditOpts, NetArgs, NetCmd, NetSetArgs};
use fune_service::dto::{Net, NetSetText, NetUnsetText, Op, PortRef};
use fune_service::edit;

/// 引数から作った Op の列（読み取りのコマンドなら None）。
pub type BuiltMany<'a> = anyhow::Result<Option<(Vec<Op>, &'a EditOpts)>>;

/// `fune net …` の編集。
pub fn net(c: &NetCmd) -> BuiltMany<'_> {
    Ok(Some(match c {
        NetCmd::Add(a) => (vec![Op::CreateNet { net: net_of(a)? }], &a.edit),
        NetCmd::Replace(a) => (vec![Op::UpdateNet { net: net_of(a)? }], &a.edit),
        NetCmd::Set(a) => (edit::net_set_ops(&set_text(a))?, &a.edit),
        NetCmd::Unset { id, field, edit } => (
            vec![edit::net_unset_op(&NetUnsetText {
                net: id.clone(),
                field: field.clone(),
            })?],
            edit,
        ),
        NetCmd::Rm { id, edit } => (vec![Op::RemoveNet { id: id.clone() }], edit),
        NetCmd::Rename { id, to, edit } => (
            vec![Op::RenameNet {
                id: id.clone(),
                to: to.clone(),
            }],
            edit,
        ),
        NetCmd::Connect { port, net, edit } => (vec![set_port(port, Some(net.clone()))?], edit),
        NetCmd::Disconnect { port, edit } => (vec![set_port(port, None)?], edit),
        NetCmd::Ls => return Ok(None),
    }))
}

fn set_port(port: &str, net: Option<String>) -> anyhow::Result<Op> {
    let r: PortRef = port.parse()?;
    Ok(Op::SetPort {
        part: r.part,
        port: r.port,
        net,
    })
}

/// 打ったポートの参照（`PCDU.OUT3`）。読み方は `PortRef` の `FromStr`（ファイルの読み込みと同じ）。
fn port_arg(text: &Option<String>) -> anyhow::Result<Option<PortRef>> {
    Ok(text.as_deref().map(str::parse).transpose()?)
}

fn quantity(t: &Option<String>) -> anyhow::Result<Option<fune_service::dto::Quantity>> {
    Ok(t.as_deref().map(edit::quantity_from_text).transpose()?)
}

fn net_of(a: &NetArgs) -> anyhow::Result<Net> {
    let mut net = Net::new(a.id.as_str(), edit::net_kind(&a.kind)?);
    net.label = a.label.clone();
    net.source = port_arg(&a.source)?;
    net.rail = quantity(&a.rail)?;
    net.limit = quantity(&a.limit)?;
    net.protocol = a.protocol.clone();
    net.bitrate = quantity(&a.bitrate)?;
    net.master = port_arg(&a.master)?;
    Ok(net)
}

/// `fune net set` の旗 → 打った引数（GUI のネットの欄と同じ型）。
fn set_text(a: &NetSetArgs) -> NetSetText {
    NetSetText {
        net: a.id.clone(),
        kind: a.kind.clone(),
        label: a.label.clone(),
        source: a.source.clone(),
        rail: a.rail.clone(),
        limit: a.limit.clone(),
        protocol: a.protocol.clone(),
        bitrate: a.bitrate.clone(),
        master: a.master.clone(),
    }
}
