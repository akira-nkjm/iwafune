//! 言語で変わる値の表示（日本語と英語）。カードの行（`rows`・`parts`）が使う葉のモジュール。

/// 言語で変わる値の表示（英語が日本語と同じなら `en` は None）。
pub(super) struct Bilingual {
    pub ja: String,
    pub en: Option<String>,
}

impl Bilingual {
    /// 日本語と英語の表示（同じなら英語は持たない）。
    pub(super) fn of(ja: String, en: String) -> Self {
        let en = (en != ja).then_some(en);
        Self { ja, en }
    }

    /// 言語で変わらない表示。
    pub(super) fn same(text: String) -> Self {
        Self { ja: text, en: None }
    }

    /// 範囲の表示（日本語は「〜」、英語は「–」。値の文字列は `sat_units::format` で作ったもの）。
    pub(super) fn range(lo: &str, hi: &str) -> Self {
        Self::of(format!("{lo}〜{hi}"), format!("{lo} – {hi}"))
    }
}
