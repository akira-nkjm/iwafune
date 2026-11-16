//! 外部からの取得（`curl`）の唯一の入口。HTTPS だけ・リダイレクトに従う・HTTP の誤りは失敗にする。
//! 宇宙天気の表と `fune self-update` が使う。取得したものの検証（SHA-256 など）は呼び出し側が行う。

use crate::error::{Result, ServiceError};
use std::path::Path;
use std::process::{Command, Output, Stdio};

/// 取得の制限時間（秒）。
const MAX_TIME_SECS: &str = "300";

/// `curl` の共通の引数（HTTPS だけ・TLS 1.2 以上・リダイレクトに従う・HTTP の誤りは失敗にする）。
/// `progress` のときは進捗バーを描かせる（`--silent` の代わりに `--progress-bar`。バーは curl が標準エラーへ描く）。
fn base(progress: bool) -> Command {
    let mut c = Command::new("curl");
    c.args(["--fail", "--show-error", "--location"])
        .arg(if progress {
            "--progress-bar"
        } else {
            "--silent"
        })
        .args(["--proto", "=https", "--proto-redir", "=https", "--tlsv1.2"])
        .args(["--max-time", MAX_TIME_SECS]);
    c
}

fn run(mut command: Command, url: &str) -> Result<Output> {
    let output = command
        .output()
        .map_err(|e| ServiceError::Data(format!("Cannot start curl: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // 標準エラーを継承したとき（進捗バー）は本文が空なので、`--write-out` の HTTP ステータスで説明する
        let detail = match stderr.trim() {
            "" => format!("HTTP {}", String::from_utf8_lossy(&output.stdout).trim()),
            text => text.to_owned(),
        };
        return Err(ServiceError::Data(format!("Cannot fetch {url}: {detail}")));
    }
    Ok(output)
}

/// `url` を `target` に保存する。`progress` のときは進捗バーを標準エラーに描く（端末のときだけ渡す）。
pub(crate) fn download(url: &str, target: &Path, progress: bool) -> Result<()> {
    let mut c = base(progress);
    if progress {
        c.stderr(Stdio::inherit());
    }
    c.args(["--write-out", "%{http_code}"]);
    c.arg("--output").arg(target).arg(url);
    run(c, url).map(|_| ())
}

/// `url` の本文を文字列で取る。
pub(crate) fn text(url: &str) -> Result<String> {
    let mut c = base(false);
    c.arg(url);
    let output = run(c, url)?;
    String::from_utf8(output.stdout)
        .map_err(|_| ServiceError::Data(format!("The response from {url} is not UTF-8 text")))
}
