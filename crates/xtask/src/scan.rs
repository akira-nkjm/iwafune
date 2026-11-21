//! Rust のソースを読む検査が共有する分類器。
//!
//! - `is_test_file`: パスからテスト専用のファイルかを決める（唯一の定義）。
//! - `production_text`: テストの項目だけを取り除いた本番のコード。`#[cfg(test)]` が付いた項目
//!   （ファイルの途中の `fn`・`use`・`mod x;`・impl の中のメソッドを含む）を、その項目の終わりまで消す。
//!   消した部分は空白に置き換え、改行は残すので、行番号は元のファイルと同じ。
//! - `function_lengths`: 関数ごとの行数（長い関数の警告用）。
//!
//! 波括弧の対応は、コメント・文字列・raw 文字列・文字リテラルの中を無視して数える（このコードベースで足りる程度）。
//! 文字を 1 つずつ読む自前の走査で、`english_text` の走査器（リテラルの中身を見る別の仕事）とは分けてある。

/// テスト専用のファイルか（本番のコードを見る検査から外す）。
pub fn is_test_file(rel: &str) -> bool {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    rel.contains("/tests/")
        || rel.starts_with("crates/app/cli/src/cmd/roundtrip/")
        || name == "tests.rs"
        || name.ends_with("_tests.rs")
        || name == "testkit.rs"
        || name.contains(".test.")
}

/// テスト以外の部分（テストのファイルは None、それ以外は `#[cfg(test)]` の項目を空白にしたもの）。
pub fn production_text(rel: &str, text: &str) -> Option<String> {
    (!is_test_file(rel)).then(|| strip_test_items(text))
}

/// `#[cfg(test)]` が付いた項目を取り除く（改行は残す）。
pub fn strip_test_items(text: &str) -> String {
    let c: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < c.len() {
        if let Some(end) = lexeme_end(&c, i) {
            out.extend(&c[i..end]);
            i = end;
            continue;
        }
        if c[i] == '#'
            && let Some(attr_end) = cfg_test_attr_end(&c, i)
        {
            let end = item_end(&c, attr_end);
            out.extend(
                c[i..end]
                    .iter()
                    .map(|&ch| if ch == '\n' { '\n' } else { ' ' }),
            );
            i = end;
            continue;
        }
        out.push(c[i]);
        i += 1;
    }
    out
}

/// 関数 1 つの長さ。
#[derive(Debug, PartialEq, Eq)]
pub struct FnSpan {
    /// 関数名。
    pub name: String,
    /// `fn` のある行（1 始まり）。
    pub line: usize,
    /// `fn` の行から閉じ括弧の行まで（両端を含む）の行数。
    pub len: usize,
}

/// 本体のある関数（`fn`）の行数の一覧。入れ子の関数は別に数える。宣言だけ（`;` で終わるもの）は含めない。
pub fn function_lengths(code: &str) -> Vec<FnSpan> {
    let c: Vec<char> = code.chars().collect();
    let line_of = line_table(&c);
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        if let Some(end) = lexeme_end(&c, i) {
            i = end;
            continue;
        }
        let Some(name_at) = fn_keyword_name_start(&c, i) else {
            i += 1;
            continue;
        };
        let name_end = ident_end(&c, name_at);
        let name: String = c[name_at..name_end].iter().collect();
        match signature_end(&c, name_end) {
            Some(Sig::Body(open)) => {
                let close = match_brace(&c, open);
                let last = line_of[close.saturating_sub(1).min(c.len() - 1)];
                out.push(FnSpan {
                    name,
                    line: line_of[i],
                    len: last - line_of[i] + 1,
                });
                i = open + 1;
            }
            Some(Sig::Declaration(end)) => i = end,
            None => i = name_end,
        }
    }
    out
}

/// 各文字の行（1 始まり）。
fn line_table(c: &[char]) -> Vec<usize> {
    let mut line = 1;
    c.iter()
        .map(|&ch| {
            let here = line;
            if ch == '\n' {
                line += 1;
            }
            here
        })
        .collect()
}

/// 識別子に使える文字か。
fn is_ident(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn ident_end(c: &[char], mut i: usize) -> usize {
    while i < c.len() && is_ident(c[i]) {
        i += 1;
    }
    i
}

/// `i` が単語としての `fn` の先頭なら、続く関数名の先頭位置（`fn(` の型や `Fn` は除く）。
fn fn_keyword_name_start(c: &[char], i: usize) -> Option<usize> {
    if c[i] != 'f' || c.get(i + 1) != Some(&'n') || (i > 0 && is_ident(c[i - 1])) {
        return None;
    }
    let mut j = i + 2;
    if !c.get(j).is_some_and(|ch| ch.is_whitespace()) {
        return None;
    }
    while c.get(j).is_some_and(|ch| ch.is_whitespace()) {
        j += 1;
    }
    c.get(j)
        .is_some_and(|&ch| ch.is_alphabetic() || ch == '_')
        .then_some(j)
}

/// 関数の署名の終わり。
enum Sig {
    /// 本体の `{` の位置。
    Body(usize),
    /// `;` で終わる宣言（trait のメソッドなど）の直後。
    Declaration(usize),
}

/// 関数名の後から、本体の `{` か宣言の `;` を探す（引数の `()`・`[]` の中は見ない）。
fn signature_end(c: &[char], mut i: usize) -> Option<Sig> {
    let mut depth = 0usize;
    while i < c.len() {
        if let Some(end) = lexeme_end(c, i) {
            i = end;
            continue;
        }
        match c[i] {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            '{' if depth == 0 => return Some(Sig::Body(i)),
            ';' if depth == 0 => return Some(Sig::Declaration(i + 1)),
            _ => {}
        }
        i += 1;
    }
    None
}

/// `i` にコメント・文字列・raw 文字列・文字リテラルが始まっていれば、その直後の位置（ライフタイム `'a` は対象外）。
fn lexeme_end(c: &[char], i: usize) -> Option<usize> {
    match c[i] {
        '/' if c.get(i + 1) == Some(&'/') => Some(
            c[i..]
                .iter()
                .position(|&ch| ch == '\n')
                .map_or(c.len(), |p| i + p),
        ),
        '/' if c.get(i + 1) == Some(&'*') => Some(block_comment_end(c, i)),
        '"' => Some(string_end(c, i + 1)),
        'r' | 'b' if i == 0 || !is_ident(c[i - 1]) => raw_string_end(c, i),
        '\'' => char_literal_end(c, i),
        _ => None,
    }
}

/// ネストするブロックコメントの直後。
fn block_comment_end(c: &[char], start: usize) -> usize {
    let (mut depth, mut i) = (0usize, start);
    while i < c.len() {
        if c[i] == '/' && c.get(i + 1) == Some(&'*') {
            depth += 1;
            i += 2;
        } else if c[i] == '*' && c.get(i + 1) == Some(&'/') {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return i;
            }
        } else {
            i += 1;
        }
    }
    c.len()
}

/// 通常の文字列の直後（`i` は開きの `"` の次）。
fn string_end(c: &[char], mut i: usize) -> usize {
    while i < c.len() {
        match c[i] {
            '\\' => i += 2,
            '"' => return i + 1,
            _ => i += 1,
        }
    }
    c.len()
}

/// raw 文字列（`r"…"`・`r#"…"#`・`br"…"`）の直後。raw 文字列でなければ None。
fn raw_string_end(c: &[char], i: usize) -> Option<usize> {
    let mut j = i;
    if c[j] == 'b' {
        j += 1;
    }
    if c.get(j) != Some(&'r') {
        return None;
    }
    j += 1;
    let hashes = c[j..].iter().take_while(|&&ch| ch == '#').count();
    j += hashes;
    if c.get(j) != Some(&'"') {
        return None;
    }
    j += 1;
    while j < c.len() {
        if c[j] == '"' && c[j + 1..].iter().take_while(|&&ch| ch == '#').count() >= hashes {
            return Some(j + 1 + hashes);
        }
        j += 1;
    }
    Some(c.len())
}

/// 文字リテラル（`'x'`・`'\n'`・`'\u{1F}'`）の直後。ライフタイムなら None。
fn char_literal_end(c: &[char], i: usize) -> Option<usize> {
    if c.get(i + 1) == Some(&'\\') {
        let close = c.get(i + 3..)?.iter().position(|&ch| ch == '\'')?;
        return Some(i + 3 + close + 1);
    }
    (c.get(i + 2) == Some(&'\'')).then_some(i + 3)
}

/// `{` の位置から、対応する `}` の直後（見つからなければ末尾）。
fn match_brace(c: &[char], open: usize) -> usize {
    let (mut depth, mut i) = (0usize, open);
    while i < c.len() {
        if let Some(end) = lexeme_end(c, i) {
            i = end;
            continue;
        }
        match c[i] {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    c.len()
}

/// `#[…]` の `[` から対応する `]` の直後。
fn attr_end(c: &[char], hash: usize) -> Option<usize> {
    let mut i = hash + 1;
    while c.get(i).is_some_and(|ch| ch.is_whitespace()) {
        i += 1;
    }
    if c.get(i) != Some(&'[') {
        return None;
    }
    let mut depth = 0usize;
    while i < c.len() {
        if let Some(end) = lexeme_end(c, i) {
            i = end;
            continue;
        }
        match c[i] {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// `i` の `#` が `#[cfg(test)]`（空白は無視）なら、その直後。
fn cfg_test_attr_end(c: &[char], i: usize) -> Option<usize> {
    let end = attr_end(c, i)?;
    let body: String = c[i + 1..end]
        .iter()
        .filter(|ch| !ch.is_whitespace())
        .collect();
    (body == "[cfg(test)]").then_some(end)
}

/// 属性の直後から、その属性が付いた項目の終わり。後続の属性・コメントも項目の一部として含める。
fn item_end(c: &[char], mut i: usize) -> usize {
    loop {
        while c.get(i).is_some_and(|ch| ch.is_whitespace()) {
            i += 1;
        }
        let comment = c.get(i) == Some(&'/') && matches!(c.get(i + 1), Some('/' | '*'));
        if comment && let Some(end) = lexeme_end(c, i) {
            i = end;
        } else if c.get(i) == Some(&'#')
            && let Some(end) = attr_end(c, i)
        {
            i = end;
        } else {
            break;
        }
    }
    let mut depth = 0usize;
    while i < c.len() {
        if let Some(end) = lexeme_end(c, i) {
            i = end;
            continue;
        }
        match c[i] {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            ';' if depth == 0 => return i + 1,
            '{' if depth == 0 => return after_trailing_semicolon(c, match_brace(c, i)),
            _ => {}
        }
        i += 1;
    }
    c.len()
}

/// `static X: T = T { … };` のように `}` の直後にある `;` まで項目に含める。
fn after_trailing_semicolon(c: &[char], close: usize) -> usize {
    let mut i = close;
    while matches!(c.get(i), Some(' ' | '\t')) {
        i += 1;
    }
    if c.get(i) == Some(&';') { i + 1 } else { close }
}

#[cfg(test)]
mod tests;
