//! 開発用の検査。`cargo xtask check` で実行する（CI でも同じものを流す）。
//!
//! - 依存の向き: ワークスペース内の crate が、許された crate にだけ依存しているか（CONTRIBUTING.md の「依存の向き」）
//! - crate 内モジュールの循環: production の crate / super 参照（公開 re-export は定義先へ解決）
//! - 台帳: すべての crate が docs/architecture/registry.md に載っているか
//! - ファイルの長さ: 400 行を超える Rust / TS のファイルを警告する（CONTRIBUTING.md の目安）
//! - 言語: Python のファイルが無いか（言語は Rust と TS だけ）
//! - ファイル名: 同じディレクトリで大文字・小文字だけ違う名前が無いか（[`file_names`]。macOS で import が取り違える）
//! - projection にモデル化の既定値を書かない: `const DEFAULT_…` と値の直書きの `unwrap_or(`（[`projection_defaults`]）
//! - 1 機能 1 か所: 台帳で決めた定義が別のファイルに現れていないか（[`single_home`]）
//! - 設計ファイルの書き込みの入口: `write::commit` を呼ぶのは store と ops だけ（例外は理由付きの許可リスト。[`store_writes`]）
//! - Web の文字列の分解: ID やパスを `/`・`.` で分けていないか（[`web_strings`]）
//! - Web の色の直書き: hex・色の名前（`white` など）・`color-mix(` が `lib/tone.ts` の外に無いか（[`web_colors`]）
//! - Web のページ: ページの URL を `lib/nav.ts` の外で組み立てていないか、`features/` に shell などから依存していないか
//!   （[`web_pages`]）
//! - Web の役割名の比較: 部品の役割を文字列と比べて決めていないか（[`web_roles`]）
//! - Web の系の名前の比較: 系を表す識別子を文字列と比べて決めていないか（[`web_systems`]）

mod english_text;
mod file_names;
mod module_cycles;
mod projection_defaults;
mod single_home;
mod store_writes;
mod web_colors;
mod web_pages;
mod web_roles;
mod web_strings;
mod web_systems;

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
    ("fune-report", &["sat-units"]),
    (
        "fune-service",
        &[
            "fune-model",
            "fune-store",
            "fune-ops",
            "fune-projection",
            "fune-report",
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
        _ => bail!("usage: cargo xtask check"),
    }
}

fn check() -> Result<()> {
    let meta = metadata()?;
    let mut errors = Vec::new();
    check_dependencies(&meta, &mut errors);
    check_registry(&meta, &mut errors)?;
    check_languages(&meta.workspace_root, &mut errors)?;
    let files = source_files(&meta.workspace_root)?;
    english_text::check(&meta.workspace_root, &files, &mut errors);
    file_names::check(&meta.workspace_root, &files, &mut errors);
    single_home::check(&meta.workspace_root, &files, &mut errors);
    projection_defaults::check(&meta.workspace_root, &files, &mut errors);
    store_writes::check(&meta.workspace_root, &files, &mut errors);
    web_strings::check(&meta.workspace_root, &files, &mut errors);
    web_colors::check(&meta.workspace_root, &files, &mut errors);
    web_pages::check(&meta.workspace_root, &files, &mut errors);
    web_roles::check(&meta.workspace_root, &files, &mut errors);
    web_systems::check(&meta.workspace_root, &files, &mut errors);
    module_cycles::check(&meta.workspace_root, &mut errors)?;
    warn_long_files(&meta.workspace_root)?;
    if errors.is_empty() {
        println!("xtask check: OK ({} crates)", meta.packages.len());
        Ok(())
    } else {
        for e in &errors {
            eprintln!("error: {e}");
        }
        bail!("xtask check: {} violation(s)", errors.len())
    }
}

fn metadata() -> Result<Metadata> {
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

fn check_dependencies(meta: &Metadata, errors: &mut Vec<String>) {
    let allowed: BTreeMap<&str, &[&str]> = ALLOWED.iter().copied().collect();
    let members: Vec<&str> = meta.packages.iter().map(|p| p.name.as_str()).collect();
    for pkg in &meta.packages {
        let Some(ok) = allowed.get(pkg.name.as_str()) else {
            errors.push(format!(
                "crate `{}` is not registered in the dependency rules (ALLOWED in xtask)",
                pkg.name
            ));
            continue;
        };
        for dep in &pkg.dependencies {
            let internal = members.contains(&dep.name.as_str());
            let is_dev = dep.kind.as_deref() == Some("dev");
            if internal && !is_dev && !ok.contains(&dep.name.as_str()) {
                errors.push(format!(
                    "`{}` must not depend on `{}` (dependency direction in CONTRIBUTING.md)",
                    pkg.name, dep.name
                ));
            }
        }
    }
}

fn check_registry(meta: &Metadata, errors: &mut Vec<String>) -> Result<()> {
    let path = meta.workspace_root.join("docs/architecture/registry.md");
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read {}", path.display()))?;
    for pkg in &meta.packages {
        if pkg.name != "xtask" && !text.contains(&format!("`{}`", pkg.name)) {
            errors.push(format!(
                "crate `{}` is not listed in the registry (registry.md)",
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
                "Python file found: {} (only Rust and TS are used)",
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
                "warning: {} has {lines} lines (guideline: {MAX_LINES})",
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
