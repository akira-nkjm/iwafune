//! TOML プロファイル（docs/model.md §10.2）に沿わない書き方の検出。形式の問題なので store が報告する
//! （意味の検査は `fune_model::check`）。どれも `fune fmt` で直せる。

use super::spans::LineIndex;
use crate::profile::integer_keys;
use fune_model::Severity;
use toml_edit::{Item, Table, Value};

/// 見つかった書き方の問題（重さ・コード・行・本文）。
pub type ProfileFinding = (Severity, &'static str, u32, String);

/// 走査の文脈（本文・行の索引・整数のままでよいキー）。
struct Walk<'a> {
    raw: &'a str,
    lines: &'a LineIndex,
    ints: &'a [&'a str],
}

/// 文書の書き方を検査する（`path` で整数のままでよいキーが変わる）。
pub fn findings(path: &str, doc: &toml_edit::Document<String>) -> Vec<ProfileFinding> {
    let lines = LineIndex::new(doc.raw());
    let walk = Walk {
        raw: doc.raw(),
        lines: &lines,
        ints: integer_keys(path),
    };
    let mut out = Vec::new();
    walk_table(doc.as_table(), &walk, &mut out);
    out
}

fn walk_table(table: &Table, w: &Walk, out: &mut Vec<ProfileFinding>) {
    for (key, item) in table.iter() {
        match item {
            Item::Table(t) => walk_table(t, w, out),
            Item::ArrayOfTables(a) => a.iter().for_each(|t| walk_table(t, w, out)),
            Item::Value(v) => walk_value(key, v, w, out),
            Item::None => {}
        }
    }
}

fn walk_value(key: &str, value: &Value, w: &Walk, out: &mut Vec<ProfileFinding>) {
    let line = |v: &Value| v.span().map_or(0, |s| w.lines.position(s.start).0);
    match value {
        Value::Integer(_) if !w.ints.contains(&key) => out.push((
            Severity::Info,
            "toml.integer",
            line(value),
            format!("`{key}` は物理量なので float で書きます（`0` ではなく `0.0`）"),
        )),
        Value::InlineTable(t) => {
            let multiline = t
                .span()
                .and_then(|s| w.raw.get(s))
                .is_some_and(|text| text.contains('\n'));
            if multiline {
                out.push((
                    Severity::Warning,
                    "toml.v11",
                    line(value),
                    format!(
                        "`{key}` の複数行のインライン表は TOML 1.1 の書き方です（Python 3.14 以前の tomllib では読めません）"
                    ),
                ));
            }
            for (k, v) in t.iter() {
                walk_value(k, v, w, out);
            }
        }
        Value::Array(a) => a.iter().for_each(|v| walk_value(key, v, w, out)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_integers_and_multiline_inline_tables() {
        let text = "count = 2\nmass = { value = 1, unit = \"kg\" }\nsize = {\n  x = 1.0,\n  y = 2.0,\n  z = 3.0,\n}\n";
        let doc = toml_edit::Document::parse(text.to_string()).unwrap();
        let codes: Vec<(&str, u32)> = findings("parts/A.toml", &doc)
            .iter()
            .map(|f| (f.1, f.2))
            .collect();
        assert_eq!(codes, vec![("toml.integer", 2), ("toml.v11", 3)]);
        let view = "[nodes]\nRW_X = { x = 420, y = 120 }\n\n[edge]\n\"P5V:PCDU.OUT2>CAM.PWR\" = { points = [[408, 507], [468, 507]] }\n";
        let doc = toml_edit::Document::parse(view.to_string()).unwrap();
        assert!(findings("view/electrical.toml", &doc).is_empty());
    }
}
