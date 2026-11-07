//! `fune data status|fetch`

use crate::args::DataCmd;
use crate::output::{C, Out};
use fune_service::data;
use fune_service::dto::DataStatus;

/// 外部データのコマンド（プロジェクトが無くても使える）。
pub fn run(cmd: &DataCmd, out: &Out) -> anyhow::Result<u8> {
    let status = match cmd {
        DataCmd::Status => data::data_status()?,
        DataCmd::Fetch { id } => {
            let s = data::fetch(id)?;
            out.ok(format!("{id} を取得し、SHA-256 を確かめました"));
            s
        }
    };
    show(&status, out)?;
    Ok(0)
}

fn show(s: &DataStatus, out: &Out) -> anyhow::Result<()> {
    if out.json {
        return out.json(s);
    }
    let mut t = out
        .table(&["ID", "状態", "大きさ", "読み込み", "場所"])
        .numeric(&[2]);
    for item in &s.items {
        let state = match (item.present, item.verified) {
            (false, _) => C::Dim("未取得".into()),
            (true, Some(true)) => C::Verdict("✔ SHA-256 一致".into(), true),
            (true, _) => C::Verdict("✖ SHA-256 不一致".into(), false),
        };
        let size = item
            .size_display
            .as_ref()
            .map_or(C::Dim("—".into()), |q| C::Num(q.display.clone()));
        let installed = if item.installed {
            C::Verdict("済".into(), true)
        } else {
            C::Dim(item.install_error.clone().unwrap_or_else(|| "—".into()))
        };
        t.row(vec![
            C::Strong(item.id.clone()),
            state,
            size,
            installed,
            C::Dim(item.path.clone()),
        ]);
    }
    t.print();
    if s.items.iter().any(|i| !i.present) {
        println!("{}", out.dim("取得: fune data fetch de440"));
    }
    Ok(())
}
