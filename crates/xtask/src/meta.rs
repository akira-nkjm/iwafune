//! `cargo metadata` の読み込みと、ソースのファイルの一覧（検査が共有する入力）。

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `cargo metadata` の必要な部分。
#[derive(Deserialize)]
pub struct Metadata {
    /// ワークスペースの crate。
    pub packages: Vec<Package>,
    /// ワークスペースのルート。
    pub workspace_root: PathBuf,
}

/// ワークスペースの crate 1 つ。
#[derive(Deserialize)]
pub struct Package {
    /// crate 名。
    pub name: String,
    /// 依存。
    pub dependencies: Vec<Dependency>,
}

/// crate の依存 1 つ。
#[derive(Deserialize)]
pub struct Dependency {
    /// 依存先の名前。
    pub name: String,
    /// `dev` / `build` / なし（通常）。
    pub kind: Option<String>,
}

/// `cargo metadata --no-deps` を読む。
pub fn load() -> Result<Metadata> {
    let out = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .context("cannot run cargo metadata")?;
    if !out.status.success() {
        bail!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    serde_json::from_slice(&out.stdout).context("cannot parse the output of cargo metadata")
}

/// 見ないディレクトリ名。
const SKIP: &[&str] = &["target", "node_modules", ".git", "dist", ".fune", ".claude"];

/// target・node_modules・.git などを除いたファイルの一覧。
pub fn source_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if entry.file_type()?.is_dir() {
                if !SKIP.contains(&name.as_ref()) && !name.starts_with("target") {
                    stack.push(path);
                }
            } else {
                out.push(path);
            }
        }
    }
    Ok(out)
}
