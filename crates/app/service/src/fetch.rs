//! 外部からの取得（`curl`）の唯一の入口。HTTPS だけ・リダイレクトに従う・HTTP の誤りは失敗にする。
//! 宇宙天気の表と `fune self-update` が使う。取得したものの検証（SHA-256 など）は呼び出し側が行う。

use crate::error::{Result, ServiceError};
use std::path::Path;
use std::process::{Command, Output};

/// 取得の制限時間（秒）。
const MAX_TIME_SECS: &str = "300";

/// `curl` の共通の引数（HTTPS だけ・TLS 1.2 以上・リダイレクトに従う・HTTP の誤りは失敗にする）。
fn base() -> Command {
    let mut c = Command::new("curl");
    c.args(["--fail", "--silent", "--show-error", "--location"])
        .args(["--proto", "=https", "--proto-redir", "=https", "--tlsv1.2"])
        .args(["--max-time", MAX_TIME_SECS]);
    c
}

fn run(mut command: Command, url: &str) -> Result<Output> {
    let output = command
        .output()
        .map_err(|e| ServiceError::Data(format!("Cannot start curl: {e}")))?;
    if !output.status.success() {
        return Err(ServiceError::Data(format!(
            "Cannot fetch {url}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output)
}

/// `url` を `target` に保存する。
pub(crate) fn download(url: &str, target: &Path) -> Result<()> {
    let mut c = base();
    c.arg("--output").arg(target).arg(url);
    run(c, url).map(|_| ())
}

/// `url` の本文を文字列で取る。
pub(crate) fn text(url: &str) -> Result<String> {
    let mut c = base();
    c.arg(url);
    let output = run(c, url)?;
    String::from_utf8(output.stdout)
        .map_err(|_| ServiceError::Data(format!("The response from {url} is not UTF-8 text")))
}
