//! 名前の変更の Op を、適用前の診断に写す。**名前の変更の Op の種類を並べるのはここだけ**（`rename_of`）。
//!
//! 適用前後の診断を比べて新しい error を拒む（`plan::new_errors`）とき、名前を変えたエンティティに元から error があると、
//! 診断の対象と文の中の ID が新しい名前になるので、同じ error が「新しく増えた」ように見えて名前の変更が拒まれる。
//! そこで適用前の診断の ID を、名前の変更の Op の列に沿って新しい名前に替えてから比べる。

use crate::op::Op;
use fune_model::Diagnostic;

/// 名前の変更の Op なら、`(変更前, 変更後)`。
pub(super) fn rename_of(op: &Op) -> Option<(&str, &str)> {
    match op {
        Op::RenamePart { id, to }
        | Op::RenameGroup { id, to }
        | Op::RenameAttribute { id, to }
        | Op::RenameKind { id, to }
        | Op::RenameMode { id, to }
        | Op::RenameGroundStation { id, to }
        | Op::RenamePhase { id, to }
        | Op::RenameNet { id, to }
        | Op::RenameHarness { id, to }
        | Op::RenameCase { id, to }
        | Op::RenameSource { id, to }
        | Op::RenameManeuver { id, to }
        | Op::RenameCommLink { id, to } => Some((id, to)),
        Op::RenameCatalogItem { from, to } => Some((from, to)),
        _ => None,
    }
}

/// 診断の比較に使う印（コード・対象の表示用のパス・文）。
pub(super) type Key = (String, Option<String>, String);

/// 診断の印。
pub(super) fn key(d: &Diagnostic) -> Key {
    (d.code.clone(), d.path(), d.message.clone())
}

/// 適用前の診断の印を、名前の変更 `renames`（Op の順）で新しい名前に替える。
pub(super) fn through_renames(d: &Diagnostic, renames: &[(&str, &str)]) -> Key {
    let (code, path, message) = key(d);
    let map = |text: String| {
        renames
            .iter()
            .fold(text, |t, (from, to)| replace_id(&t, from, to))
    };
    (code, path.map(map), map(message))
}

/// ID に使う文字（英数字・`_`・`-`）。これに続く・先立つ文字の中には ID の一部を置き換えない。
fn is_id_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

/// `text` の中の、ID として独立している `from` を `to` に替える（`RW` は `RW_X` の一部には当たらない）。
fn replace_id(text: &str, from: &str, to: &str) -> String {
    if from.is_empty() {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(from) {
        let (head, tail) = rest.split_at(at);
        let after = &tail[from.len()..];
        out.push_str(head);
        let bounded_before = !out.chars().next_back().is_some_and(is_id_char);
        let bounded_after = !after.chars().next().is_some_and(is_id_char);
        out.push_str(if bounded_before && bounded_after {
            to
        } else {
            from
        });
        rest = after;
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_only_whole_ids() {
        assert_eq!(replace_id("RW_X.mass", "RW_X", "RW_Y"), "RW_Y.mass");
        assert_eq!(
            replace_id("part `RW` and RW_X", "RW", "W"),
            "part `W` and RW_X"
        );
        assert_eq!(replace_id("A A.b xA", "A", "B"), "B B.b xA");
        assert_eq!(replace_id("AA A", "A", "B"), "AA B");
        assert_eq!(replace_id("abc", "", "x"), "abc");
    }

    /// 名前の変更の Op はどれも `rename_of` が拾う（Op の名前が `Rename` で始まるものを 1 つずつ作って確かめる）。
    #[test]
    fn every_rename_op_is_mapped() {
        let pair = |id: &str, to: &str| (id.to_owned(), to.to_owned());
        let (id, to) = pair("a", "b");
        let ops = [
            Op::RenamePart {
                id: id.clone(),
                to: to.clone(),
            },
            Op::RenameGroup {
                id: id.clone(),
                to: to.clone(),
            },
            Op::RenameAttribute {
                id: id.clone(),
                to: to.clone(),
            },
            Op::RenameKind {
                id: id.clone(),
                to: to.clone(),
            },
            Op::RenameMode {
                id: id.clone(),
                to: to.clone(),
            },
            Op::RenameGroundStation {
                id: id.clone(),
                to: to.clone(),
            },
            Op::RenamePhase {
                id: id.clone(),
                to: to.clone(),
            },
            Op::RenameNet {
                id: id.clone(),
                to: to.clone(),
            },
            Op::RenameHarness {
                id: id.clone(),
                to: to.clone(),
            },
            Op::RenameCase {
                id: id.clone(),
                to: to.clone(),
            },
            Op::RenameSource {
                id: id.clone(),
                to: to.clone(),
            },
            Op::RenameManeuver {
                id: id.clone(),
                to: to.clone(),
            },
            Op::RenameCommLink {
                id: id.clone(),
                to: to.clone(),
            },
            Op::RenameCatalogItem { from: id, to },
        ];
        for op in &ops {
            assert_eq!(rename_of(op), Some(("a", "b")), "{op:?}");
        }
        // Op の定義に `Rename*` が増えたら、この数と `rename_of` の両方を直す
        let source = include_str!("../op.rs");
        let declared = source
            .lines()
            .filter(|l| l.starts_with("    Rename"))
            .count();
        assert_eq!(declared, ops.len());
    }

    #[test]
    fn rename_ops_give_their_pair() {
        let op = Op::RenameCatalogItem {
            from: "local/a".into(),
            to: "local/b".into(),
        };
        assert_eq!(rename_of(&op), Some(("local/a", "local/b")));
        assert_eq!(rename_of(&Op::RemovePart { id: "x".into() }), None);
    }
}
