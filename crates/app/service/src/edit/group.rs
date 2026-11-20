//! 打った置き先（`fune part mv RW_X aocs/RW`・`fune part add --group aocs/RW`）→ Op の `to` / `group` と親の連鎖。
//! **置き先の文字列を ID の列に読むのはここだけ**（GUI は最初から構造で送るので、Web は文字列を割らない）。

use super::text::invalid;
use crate::error::Result;

/// 置き先の文字列 → (置き先の ID, その祖先の ID の列)。`aocs/RW` → (`RW`, [`aocs`])。`/` が無ければ従来どおり 1 つの ID。
pub fn group_target(text: &str) -> Result<(String, Vec<String>)> {
    let mut ids: Vec<String> = text.split('/').map(|s| s.trim().to_string()).collect();
    if ids.iter().any(String::is_empty) {
        return Err(invalid(format!(
            "Invalid group path `{text}` (write IDs separated by `/`, e.g. `aocs/RW`)"
        )));
    }
    let to = ids.pop().unwrap_or_default();
    Ok((to, ids))
}

#[cfg(test)]
mod tests {
    use super::group_target;

    #[test]
    fn splits_into_target_and_parents() {
        assert_eq!(group_target("aocs").unwrap(), ("aocs".into(), vec![]));
        assert_eq!(
            group_target("aocs/RW").unwrap(),
            ("RW".into(), vec!["aocs".to_string()])
        );
        assert_eq!(
            group_target("aocs/RW/pair").unwrap(),
            ("pair".into(), vec!["aocs".to_string(), "RW".to_string()])
        );
    }

    #[test]
    fn rejects_empty_segments() {
        for bad in ["", "aocs/", "/RW", "aocs//RW"] {
            assert!(group_target(bad).is_err(), "{bad}");
        }
    }
}
