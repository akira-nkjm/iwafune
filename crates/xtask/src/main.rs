//! 開発用の検査。`cargo xtask check` で実行する（CI でも同じものを流す）。
//!
//! - 依存の向き: ワークスペース内の crate が、許された crate にだけ依存しているか（CONTRIBUTING.md の「依存の向き」。[`deps`]）
//! - crate 内モジュールの循環: production の crate / super 参照（公開 re-export は定義先へ解決。[`module_cycles`]）
//! - 台帳: すべての crate が docs/architecture/registry.md の crate の表に載っているか（[`registry`]）
//! - 大きさの警告: 400 行を超える Rust / TS のファイルと、60 行を超える Rust の関数（エラーにはしない。[`sizes`]）
//! - 言語: Python のファイルが無いか（言語は Rust と TS だけ。このファイルの [`check_languages`]）
//! - Rust の文字列リテラルに日本語が戻っていないか（[`english_text`]）
//! - ファイル名: 同じディレクトリで大文字・小文字だけ違う名前が無いか（[`file_names`]。macOS で import が取り違える）
//! - projection にモデル化の既定値を書かない: `const DEFAULT_…` と値の直書きの `unwrap_or(`（[`projection_defaults`]）
//! - 1 機能 1 か所: 台帳で決めた定義が別のファイルに現れていないか。許したパスのどこにも現れない（死んだ）定義の規則も違反にする（[`single_home`]）
//! - 設計ファイルの書き込みの入口: `write::commit` を呼ぶのは store と ops だけ（例外は理由付きの許可リスト。[`store_writes`]）
//! - Web の文字列の分解: ID やパスを `/`・`.` で分けていないか（[`web_strings`]）
//! - Web の色の直書き: hex・色の名前（`white` など）・`color-mix(` が `lib/tone.ts` の外に無いか（[`web_colors`]）
//! - Web のページ: ページの URL を `lib/nav.ts` の外で組み立てていないか、`features/` に shell などから依存していないか
//!   （[`web_pages`]）
//! - Web の役割名の比較: 部品の役割を文字列と比べて決めていないか（[`web_roles`]）
//! - Web の系の名前の比較: 系を表す識別子を文字列と比べて決めていないか（[`web_systems`]）
//!
//! テスト項目の判別（`#[cfg(test)]` の項目を取り除いた本番のコード）は [`scan`] が 1 か所で持つ。

mod deps;
mod english_text;
mod file_names;
mod meta;
mod module_cycles;
mod projection_defaults;
mod registry;
mod scan;
mod single_home;
mod sizes;
mod store_writes;
mod web_colors;
mod web_pages;
mod web_roles;
mod web_strings;
mod web_systems;

use anyhow::{Result, bail};
use std::path::PathBuf;

fn main() -> Result<()> {
    let task = std::env::args().nth(1).unwrap_or_default();
    match task.as_str() {
        "check" => check(),
        _ => bail!("usage: cargo xtask check"),
    }
}

fn check() -> Result<()> {
    let meta = meta::load()?;
    let root = &meta.workspace_root;
    // ファイルの一覧は 1 回だけ作り、各検査に渡す
    let files = meta::source_files(root)?;
    let mut errors = Vec::new();
    deps::check(&meta.packages, &mut errors);
    registry::check(root, &meta.packages, &mut errors)?;
    check_languages(&files, &mut errors);
    english_text::check(root, &files, &mut errors);
    file_names::check(root, &files, &mut errors);
    single_home::check(root, &files, &mut errors);
    projection_defaults::check(root, &files, &mut errors);
    store_writes::check(root, &files, &mut errors);
    web_strings::check(root, &files, &mut errors);
    web_colors::check(root, &files, &mut errors);
    web_pages::check(root, &files, &mut errors);
    web_roles::check(root, &files, &mut errors);
    web_systems::check(root, &files, &mut errors);
    module_cycles::check(root, &mut errors)?;
    sizes::warn(root, &files);
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

/// Python のファイルがないこと（言語は Rust と TS だけ）。
fn check_languages(files: &[PathBuf], errors: &mut Vec<String>) {
    errors.extend(python_files(files));
}

/// Python のファイル（拡張子 `py`）の違反の文。
fn python_files(files: &[PathBuf]) -> Vec<String> {
    files
        .iter()
        .filter(|f| f.extension().is_some_and(|e| e == "py"))
        .map(|f| {
            format!(
                "Python file found: {} (only Rust and TS are used)",
                f.display()
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn only_python_files_are_reported() {
        let files = [
            Path::new("a/b.py").to_path_buf(),
            Path::new("a/b.rs").to_path_buf(),
            Path::new("a/py").to_path_buf(),
        ];
        let errors = python_files(&files);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("a/b.py"));
    }
}
