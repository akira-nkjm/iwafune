//! `fune net add|set|unset|replace|rm|connect|disconnect` の引数 → Op。綴りは `fune_ops::to_cli` と対になる。
//! `set` は指定した旗ごとに `SetNetField` を 1 つ、`replace` は丸ごとの `UpdateNet`。

use crate::args::{EditOpts, NetArgs, NetCmd, NetSetArgs};
use fune_service::dto::{Net, NetField, NetKind, Op, PortRef};
use fune_service::edit;

/// 引数から作った Op の列（読み取りのコマンドなら None）。
pub type BuiltMany<'a> = anyhow::Result<Option<(Vec<Op>, &'a EditOpts)>>;

/// `fune net …` の編集。
pub fn net(c: &NetCmd) -> BuiltMany<'_> {
    Ok(Some(match c {
        NetCmd::Add(a) => (vec![Op::CreateNet { net: net_of(a)? }], &a.edit),
        NetCmd::Replace(a) => (vec![Op::UpdateNet { net: net_of(a)? }], &a.edit),
        NetCmd::Set(a) => (set_fields(a)?, &a.edit),
        NetCmd::Unset { id, field, edit } => (
            vec![Op::SetNetField {
                net: id.clone(),
                field: cleared(field)?,
            }],
            edit,
        ),
        NetCmd::Rm { id, edit } => (vec![Op::RemoveNet { id: id.clone() }], edit),
        NetCmd::Connect { port, net, edit } => (vec![set_port(port, Some(net.clone()))?], edit),
        NetCmd::Disconnect { port, edit } => (vec![set_port(port, None)?], edit),
        NetCmd::Ls => return Ok(None),
    }))
}

fn set_port(port: &str, net: Option<String>) -> anyhow::Result<Op> {
    let r = PortRef::parse(port)
        .ok_or_else(|| anyhow::anyhow!("ポートは 部品.ポート の形です（例: RW_X.PWR）: {port}"))?;
    Ok(Op::SetPort {
        part: r.part,
        port: r.port,
        net,
    })
}

fn kind_of(text: &str) -> anyhow::Result<NetKind> {
    text.parse()
        .map_err(|_| anyhow::anyhow!("--kind は power / data / rf です: {text}"))
}

fn quantity(t: &Option<String>) -> anyhow::Result<Option<fune_service::dto::Quantity>> {
    Ok(t.as_deref().map(edit::quantity_from_text).transpose()?)
}

fn net_of(a: &NetArgs) -> anyhow::Result<Net> {
    let mut net = Net::new(a.id.as_str(), kind_of(&a.kind)?);
    net.label = a.label.clone();
    net.source = a.source.clone();
    net.rail = quantity(&a.rail)?;
    net.limit = quantity(&a.limit)?;
    net.protocol = a.protocol.clone();
    net.bitrate = quantity(&a.bitrate)?;
    net.master = a.master.clone();
    Ok(net)
}

/// 指定した旗ごとに `SetNetField`（旗の順はフィールドの宣言順）。
fn set_fields(a: &NetSetArgs) -> anyhow::Result<Vec<Op>> {
    let mut fields = Vec::new();
    if let Some(k) = &a.kind {
        fields.push(NetField::Kind(kind_of(k)?));
    }
    let text = |v: &Option<String>| v.clone().map(Some);
    fields.extend(text(&a.label).map(NetField::Label));
    fields.extend(text(&a.source).map(NetField::Source));
    fields.extend(quantity(&a.rail)?.map(|q| NetField::Rail(Some(q))));
    fields.extend(quantity(&a.limit)?.map(|q| NetField::Limit(Some(q))));
    fields.extend(text(&a.protocol).map(NetField::Protocol));
    fields.extend(quantity(&a.bitrate)?.map(|q| NetField::Bitrate(Some(q))));
    fields.extend(text(&a.master).map(NetField::Master));
    if fields.is_empty() {
        anyhow::bail!("書くフィールドの旗がありません（--rail など）");
    }
    Ok(fields
        .into_iter()
        .map(|field| Op::SetNetField {
            net: a.id.clone(),
            field,
        })
        .collect())
}

/// `fune net unset <ID> <field>` → 空にしたフィールド。
fn cleared(field: &str) -> anyhow::Result<NetField> {
    Ok(match field {
        "label" => NetField::Label(None),
        "source" => NetField::Source(None),
        "rail" => NetField::Rail(None),
        "limit" => NetField::Limit(None),
        "protocol" => NetField::Protocol(None),
        "bitrate" => NetField::Bitrate(None),
        "master" => NetField::Master(None),
        other => anyhow::bail!("消せるネットのフィールドではありません: {other}"),
    })
}
