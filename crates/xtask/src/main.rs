//! 開発用の検査。`cargo xtask check` で実行する（CI でも同じものを流す）。
//!
//! - 依存の向き: ワークスペース内の crate が、許された crate にだけ依存しているか（CONTRIBUTING.md の「依存の向き」）
//! - crate 内モジュールの循環: production の crate / super 参照（公開 re-export は定義先へ解決）
//! - 台帳: すべての crate が docs/architecture/registry.md に載っているか
//! - ファイルの長さ: 400 行を超える Rust / TS のファイルを警告する（CONTRIBUTING.md の目安）
//! - 言語: Python のファイルが無いか（言語は Rust と TS だけ）
//! - 1 機能 1 か所: 台帳で決めた定義が別のファイルに現れていないか（[`single_home`]）
//! - Web の文字列の分解: ID やパスを `/`・`.` で分けていないか（[`web_strings`]）

mod module_cycles;
mod single_home;
mod web_strings;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// crate ごとに、依存してよいワークスペース内の crate。ここに無い crate を足したら検査が失敗する。
const ALLOWED: &[(&str, &[&str])] = &[
    ("sat-units", &[]),
    ("sat-astro", &["sat-units"]),
    ("sat-analysis", &["sat-units", "sat-astro"]),
    ("fune-model", &["sat-units"]),
    ("fune-store", &["fune-model"]),
    ("fune-ops", &["fune-model", "fune-store"]),
    (
        "fune-projection",
        &["fune-model", "sat-analysis", "sat-astro", "sat-units"],
    ),
    (
        "fune-service",
        &[
            "fune-model",
            "fune-store",
            "fune-ops",
            "fune-projection",
            "sat-analysis",
            "sat-astro",
            "sat-units",
        ],
    ),
    ("fune-server", &["fune-service"]),
    ("fune-cli", &["fune-service", "fune-server"]),
    ("xtask", &[]),
];

const MAX_LINES: usize = 400;

/// 行数の目安から外すデータの表（ロジックを持たない係数表）。
const LONG_DATA_FILES: &[&str] = &["crates/lib/astro/src/atmosphere/nrlmsise00/coeffs.rs"];

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_root: PathBuf,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
struct Dependency {
    name: String,
    kind: Option<String>,
}

fn main() -> Result<()> {
    let task = std::env::args().nth(1).unwrap_or_default();
    match task.as_str() {
        "check" => check(),
        _ => bail!("使い方: cargo xtask check"),
    }
}

fn check() -> Result<()> {
    let meta = metadata()?;
    let mut errors = Vec::new();
    check_dependencies(&meta, &mut errors);
    check_registry(&meta, &mut errors)?;
    check_languages(&meta.workspace_root, &mut errors)?;
    let files = source_files(&meta.workspace_root)?;
    single_home::check(&meta.workspace_root, &files, &mut errors);
    web_strings::check(&meta.workspace_root, &files, &mut errors);
    module_cycles::check(&meta.workspace_root, &mut errors)?;
    warn_long_files(&meta.workspace_root)?;
    if errors.is_empty() {
        println!("xtask check: OK（{} crate）", meta.packages.len());
        Ok(())
    } else {
        for e in &errors {
            eprintln!("error: {e}");
        }
        bail!("xtask check: {} 件の違反", errors.len())
    }
}

fn metadata() -> Result<Metadata> {
    let out = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .context("cargo metadata を実行できません")?;
    if !out.status.success() {
        bail!(
            "cargo metadata が失敗しました: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    serde_json::from_slice(&out.stdout).context("cargo metadata の出力を解釈できません")
}

fn check_dependencies(meta: &Metadata, errors: &mut Vec<String>) {
    let allowed: BTreeMap<&str, &[&str]> = ALLOWED.iter().copied().collect();
    let members: Vec<&str> = meta.packages.iter().map(|p| p.name.as_str()).collect();
    for pkg in &meta.packages {
        let Some(ok) = allowed.get(pkg.name.as_str()) else {
            errors.push(format!(
                "crate `{}` が依存ルール（xtask の ALLOWED）に登録されていません",
                pkg.name
            ));
            continue;
        };
        for dep in &pkg.dependencies {
            let internal = members.contains(&dep.name.as_str());
            let is_dev = dep.kind.as_deref() == Some("dev");
            if internal && !is_dev && !ok.contains(&dep.name.as_str()) {
                errors.push(format!(
                    "`{}` は `{}` に依存してはいけません（CONTRIBUTING.md の「依存の向き」）",
                    pkg.name, dep.name
                ));
            }
        }
    }
}

fn check_registry(meta: &Metadata, errors: &mut Vec<String>) -> Result<()> {
    let path = meta.workspace_root.join("docs/architecture/registry.md");
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("{} を読めません", path.display()))?;
    for pkg in &meta.packages {
        if pkg.name != "xtask" && !text.contains(&format!("`{}`", pkg.name)) {
            errors.push(format!(
                "crate `{}` が構成台帳（registry.md）に載っていません",
                pkg.name
            ));
        }
    }
    Ok(())
}

fn check_languages(root: &Path, errors: &mut Vec<String>) -> Result<()> {
    for file in source_files(root)? {
        if file.extension().is_some_and(|e| e == "py") {
            errors.push(format!(
                "Python のファイルがあります: {}（言語は Rust と TS だけ）",
                file.display()
            ));
        }
    }
    Ok(())
}

fn warn_long_files(root: &Path) -> Result<()> {
    for file in source_files(root)? {
        let is_code = file
            .extension()
            .is_some_and(|e| e == "rs" || e == "ts" || e == "tsx");
        let generated = file.components().any(|c| c.as_os_str() == "gen");
        let data = LONG_DATA_FILES.iter().any(|d| file.ends_with(d));
        if !is_code || generated || data {
            continue;
        }
        let lines = std::fs::read_to_string(&file)
            .map(|t| t.lines().count())
            .unwrap_or(0);
        if lines > MAX_LINES {
            println!(
                "warning: {} は {lines} 行です（目安 {MAX_LINES} 行）",
                file.display()
            );
        }
    }
    Ok(())
}

/// target・node_modules・.git などを除いたファイルの一覧。
fn source_files(root: &Path) -> Result<Vec<PathBuf>> {
    const SKIP: &[&str] = &["target", "node_modules", ".git", "dist", ".fune", ".claude"];
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
