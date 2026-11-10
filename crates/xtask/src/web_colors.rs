//! Web の色の直書きの検査（CONTRIBUTING.md: 色の直書きは `web/src/lib/tone.ts` だけ）。
//!
//! - TS / TSX（テストと生成物を除く）: 引用符で囲んだ hex（`"#3382c2"`）、色の名前（`white`・`black`。
//!   `bg-white` のような Tailwind のクラスや `color-mix(…, white)` の中も）、色の混ぜ方 `color-mix(`
//!   （サブシステム色の淡い地・熱の配色・覆いは tone.ts の関数で作る）。shadcn/ui から取り込んだ `ui/` の
//!   `color-mix(` は見ない。
//! - CSS（`web/src/`）: 値は tone.ts が正本で、書いてよいのは書き出した区間（`colors:begin` 〜 `colors:end`）だけ。
//!   区間の外の hex・`rgb()` / `rgba()`・色の名前を違反にする。

use std::path::Path;

/// 色を直書きしてよい web のファイル（配色の表）。テスト（`.test.`）はどれも見ない。
const COLOR_ALLOWED: &[&str] = &["web/src/lib/tone.ts"];

/// `color-mix(` を書いてよい場所（配色の表と、shadcn/ui から取り込んだプリミティブ）。
const MIX_ALLOWED: &[&str] = &["web/src/lib/tone.ts", "web/src/ui/"];

/// 直書きを見つける色の名前（カテゴリ色の名前 `blue` などは tone.ts の語彙なので見ない）。
const COLOR_NAMES: &[&str] = &["white", "black"];

/// 違反を `errors` に積む。`files` はワークスペースのファイル一覧。
pub fn check(root: &Path, files: &[std::path::PathBuf], errors: &mut Vec<String>) {
    for file in files {
        let rel = file.strip_prefix(root).unwrap_or(file);
        let rel = rel.to_string_lossy().replace('\\', "/");
        let css = rel.starts_with("web/src/") && rel.ends_with(".css");
        let code = (rel.ends_with(".ts") || rel.ends_with(".tsx"))
            && rel.starts_with("web/")
            && !rel.contains(".test.")
            && !rel.contains("/gen/");
        if !css && !code {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        if css {
            check_css(&rel, &text, errors);
        } else {
            errors.extend(code_problems(&rel, &text));
        }
    }
}

/// TS / TSX の 1 ファイルの違反の文言。
fn code_problems(rel: &str, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let tone = COLOR_ALLOWED[0];
    if !COLOR_ALLOWED.contains(&rel) {
        if let Some(color) = find_hex_color(text) {
            out.push(format!(
                "{rel}: 色 `{color}` の直書きがあります。配色は {tone} だけに置きます（registry.md）"
            ));
        }
        if let Some(name) = find_color_name(text) {
            out.push(format!(
                "{rel}: 色の名前 `{name}` の直書きがあります。地や字の色は {tone} の色（`palette` など）を使います"
            ));
        }
    }
    if text.contains("color-mix(") && !MIX_ALLOWED.iter().any(|a| rel.starts_with(a)) {
        out.push(format!(
            "{rel}: `color-mix(` が見つかりました。色の混ぜ方は {tone} の関数（`toneTint`・`heatHex` など）だけに置きます"
        ));
    }
    out
}

/// CSS の 1 ファイル。書き出した区間の外にある色を違反にする。
fn check_css(rel: &str, text: &str, errors: &mut Vec<String>) {
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
        let color = css_hex(line)
            .or(rgb.then_some("rgb()"))
            .or_else(|| find_color_name(line));
        if let Some(color) = color {
            errors.push(format!(
                "{rel}:{}: 色 `{color}` の直書きがあります。色の値は {} に書き、pnpm -C web tokens で書き出します",
                n + 1,
                COLOR_ALLOWED[0]
            ));
        }
    }
}

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

/// 引用符で囲んだ `#rrggbb` か `#rgb`（CSS の色）を探す。
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

/// 色の名前（[`COLOR_NAMES`]）を 1 語として書いた所を探す。前が英数字・`_` でなく、後ろが英数字・`_`・`-` で
/// ないものだけ（`bg-white`・`, white)` は当たり、`whitespace-nowrap`・`white-space`・`isWhite` は当たらない）。
fn find_color_name(text: &str) -> Option<&'static str> {
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    COLOR_NAMES.iter().copied().find(|name| {
        text.match_indices(name).any(|(i, _)| {
            let before = i.checked_sub(1).map(|j| text.as_bytes()[j]);
            let after = text.as_bytes().get(i + name.len()).copied();
            !before.is_some_and(word) && !after.is_some_and(|b| word(b) || b == b'-')
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_names_are_found_only_as_words() {
        assert_eq!(
            find_color_name("color-mix(in srgb, x 6%, white)"),
            Some("white")
        );
        assert_eq!(find_color_name("className=\"bg-white/50\""), Some("white"));
        assert_eq!(find_color_name("text-black"), Some("black"));
        assert_eq!(find_color_name("whitespace-nowrap"), None);
        assert_eq!(find_color_name("white-space: pre"), None);
        assert_eq!(find_color_name("const isWhite = true; blackbox"), None);
    }

    #[test]
    fn hex_colors_are_found_in_quotes_and_css_values() {
        assert_eq!(find_hex_color("const c = \"#3382c2\";"), Some("#3382c2"));
        assert_eq!(find_hex_color("'#fff'"), Some("#fff"));
        assert_eq!(find_hex_color("\"#section\" `#${hex}`"), None);
        assert_eq!(css_hex("  color: #b92626;"), Some("#b92626"));
        assert_eq!(css_hex("#root { display: grid; }"), None);
    }

    #[test]
    fn code_rules_allow_the_palette_and_ui_only() {
        let mix = "background: `color-mix(in srgb, ${a} 10%, var(--color-panel))`";
        assert_eq!(code_problems("web/src/features/x/Card.tsx", mix).len(), 1);
        assert!(code_problems("web/src/lib/tone.ts", mix).is_empty());
        assert!(code_problems("web/src/ui/button.tsx", "bg-[color-mix(in_oklch,a,b)]").is_empty());
        let named = "style={{ background: \"white\" }}";
        assert_eq!(code_problems("web/src/features/x/Card.tsx", named).len(), 1);
        assert!(code_problems("web/src/lib/tone.ts", named).is_empty());
        assert!(code_problems("web/src/kit/Td.tsx", "whitespace-nowrap").is_empty());
    }

    #[test]
    fn css_colors_are_allowed_only_in_the_generated_block() {
        let css = "@theme {\n  /* colors:begin */\n  --color-panel: #ffffff;\n  /* colors:end */\n  --x: white;\n}\n.a { color: rgb(0 0 0); }\n";
        let mut errors = Vec::new();
        check_css("web/src/styles/index.css", css, &mut errors);
        assert_eq!(errors.len(), 2);
        assert!(errors[0].contains(":5:") && errors[0].contains("white"));
        assert!(errors[1].contains(":7:") && errors[1].contains("rgb()"));
    }
}
