//! 表示名と ID の組み方（**この規則はここだけ**）。
//!
//! 設計に書かれた表示名（`label`）が無い・空・空白だけ・ID と同じのとき、表示名は無いものとして ID だけを出す。
//! Web の `idWithName`（`web/src/lib/design.ts`）も同じ規則に従う（両方が同じ表で合うことは
//! `testdata/display_name_cases.json` を読むテストで確かめる）。

/// 表示名が実際に使えるか（空・空白だけ・ID と同じなら使えない）。
fn usable<'a>(label: Option<&'a str>, id: &str) -> Option<&'a str> {
    label.filter(|l| !l.trim().is_empty() && *l != id)
}

/// 表示名（無い・空・空白だけなら ID）。見出しや列名に使う。
pub fn name_or_id(label: Option<&str>, id: &str) -> String {
    label
        .filter(|l| !l.trim().is_empty())
        .unwrap_or(id)
        .to_owned()
}

/// `<表示名> (<ID>)`。表示名が使えない（無い・空・空白だけ・ID と同じ）ときは ID だけ。
pub fn name_with_id(label: Option<&str>, id: &str) -> String {
    match usable(label, id) {
        Some(label) => format!("{label} ({id})"),
        None => id.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    /// Web と共有する表（`web/src/lib/design.test.ts` も同じファイルを読む）。
    #[derive(Deserialize)]
    struct Case {
        id: String,
        label: Option<String>,
        shows_name: bool,
    }

    #[test]
    fn follows_the_shared_table() {
        let cases: Vec<Case> =
            serde_json::from_str(include_str!("../testdata/display_name_cases.json")).unwrap();
        assert!(cases.len() >= 5);
        for c in cases {
            let label = c.label.as_deref();
            let with_id = name_with_id(label, &c.id);
            if c.shows_name {
                let l = c.label.clone().unwrap();
                assert_eq!(with_id, format!("{l} ({})", c.id));
                assert_eq!(name_or_id(label, &c.id), l);
            } else {
                assert_eq!(with_id, c.id, "{:?}", c.label);
            }
        }
    }

    #[test]
    fn name_or_id_falls_back_for_blank_labels() {
        assert_eq!(name_or_id(Some(""), "A"), "A");
        assert_eq!(name_or_id(Some("  "), "A"), "A");
        assert_eq!(name_or_id(None, "A"), "A");
        assert_eq!(name_or_id(Some("Alpha"), "A"), "Alpha");
    }
}
