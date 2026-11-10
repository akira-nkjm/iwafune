//! 「1 つの機能は 1 か所だけ」の機械的な検査（docs/architecture/registry.md §2 の一部を強制する）。
//!
//! 定義の書き出し（`fn rk4` や定数の値など）が、台帳で決めたファイル以外に現れたら違反にする。
//! 使う側（呼び出し）は検査しない。台帳に行を足したら、規則の表（[`rules`]）にも規則を足す。

use std::path::Path;

mod rules;
use rules::{PROD_RULES, RULES, WEB_RULES};

/// 色を直書きしてよい web のファイル（配色の表とそのテスト）。
const HEX_COLOR_ALLOWED: &[&str] = &["web/src/lib/tone.ts", "web/src/lib/tone.test.ts"];

/// 違反を `errors` に積む。`files` はワークスペースのファイル一覧。
pub fn check(root: &Path, files: &[std::path::PathBuf], errors: &mut Vec<String>) {
    for file in files {
        let rel = file.strip_prefix(root).unwrap_or(file);
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        if rel_s.starts_with("web/src/") && rel_s.ends_with(".css") {
            check_css_colors(&rel_s, file, errors);
            continue;
        }
        let is_code = rel_s.ends_with(".rs") || rel_s.ends_with(".ts") || rel_s.ends_with(".tsx");
        if !is_code || rel_s.starts_with("crates/xtask/") || rel_s.contains("/gen/") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        let web = rel_s.starts_with("web/");
        let rules = if web {
            RULES.iter().chain(WEB_RULES)
        } else {
            RULES.iter().chain(&[][..])
        };
        for rule in rules {
            check_rule(&rel_s, &text, rule, errors);
        }
        if let Some(prod) = production_text(&rel_s, &text) {
            for rule in PROD_RULES {
                check_rule(&rel_s, prod, rule, errors);
            }
        }
        if web
            && !rel_s.contains(".test.")
            && !HEX_COLOR_ALLOWED.contains(&rel_s.as_str())
            && let Some(color) = find_hex_color(&text)
        {
            errors.push(format!(
                "{rel_s}: 色 `{color}` の直書きがあります。配色は {} だけに置きます（registry.md）",
                HEX_COLOR_ALLOWED[0]
            ));
        }
    }
}

/// CSS の色: 値は lib/tone.ts が正本で、CSS に書いてよいのは書き出した区間（`colors:begin` 〜 `colors:end`）だけ。
/// 区間の外に hex や rgb() / rgba() の色があれば違反にする。
fn check_css_colors(rel: &str, file: &Path, errors: &mut Vec<String>) {
    let Ok(text) = std::fs::read_to_string(file) else {
        return;
    };
    let mut inside = false;
    for (n, line) in text.lines().enumerate() {
        if line.contains("colors:begin") {
            inside = true;
            continue;
        }
        if line.contains("colors:end") {
            inside = false;
            continue;
        }
        if inside {
            continue;
        }
        let rgb = line.contains("rgb(") || line.contains("rgba(");
        if let Some(color) = css_hex(line).or(rgb.then_some("rgb()")) {
            errors.push(format!(
                "{rel}:{}: 色 `{color}` の直書きがあります。色の値は {} に書き、pnpm -C web tokens で書き出します",
                n + 1,
                HEX_COLOR_ALLOWED[0]
            ));
        }
    }
}

fn check_rule(
    rel: &str,
    text: &str,
    (needle, allowed, what): &(&str, &[&str], &str),
    errors: &mut Vec<String>,
) {
    if text.contains(needle) && !allowed.iter().any(|a| is_allowed(rel, a)) {
        errors.push(format!(
            "{rel}: `{needle}` が見つかりました。{what}は {} だけに置きます（registry.md）",
            allowed.join(", ")
        ));
    }
}

/// テスト以外の部分（テストのファイルは None、それ以外は最初の `#[cfg(test)]` より前）。
fn production_text<'a>(rel: &str, text: &'a str) -> Option<&'a str> {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    let test_file = rel.contains("/tests/")
        || name == "tests.rs"
        || name.ends_with("_tests.rs")
        || name == "testkit.rs"
        || name.contains(".test.");
    if test_file {
        return None;
    }
    Some(text.split("#[cfg(test)]").next().unwrap_or(text))
}

/// 引用符で囲んだ `#rrggbb` か `#rgb`（CSS の色）を探す。
/// CSS の宣言の値に書いた hex の色（`#abc`・`#aabbcc` など。セレクタの `#id` は宣言の `:` より前なので見ない）
fn css_hex(line: &str) -> Option<&str> {
    let value = &line[line.find(':')? + 1..];
    for (i, _) in value.match_indices('#') {
        let digits = value[i + 1..]
            .bytes()
            .take_while(u8::is_ascii_hexdigit)
            .count();
        let after = value[i + 1 + digits..].bytes().next();
        let ends = after.is_none_or(|b| !b.is_ascii_alphanumeric() && b != b'_' && b != b'-');
        if matches!(digits, 3 | 4 | 6 | 8) && ends {
            return Some(&value[i..i + 1 + digits]);
        }
    }
    None
}

fn find_hex_color(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    for (i, _) in text.match_indices("\"#").chain(text.match_indices("'#")) {
        let start = i + 1;
        let digits = bytes[start + 1..]
            .iter()
            .take_while(|b| b.is_ascii_hexdigit())
            .count();
        let close = bytes.get(start + 1 + digits).copied();
        if (digits == 3 || digits == 6) && matches!(close, Some(b'"' | b'\'')) {
            return text.get(start..start + 1 + digits);
        }
    }
    None
}

/// `allowed` がファイル（`foo.rs`）なら、そのサブモジュールのディレクトリ（`foo/`）も許す。
fn is_allowed(path: &str, allowed: &str) -> bool {
    if path == allowed || path.starts_with(allowed) {
        return true;
    }
    allowed
        .strip_suffix(".rs")
        .is_some_and(|stem| path.starts_with(&format!("{stem}/")))
}

#[cfg(test)]
mod tests;
