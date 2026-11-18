//! 電源のモードの並びの名前と選び方（`analysis/power.toml` の `[[sequence]]`・`[sequences.<名前>]`・`active_sequence`）。
//! **どの並びを使うか・名前の一覧はここだけ**（解析への写しも GUI の表示もこれを引く）。
//!
//! 従来の名前の無い `[[sequence]]` は、そのまま読めて `default` という名前の並びとして扱う（基準は持てない）。
//! 並びの編集（`SetSequence`）は `[sequences.<名前>]` に書き、同じ `default` を書くと従来の `[[sequence]]` は消える（移行）。

use super::analysis::{ModeSegment, PowerSettings, SequenceSetting};
use super::timing::StartRef;

/// 名前の無い `[[sequence]]` を指す名前。
pub const DEFAULT_SEQUENCE: &str = "default";

/// 名前で引いた並び 1 つ。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SequenceView<'a> {
    pub name: &'a str,
    pub anchor: Option<&'a StartRef>,
    pub segments: &'a [ModeSegment],
    /// 従来の `[[sequence]]` の形で書かれているか
    pub legacy: bool,
}

impl PowerSettings {
    /// 並びの名前（`default` の並びがあれば最初、残りは名前の順）。
    pub fn sequence_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = Vec::new();
        if !self.sequence.is_empty() && !self.sequences.contains_key(DEFAULT_SEQUENCE) {
            names.push(DEFAULT_SEQUENCE);
        }
        names.extend(self.sequences.keys().map(String::as_str));
        names.sort_unstable_by_key(|n| (*n != DEFAULT_SEQUENCE, *n));
        names
    }

    /// 名前で並びを引く（`[sequences.<名前>]` を優先し、`default` だけ従来の `[[sequence]]` に戻る）。
    pub fn sequence(&self, name: &str) -> Option<SequenceView<'_>> {
        if let Some((n, s)) = self.sequences.get_key_value(name) {
            return Some(view(n, s));
        }
        (name == DEFAULT_SEQUENCE && !self.sequence.is_empty()).then(|| SequenceView {
            name: DEFAULT_SEQUENCE,
            anchor: None,
            segments: &self.sequence,
            legacy: true,
        })
    }

    /// 解析に使う並びの名前。`active_sequence`、無ければ `default`、それも無ければ名前の順で最初の並び。
    /// `active_sequence` が存在しない名前のときは None（検査が誤りにする。黙って別の並びを使わない）。
    pub fn active_sequence_name(&self) -> Option<&str> {
        match &self.active_sequence {
            Some(n) => self.sequence(n).map(|v| v.name),
            None => self.sequence_names().into_iter().next(),
        }
    }

    /// 解析に使う並び。
    pub fn active_sequence_view(&self) -> Option<SequenceView<'_>> {
        self.active_sequence_name().and_then(|n| self.sequence(n))
    }
}

fn view<'a>(name: &'a str, s: &'a SequenceSetting) -> SequenceView<'a> {
    SequenceView {
        name,
        anchor: s.anchor.as_ref(),
        segments: &s.segments,
        legacy: false,
    }
}
