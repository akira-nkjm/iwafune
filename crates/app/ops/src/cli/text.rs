//! コマンド文字列の組み立ての道具（引用・旗）。値の文字列の書き方は `fune_model::value`（`value_to_text` など。読み方と同じ場所）。

use fune_model::value::Provenance;

/// シェルにそのまま貼れるように引数を引用する（グロブや空白を含むものだけ `"…"` で囲む）。
pub fn quote(arg: &str) -> String {
    let bare = !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_./:=,+-@%".contains(c));
    if bare {
        return arg.to_string();
    }
    let mut out = String::from('"');
    for c in arg.chars() {
        if matches!(c, '"' | '\\' | '$' | '`') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// コマンドを組み立てる小さな道具。
pub(super) struct Cmd(Vec<String>);

impl Cmd {
    pub(super) fn new(words: &[&str]) -> Self {
        Self(words.iter().map(|w| quote(w)).collect())
    }
    pub(super) fn arg(mut self, a: &str) -> Self {
        self.0.push(quote(a));
        self
    }
    pub(super) fn flag(mut self, name: &str, value: Option<&str>) -> Self {
        match value {
            // `-` で始まる値は `--name=value` の形にする（clap が別の旗と読み違えないように）。
            Some(v) if v.starts_with('-') => self.0.push(quote(&format!("--{name}={v}"))),
            Some(v) => {
                self.0.push(format!("--{name}"));
                self.0.push(quote(v));
            }
            None => {}
        }
        self
    }

    /// 繰り返しの旗（`--port PWR=power --port DATA=data`）。
    pub(super) fn flags<'a>(
        mut self,
        name: &str,
        values: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        for v in values {
            self = self.flag(name, Some(v));
        }
        self
    }
    pub(super) fn switch(mut self, name: &str, on: bool) -> Self {
        if on {
            self.0.push(format!("--{name}"));
        }
        self
    }
    pub(super) fn from(self, from: Option<&Provenance>) -> Self {
        let source = from.and_then(|f| f.source.as_deref());
        let at = from.and_then(|f| f.at.as_deref());
        let note = from.and_then(|f| f.note.as_deref());
        self.flag("source", source)
            .flag("at", at)
            .flag("note", note)
    }
    pub(super) fn done(self) -> String {
        self.0.join(" ")
    }
}
