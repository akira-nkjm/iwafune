//! ファイル内のキー → 行・列の索引（診断の位置のため）。toml_edit の解析結果の span から作る。

use toml_edit::{Item, Key, Table, Value};

/// バイト位置 → (行, 列)（1 始まり）。
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    /// 本文の行頭の位置を数える。
    pub fn new(text: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        Self { starts }
    }

    /// バイト位置の (行, 列)。
    pub fn position(&self, offset: usize) -> (u32, u32) {
        let line = self.starts.partition_point(|&s| s <= offset).max(1);
        let column = offset - self.starts[line - 1] + 1;
        (to_u32(line), to_u32(column))
    }
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// キー（ドット区切り）と位置の列。
pub fn index(doc: &toml_edit::Document<String>) -> Vec<(String, u32, u32)> {
    let lines = LineIndex::new(doc.raw());
    let mut out = Vec::new();
    walk_table(doc.as_table(), "", &lines, &mut out);
    out
}

fn join(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{prefix}.{key}")
    }
}

fn record(out: &mut Vec<(String, u32, u32)>, lines: &LineIndex, path: &str, span: Option<usize>) {
    if let Some(offset) = span {
        let (line, column) = lines.position(offset);
        out.push((path.to_string(), line, column));
    }
}

fn key_start(key: &Key) -> Option<usize> {
    key.span().map(|s| s.start)
}

fn walk_table(table: &Table, prefix: &str, lines: &LineIndex, out: &mut Vec<(String, u32, u32)>) {
    for (name, _) in table.iter() {
        let Some((key, item)) = table.get_key_value(name) else {
            continue;
        };
        let path = join(prefix, name);
        walk_item(key, item, &path, lines, out);
    }
}

fn walk_item(
    key: &Key,
    item: &Item,
    path: &str,
    lines: &LineIndex,
    out: &mut Vec<(String, u32, u32)>,
) {
    match item {
        Item::Table(t) => {
            record(
                out,
                lines,
                path,
                t.span().map(|s| s.start).or_else(|| key_start(key)),
            );
            walk_table(t, path, lines, out);
        }
        Item::ArrayOfTables(a) => {
            record(
                out,
                lines,
                path,
                a.span().map(|s| s.start).or_else(|| key_start(key)),
            );
            for (i, t) in a.iter().enumerate() {
                let p = format!("{path}.{i}");
                record(out, lines, &p, t.span().map(|s| s.start));
                walk_table(t, &p, lines, out);
            }
        }
        Item::Value(Value::InlineTable(t)) => {
            record(out, lines, path, key_start(key));
            for (name, _) in t.iter() {
                if let Some((k, i)) = t.get_key_value(name) {
                    walk_item(k, i, &join(path, name), lines, out);
                }
            }
        }
        _ => record(out, lines, path, key_start(key)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_keys_with_lines() {
        let text = "kind = \"obc\"\n\n[attr]\nmass = { value = 0.1, unit = \"kg\" }\n\n[attr.power]\non = { value = 1.0, unit = \"W\" }\n";
        let doc = toml_edit::Document::parse(text.to_string()).unwrap();
        let idx = index(&doc);
        let line = |k: &str| idx.iter().find(|(p, _, _)| p == k).map(|e| e.1);
        assert_eq!(line("kind"), Some(1));
        assert_eq!(line("attr.mass"), Some(4));
        assert_eq!(line("attr.mass.unit"), Some(4));
        assert_eq!(line("attr.power.on"), Some(7));
    }
}
