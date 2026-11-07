//! 変更前後の本文 → unified diff の本文（CLI の `--dry-run` と GUI の変更の確認で共通）。

use similar::TextDiff;

/// unified diff（`--- a/<path>` / `+++ b/<path>`、前後 3 行）。新規は `before = ""`、削除は `after = ""`。
pub fn unified(path: &str, before: &str, after: &str) -> String {
    TextDiff::from_lines(before, after)
        .unified_diff()
        .context_radius(3)
        .header(&format!("a/{path}"), &format!("b/{path}"))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::unified;

    #[test]
    fn one_line_change() {
        let d = unified("parts/RW_X.toml", "a = 1\nb = 2\n", "a = 1\nb = 3\n");
        assert!(d.contains("--- a/parts/RW_X.toml"));
        assert!(d.contains("-b = 2"));
        assert!(d.contains("+b = 3"));
    }
}
