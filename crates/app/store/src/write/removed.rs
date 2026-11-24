//! 書き換えで消えたキーを見つける（置き換える Op が、書かれていたキーを黙って落とさないための知らせの元）。
//! 2 つの本文（書き換え前と後）を TOML として読み、前にあって後に無いキーを `target.altitude` のようなドット区切りのパスで返す
//! （表の中は表の外と同じく掘り下げ、表ごと無くなれば表のキーだけを返す。配列は 1 つの値）。読めない本文は空を返す。

use toml_edit::{DocumentMut, TableLike};

/// `before` にあって `after` に無いキー（ファイルの順）。
pub fn removed_keys(before: &str, after: &str) -> Vec<String> {
    let (Ok(b), Ok(a)) = (before.parse::<DocumentMut>(), after.parse::<DocumentMut>()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    walk(b.as_table(), Some(a.as_table()), "", &mut out);
    out
}

/// `text` にキー（ドット区切り）があるか。
pub fn has_key(text: &str, dotted: &str) -> bool {
    let Ok(doc) = text.parse::<DocumentMut>() else {
        return false;
    };
    let mut table: &dyn TableLike = doc.as_table();
    let mut parts = dotted.split('.').peekable();
    while let Some(key) = parts.next() {
        let Some(item) = table.get(key) else {
            return false;
        };
        if parts.peek().is_none() {
            return true;
        }
        match item.as_table_like() {
            Some(t) => table = t,
            None => return false,
        }
    }
    false
}

fn walk(
    before: &dyn TableLike,
    after: Option<&dyn TableLike>,
    prefix: &str,
    out: &mut Vec<String>,
) {
    for (key, item) in before.iter() {
        let path = if prefix.is_empty() {
            key.to_owned()
        } else {
            format!("{prefix}.{key}")
        };
        match (after.and_then(|a| a.get(key)), item.as_table_like()) {
            (None, _) => out.push(path),
            (Some(kept), Some(b)) => walk(b, kept.as_table_like(), &path, out),
            (Some(_), None) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_keys_that_were_written_and_are_gone() {
        let before = "label = \"x\"\nbeta = { value = 1.0, unit = \"deg\" }\norbits = 3\n\n[target]\naltitude = { value = 600.0, unit = \"km\" }\nbody = \"earth\"\n\n[transfer]\narrival = \"2029\"\n";
        let after = "label = \"x\"\n\n[target]\nbody = \"earth\"\n";
        assert_eq!(
            removed_keys(before, after),
            ["beta", "orbits", "target.altitude", "transfer"]
        );
        assert!(removed_keys(after, before).is_empty());
        assert!(removed_keys(before, before).is_empty());
        assert!(removed_keys("= = =", before).is_empty());
        assert!(has_key(before, "target.altitude"));
        assert!(has_key(before, "beta"));
        assert!(!has_key(after, "target.altitude"));
        assert!(!has_key(after, "orbits"));
    }

    /// 値が書き換わっただけ（型が変わっても）のキーは、消えたとは言わない。
    #[test]
    fn a_changed_value_is_not_a_removed_key() {
        assert!(removed_keys("a = 1\n", "a = \"x\"\n").is_empty());
        assert!(removed_keys("[t]\na = 1\n", "t = { a = 2 }\n").is_empty());
    }
}
