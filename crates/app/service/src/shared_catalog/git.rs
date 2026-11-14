//! git の CLI の呼び出し（取得元の写し = bare リポジトリ）。認証は手元の git の設定（資格情報の保存・SSH の鍵）に任せ、
//! 端末で聞き返させない（`GIT_TERMINAL_PROMPT=0`。GUI から呼んでも止まらない）。

use crate::error::{Result, ServiceError};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// 取得元の写し（`.fune/catalogs/<名前>/repo.git`）。
pub(super) struct Mirror {
    dir: PathBuf,
}

impl Mirror {
    /// 写しを開く（無ければ bare で作る）。
    pub(super) fn open(dir: &Path) -> Result<Self> {
        let mirror = Self {
            dir: dir.to_path_buf(),
        };
        if !dir.join("HEAD").is_file() {
            std::fs::create_dir_all(dir).map_err(|e| io_error(dir, &e))?;
            git(None, &["init", "--quiet", "--bare", &path_arg(dir)], None)?;
        }
        Ok(mirror)
    }

    /// `rev`（ブランチ・タグ・commit。None は既定のブランチ）を取得元から取り、解決した commit（完全な形）を返す。
    pub(super) fn resolve(&self, url: &str, rev: Option<&str>) -> Result<String> {
        let url = operand(url)?;
        let target = operand(rev.unwrap_or("HEAD"))?;
        match self.run(&["fetch", "--quiet", "--no-tags", "--force", url, target]) {
            Ok(_) => self.commit_of("FETCH_HEAD"),
            // commit を直に取れない取得元もあるので、ブランチとタグをまとめて取ってから引く
            Err(_) if rev.is_some_and(looks_like_commit) => {
                self.fetch_all(url)?;
                self.commit_of(target)
            }
            Err(e) => Err(e),
        }
    }

    /// commit が写しに無ければ取得元から取る。
    pub(super) fn ensure(&self, url: &str, commit: &str) -> Result<()> {
        let url = operand(url)?;
        let commit = operand(commit)?;
        if self.commit_of(commit).is_ok() {
            return Ok(());
        }
        if self
            .run(&["fetch", "--quiet", "--no-tags", "--force", url, commit])
            .is_err()
        {
            self.fetch_all(url)?;
        }
        self.commit_of(commit).map(|_| ()).map_err(|_| {
            ServiceError::Data(format!(
                "catalog.lock の commit `{commit}` が取得元 {url} にありません"
            ))
        })
    }

    /// commit の中の `dir` の直下のファイル（`<dir>/<名前>`, 本文）。ディレクトリが無ければ空。
    pub(super) fn files_in(&self, commit: &str, dir: &str) -> Result<Vec<(String, Vec<u8>)>> {
        let commit = operand(commit)?;
        let listing = self.run(&["ls-tree", "-z", commit, "--", &format!("{dir}/")])?;
        let mut blobs: Vec<(String, String)> = Vec::new();
        for entry in listing.split(|b| *b == 0).filter(|e| !e.is_empty()) {
            let entry = String::from_utf8_lossy(entry);
            let Some((meta, path)) = entry.split_once('\t') else {
                continue;
            };
            let mut meta = meta.split(' ');
            let (Some(_mode), Some("blob"), Some(oid)) = (meta.next(), meta.next(), meta.next())
            else {
                continue;
            };
            blobs.push((path.to_owned(), oid.to_owned()));
        }
        if blobs.is_empty() {
            return Ok(Vec::new());
        }
        let input: String = blobs.iter().map(|(_, oid)| format!("{oid}\n")).collect();
        let out = git(
            Some(&self.dir),
            &["cat-file", "--batch"],
            Some(input.as_bytes()),
        )?;
        let contents = split_batch(&out)?;
        Ok(blobs.into_iter().map(|(p, _)| p).zip(contents).collect())
    }

    fn commit_of(&self, rev: &str) -> Result<String> {
        let rev = operand(rev)?;
        let out = self.run(&[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{rev}^{{commit}}"),
        ])?;
        Ok(String::from_utf8_lossy(&out).trim().to_owned())
    }

    fn fetch_all(&self, url: &str) -> Result<()> {
        let refs = ["+refs/heads/*:refs/heads/*", "+refs/tags/*:refs/tags/*"];
        let mut args = vec!["fetch", "--quiet", "--force", url];
        args.extend(refs);
        self.run(&args).map(|_| ())
    }

    fn run(&self, args: &[&str]) -> Result<Vec<u8>> {
        git(Some(&self.dir), args, None)
    }
}

/// fune.toml・catalog.lock から来た取得元・rev・commit を git に渡せるか。`-` で始まるものは git がオプションと読むので断る
/// （`--upload-pack=…` などで手元のコマンドを動かされないように）。
fn operand(value: &str) -> Result<&str> {
    if value.starts_with('-') {
        return Err(ServiceError::Data(format!(
            "`{value}` は `-` で始まるので git に渡せません（共有カタログの git・rev・commit）"
        )));
    }
    Ok(value)
}

/// 16 進数の commit（短縮を含む）に見えるか。
fn looks_like_commit(rev: &str) -> bool {
    (7..=64).contains(&rev.len()) && rev.bytes().all(|b| b.is_ascii_hexdigit())
}

/// `cat-file --batch` の出力（`<oid> blob <大きさ>\n<本文>\n` の繰り返し）→ 本文の列。
fn split_batch(out: &[u8]) -> Result<Vec<Vec<u8>>> {
    let bad = || ServiceError::Data("git cat-file の出力を読めません".into());
    let mut rest = out;
    let mut contents = Vec::new();
    while !rest.is_empty() {
        let end = rest.iter().position(|b| *b == b'\n').ok_or_else(bad)?;
        let header = String::from_utf8_lossy(&rest[..end]);
        let size: usize = header
            .rsplit(' ')
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(bad)?;
        let body = rest.get(end + 1..end + 1 + size).ok_or_else(bad)?;
        contents.push(body.to_vec());
        rest = rest.get(end + 2 + size..).unwrap_or_default();
    }
    Ok(contents)
}

/// `git [--git-dir <dir>] <args>` を実行して標準出力を返す（失敗は標準エラーの文）。
fn git(dir: Option<&Path>, args: &[&str], stdin: Option<&[u8]>) -> Result<Vec<u8>> {
    let mut cmd = Command::new("git");
    if let Some(dir) = dir {
        cmd.arg("--git-dir").arg(dir);
    }
    cmd.args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| {
        ServiceError::Data(format!(
            "git を実行できません（git をインストールしてください）: {e}"
        ))
    })?;
    // 出力を読みながら書く（大きな入力で出力の管が詰まって止まらないよう、書くのは別のスレッド）
    let writer = match (stdin, child.stdin.take()) {
        (Some(input), Some(mut pipe)) => {
            let input = input.to_vec();
            Some(std::thread::spawn(move || pipe.write_all(&input)))
        }
        _ => None,
    };
    let out = child
        .wait_with_output()
        .map_err(|e| ServiceError::Data(format!("git の終わりを待てません: {e}")))?;
    if let Some(writer) = writer {
        writer
            .join()
            .map_err(|_| ServiceError::Data("git に書けません".into()))?
            .map_err(|e| ServiceError::Data(format!("git に書けません: {e}")))?;
    }
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(ServiceError::Data(format!(
            "git {} に失敗しました: {}",
            args.first().copied().unwrap_or_default(),
            stderr.trim()
        )));
    }
    Ok(out.stdout)
}

fn path_arg(path: &Path) -> String {
    path.display().to_string()
}

pub(super) fn io_error(path: &Path, e: &std::io::Error) -> ServiceError {
    ServiceError::Data(format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_cat_file_batch_output() {
        let out = b"aaa blob 3\nabc\nbbb blob 0\n\nccc blob 2\nx\n\n";
        let parts = split_batch(out).unwrap();
        assert_eq!(parts, [b"abc".to_vec(), Vec::new(), b"x\n".to_vec()]);
        assert!(split_batch(b"aaa blob 9\nabc\n").is_err());
        assert!(looks_like_commit("0123abc") && !looks_like_commit("v1.0"));
        assert!(operand("--upload-pack=touch x").is_err() && operand("file:///r").is_ok());
    }
}
