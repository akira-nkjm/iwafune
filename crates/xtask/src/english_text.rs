//! Rust の文字列リテラルに日本語が戻らないかの検査（CONTRIBUTING.md「ユーザーに見せる文は英語だけ」）。
//!
//! - `crates/lib/*/src`・`crates/app/*/src`・`crates/xtask/src` のテスト以外のコードにある文字列リテラル
//!   （通常・raw・byte・char）に、ひらがな・カタカナ・漢字・全角記号が入っていたら違反にする。
//!   コメント（`//`・`///`・`//!`・`/* */`）と `#[cfg(test)]` の項目、`tests.rs`・`tests/` のファイルは見ない。
//! - 日本語を残してよいのは、日本語と英語の表示名の対（`label` + `label_en` など）を持つファイルだけ。
//!   [`ALLOWED`] に理由付きで置く。日本語のリテラルが無くなったファイルが残っていたら、それも違反にする。
//! - clap の `--help` になる `///` は英語。`crates/app/cli/src/args.rs` と `args/` の `///` には日本語を置かない。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod allowed;
use allowed::ALLOWED;

/// 走査するディレクトリの接頭辞（`crates/<lib|app>/<名前>/src/` と xtask 自身）。
fn in_scope(rel: &str) -> bool {
    let code = rel.ends_with(".rs");
    let src = (rel.starts_with("crates/lib/") || rel.starts_with("crates/app/"))
        && rel.split('/').nth(3) == Some("src");
    code && (src || rel.starts_with("crates/xtask/src/")) && !is_test_file(rel)
}

/// テスト専用のファイル（日本語のデータを使ってよい）。
fn is_test_file(rel: &str) -> bool {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    rel.split('/').any(|c| c == "tests")
        || name == "tests.rs"
        || name.ends_with("_tests.rs")
        || name == "testkit.rs"
}

/// ひらがな・カタカナ・漢字・CJK の記号・全角形（半角カナを含む）。
fn is_japanese(c: char) -> bool {
    matches!(c, '\u{3000}'..='\u{30FF}' | '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{FF00}'..='\u{FFEF}')
}

/// 日本語を含む文字列リテラルの位置（1 始まりの行）と中身。
#[derive(Debug, PartialEq, Eq)]
struct Finding {
    line: usize,
    text: String,
}

/// 状態を持つ走査器。コメントとテストの項目を飛ばし、リテラルだけを見る。
struct Scanner<'a> {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    out: &'a mut Vec<Finding>,
    /// 波括弧の深さ。
    depth: usize,
    /// `#[cfg(test)]` を読んだあと、次の項目の `{` か `;` を待っている。
    pending_test: bool,
    /// テストの項目に入った深さ（その深さに戻ったら終わり）。
    test_depth: Option<usize>,
}

impl Scanner<'_> {
    fn peek(&self, k: usize) -> Option<char> {
        self.chars.get(self.pos + k).copied()
    }

    /// 1 文字進む（改行で行を数える）。
    fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }

    fn skipping(&self) -> bool {
        self.test_depth.is_some()
    }

    fn record(&mut self, line: usize, text: String) {
        if !self.skipping() && text.chars().any(is_japanese) {
            self.out.push(Finding { line, text });
        }
    }

    fn run(&mut self) {
        while let Some(c) = self.peek(0) {
            match c {
                '/' if self.peek(1) == Some('/') => self.line_comment(),
                '/' if self.peek(1) == Some('*') => self.block_comment(),
                '"' => self.string(),
                '\'' => self.quote(),
                'r' | 'b' if self.raw_prefix().is_some() => self.raw_string(),
                '#' if self.cfg_test_attr() => self.pending_test = true,
                '{' => self.open_brace(),
                '}' => self.close_brace(),
                ';' if self.pending_test => {
                    self.pending_test = false;
                    self.bump();
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    fn open_brace(&mut self) {
        if self.pending_test && self.test_depth.is_none() {
            self.test_depth = Some(self.depth);
            self.pending_test = false;
        }
        self.depth += 1;
        self.bump();
    }

    fn close_brace(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        if self.test_depth == Some(self.depth) {
            self.test_depth = None;
        }
        self.bump();
    }

    fn line_comment(&mut self) {
        while let Some(c) = self.peek(0) {
            if c == '\n' {
                break;
            }
            self.bump();
        }
    }

    /// `/* … */`（入れ子にできる）。
    fn block_comment(&mut self) {
        let mut nest = 0usize;
        while let Some(c) = self.peek(0) {
            if c == '/' && self.peek(1) == Some('*') {
                nest += 1;
                self.bump();
                self.bump();
            } else if c == '*' && self.peek(1) == Some('/') {
                nest -= 1;
                self.bump();
                self.bump();
                if nest == 0 {
                    return;
                }
            } else {
                self.bump();
            }
        }
    }

    /// 通常の文字列（`"…"`、直前の `b` は前の文字として読み飛ばし済み）。
    fn string(&mut self) {
        let line = self.line;
        self.bump();
        let mut text = String::new();
        while let Some(c) = self.bump() {
            match c {
                '\\' => {
                    if let Some(n) = self.bump() {
                        text.push(n);
                    }
                }
                '"' => break,
                _ => text.push(c),
            }
        }
        self.record(line, text);
    }

    /// `'` は char リテラルか、ライフタイム・ラベルか。char なら中身を調べる。
    fn quote(&mut self) {
        let line = self.line;
        let is_char = match (self.peek(1), self.peek(2)) {
            (Some('\\'), _) => true,
            (Some(c), Some('\'')) => c != '\'',
            _ => false,
        };
        self.bump();
        if !is_char {
            return;
        }
        let mut text = String::new();
        while let Some(c) = self.bump() {
            match c {
                '\\' => {
                    if let Some(n) = self.bump() {
                        text.push(n);
                    }
                }
                '\'' => break,
                _ => text.push(c),
            }
        }
        self.record(line, text);
    }

    /// `r"`・`r#"`・`br#"` の始まりなら `#` の数を返す（識別子の途中の `r` は除く）。
    fn raw_prefix(&self) -> Option<usize> {
        if self.pos > 0 {
            let prev = self.chars[self.pos - 1];
            if prev.is_alphanumeric() || prev == '_' {
                return None;
            }
        }
        let mut k = 0;
        if self.peek(k) == Some('b') {
            k += 1;
        }
        if self.peek(k) != Some('r') {
            return None;
        }
        k += 1;
        let mut hashes = 0;
        while self.peek(k) == Some('#') {
            hashes += 1;
            k += 1;
        }
        (self.peek(k) == Some('"')).then_some(hashes)
    }

    fn raw_string(&mut self) {
        let hashes = self.raw_prefix().unwrap_or(0);
        let line = self.line;
        while self.peek(0) != Some('"') {
            self.bump();
        }
        self.bump();
        let mut text = String::new();
        while let Some(c) = self.peek(0) {
            if c == '"' && (1..=hashes).all(|k| self.peek(k) == Some('#')) {
                for _ in 0..=hashes {
                    self.bump();
                }
                break;
            }
            text.push(c);
            self.bump();
        }
        self.record(line, text);
    }

    /// 今の `#` が `#[cfg(test)]`（空白は無視）なら、その属性を読み終えて true を返す。
    fn cfg_test_attr(&mut self) -> bool {
        let rest: String = self.chars[self.pos..].iter().take(40).collect();
        let Some(end) = rest.find(']') else {
            return false;
        };
        let attr: String = rest[..=end]
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        if attr != "#[cfg(test)]" {
            return false;
        }
        for _ in 0..rest[..=end].chars().count() {
            self.bump();
        }
        true
    }
}

/// ソースのテキストから、テスト以外の日本語の文字列リテラルを探す。
fn scan(text: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut scanner = Scanner {
        chars: text.chars().collect(),
        pos: 0,
        line: 1,
        out: &mut out,
        depth: 0,
        pending_test: false,
        test_depth: None,
    };
    scanner.run();
    out
}

/// `args` の `///`（`--help` の文）に日本語がある行（1 始まり）。
fn japanese_doc_lines(text: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| l.trim_start().starts_with("///") && l.chars().any(is_japanese))
        .map(|(i, _)| i + 1)
        .collect()
}

fn is_cli_args(rel: &str) -> bool {
    rel == "crates/app/cli/src/args.rs" || rel.starts_with("crates/app/cli/src/args/")
}

/// 日本語を含むリテラルの一覧（ファイルごと）と許可リストから、違反の文を作る。
fn judge(found: &BTreeMap<String, Vec<Finding>>, allowed: &[(&str, &str)]) -> Vec<String> {
    let mut errors = Vec::new();
    for (rel, findings) in found {
        if allowed.iter().any(|(path, _)| path == rel) {
            continue;
        }
        for f in findings {
            let preview: String = f.text.chars().take(30).collect();
            errors.push(format!(
                "{rel}:{}: string literal with Japanese text `{preview}`. Text shown to users is English only; only files with bilingual name pairs go in english_text.rs ALLOWED, with a reason (CONTRIBUTING.md)",
                f.line
            ));
        }
    }
    for (path, reason) in allowed {
        if !found.contains_key(*path) {
            errors.push(format!(
                "english_text.rs ALLOWED lists `{path}` ({reason}) but it no longer has Japanese string literals; remove the entry"
            ));
        }
    }
    errors
}

/// 違反を `errors` に積む。`files` はワークスペースのファイル一覧。
pub fn check(root: &Path, files: &[PathBuf], errors: &mut Vec<String>) {
    let mut found: BTreeMap<String, Vec<Finding>> = BTreeMap::new();
    for file in files {
        let rel = file.strip_prefix(root).unwrap_or(file);
        let rel = rel.to_string_lossy().replace('\\', "/");
        if !in_scope(&rel) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        if is_cli_args(&rel) {
            for line in japanese_doc_lines(&text) {
                errors.push(format!(
                    "{rel}:{line}: Japanese `///` doc comment in the CLI argument definitions. These lines are `--help` text and must be English"
                ));
            }
        }
        let findings = scan(&text);
        if !findings.is_empty() {
            found.insert(rel, findings);
        }
    }
    errors.extend(judge(&found, ALLOWED));
}

#[cfg(test)]
mod tests;
