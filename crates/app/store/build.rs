//! `fixtures/demo-6u` の全ファイルを列挙し、`include_str!` で同梱する表（`demo_files.rs`）を OUT_DIR に作る。
//! 並びはパスの昇順で、ビルドのたびに同じになる。`.fune/`（作業用のキャッシュ・履歴）と `.DS_Store` は設計データではないので含めない。

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// 同梱するディレクトリ（ワークスペースの根からの相対）。
const DEMO_DIR: &str = "fixtures/demo-6u";

fn main() {
    if let Err(e) = run() {
        eprintln!("build.rs: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?);
    let root = manifest.join("../../..").join(DEMO_DIR);
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    walk(&root, &root, &mut files)?;
    files.sort();
    let mut text = String::from("&[\n");
    for rel in &files {
        let abs = root.join(rel);
        writeln!(
            text,
            "    ({rel:?}, include_str!({:?})),",
            abs.canonicalize()
                .map_err(|e| e.to_string())?
                .display()
                .to_string()
        )
        .map_err(|e| e.to_string())?;
    }
    text.push_str("]\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").map_err(|e| e.to_string())?);
    std::fs::write(out.join("demo_files.rs"), text).map_err(|e| e.to_string())
}

/// `dir` の下のファイルを、`root` からの相対パス（`/` 区切り）で集める。
fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == ".fune" || name == ".DS_Store" {
            continue;
        }
        // 変更の検知は個々のファイルにも掛ける（ディレクトリの mtime は中身の書き換えでは変わらない）
        println!("cargo:rerun-if-changed={}", path.display());
        if path.is_dir() {
            walk(root, &path, out)?;
        } else {
            let rel = path.strip_prefix(root).map_err(|e| e.to_string())?;
            let parts: Vec<_> = rel.iter().map(|p| p.to_string_lossy()).collect();
            out.push(parts.join("/"));
        }
    }
    Ok(())
}
