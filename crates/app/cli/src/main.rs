//! # fune — CLI
//!
//! 引数の解析と表示だけをする。処理はすべて `fune_service` を呼ぶ。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `args` | clap の定義（`fune <名詞> <動詞>` と短縮） |
//! | `cmd` | コマンドごとに service を呼び、結果を表示する |
//! | `output` | 表・色・状態の行・JSON |
//! | `diag` | 診断の表示（miette） |
//!
//! 終了コード: 0 正常 / 1 検証エラー / 2 引数エラー / 3 競合。

mod args;
mod cmd;
mod diag;
mod output;

use clap::Parser;
use fune_service::{ErrorKind, ServiceError};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = args::Cli::parse();
    let out = output::Out::new(cli.global.json, cli.global.color);
    match cmd::run(&cli, &out) {
        Ok(code) => ExitCode::from(code),
        Err(e) => ExitCode::from(report(&out, &e)),
    }
}

/// エラーを表示して終了コードを返す。
fn report(out: &output::Out, e: &anyhow::Error) -> u8 {
    let Some(se) = e.downcast_ref::<ServiceError>() else {
        if out.json {
            let body = serde_json::json!({ "error": { "code": "cli_error", "message": format!("{e:#}") }, "diagnostics": [] });
            println!("{body}");
        } else {
            out.fail(format!("{e:#}"));
        }
        return 2;
    };
    if out.json {
        if let Ok(text) = serde_json::to_string_pretty(&se.envelope()) {
            println!("{text}");
        }
    } else {
        out.fail(se);
        if !se.diagnostics().is_empty() {
            eprint!("{}", diag::render(se.diagnostics(), out.color));
        }
    }
    exit_code(se.kind())
}

/// エラーの分類 → 終了コード。
fn exit_code(kind: ErrorKind) -> u8 {
    match kind {
        ErrorKind::Validation | ErrorKind::Internal => 1,
        ErrorKind::InvalidInput | ErrorKind::NotFound => 2,
        ErrorKind::Conflict => 3,
    }
}
