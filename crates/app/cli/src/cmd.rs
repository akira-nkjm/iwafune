//! コマンドごとの処理。どれも service を呼んで表示するだけ。
//!
//! | モジュール | コマンド |
//! |---|---|
//! | `common` | コマンド間で共有する型・引数補助・出所の表示 |
//! | `parts` | part ls/show |
//! | `read` | status・check・get・mode ls/table・net ls・attr ls・kind ls・setting ls・view ls・case/source/harness ls |
//! | `ops` | 編集のコマンドの引数 → Op の列（`nouns`・`records` の組み立てを呼ぶ） |
//! | `nouns` | part・mode・setting・view の編集の引数 → Op |
//! | `nets` | net の編集の引数 → Op |
//! | `defs` | attr/kind の add・replace・rm の引数 → Op |
//! | `records` | case・source・harness・catalog・part point の引数 → Op |
//! | `catalog` | catalog ls/show/fetch/update と、catalog source add/rm のあとの案内 |
//! | `edit` | Op の列の適用と結果の表示・init |
//! | `units` | units ls/parse/format |
//! | `analyze` | analyze・budget |
//! | `gui` | gui |
//! | `data` | data status/fetch |
//! | `self_update` | self-update（実行中の `fune` の置き換え。プロジェクト不要） |

mod analyze;
mod catalog;
mod common;
mod data;
mod defs;
mod edit;
mod gui;
mod maneuvers;
mod nets;
mod nouns;
mod ops;
mod output_tables;
mod parts;
mod read;
mod records;
#[cfg(test)]
mod roundtrip;
mod self_update;
mod units;

use crate::args::{Cli, Command};
use crate::output::Out;
use fune_service::Workspace;

/// コマンドを実行して終了コードを返す。
pub fn run(cli: &Cli, out: &Out) -> anyhow::Result<u8> {
    fune_service::data::install_cached_ephemeris();
    let g = &cli.global;
    match &cli.command {
        Command::Units(c) => units::run(c, out),
        Command::Data(c) => data::run(c, out),
        Command::SelfUpdate { check, version } => self_update::run(*check, version.as_deref(), out),
        Command::Gui {
            port,
            host,
            allow_host,
            no_open,
        } => gui::run(&open(g)?, host, *port, allow_host, *no_open, out),
        Command::Analyze(a) if a.target == "ls" => analyze::ls(out),
        Command::Analyze(a) => analyze::run(&open(g)?, a, out),
        Command::Budget { kind, case, ops } => {
            analyze::budget(&open(g)?, kind, case.as_deref(), ops.as_deref(), out)
        }
        Command::Fmt { check } => {
            let result = fune_service::format::format_design(
                &open(g)?,
                &fune_service::dto::FormatDesignRequest { check: *check },
            )?;
            if out.json {
                out.json(&result)?;
            } else {
                for path in &result.changed {
                    println!("{path}");
                }
            }
            Ok(u8::from(*check && !result.changed.is_empty()))
        }
        Command::Init { name } => edit::init(&g.dir, name, out),
        Command::Status => read::status(&open(g)?, out),
        Command::Check { deny_warnings } => read::check(&open(g)?, *deny_warnings, out),
        Command::Get { path, state } => read::get(&open(g)?, path, state.as_deref(), out),
        command => {
            let ws = open(g)?;
            if let Some(code) = read_more(command, &ws, out)? {
                return Ok(code);
            }
            let Some(e) = ops::edit_of(&ws, command)? else {
                anyhow::bail!("this command is not handled here");
            };
            let code = edit::run_ops(&ws, e.ops, e.opts, out)?;
            if let Command::Catalog(c) = command {
                catalog::after_edit(c, e.opts, out);
            }
            Ok(code)
        }
    }
}

/// 名詞のコマンドのうち読み取りのもの（`ls`・`show`・`table`）。
fn read_more(command: &Command, ws: &Workspace, out: &Out) -> anyhow::Result<Option<u8>> {
    match command {
        Command::Part(c) => parts::part(ws, c, out),
        Command::Mode(c) => read::mode(ws, c, out),
        Command::Net(c) => read::net(ws, c, out),
        Command::Attr(c) => read::attr(ws, c, out),
        Command::Kind(c) => read::kind(ws, c, out),
        Command::Setting(c) => read::setting(ws, c, out),
        Command::View(c) => read::view(ws, c, out),
        Command::Case(c) => read::case(ws, c, out),
        Command::Source(c) => read::source(ws, c, out),
        Command::Harness(c) => read::harness(ws, c, out),
        Command::Maneuver(c) => maneuvers::ls(ws, c, out),
        Command::Catalog(c) => catalog::read(ws, c, out),
        _ => Ok(None),
    }
}

fn open(g: &crate::args::GlobalOpts) -> anyhow::Result<Workspace> {
    Ok(Workspace::open(&g.dir)?)
}
