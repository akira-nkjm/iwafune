//! 列挙の値の綴り（ファイル・API・CLI で同じ）の正本。
//!
//! 綴りは [`enum_text!`] の中に値ごとに 1 回だけ書き、そこから serde の名前（`#[serde(rename = …)]`）・`as_str`・
//! `Display`・`FromStr`・値の一覧（`ALL`・`NAMES`）を作る。綴りをほかの場所で書かない
//! （手書きの match や、serde_json を経由して綴りを得る書き方をしない）。

/// 綴りを持つ列挙（[`enum_text!`] が実装する）。総称の検査（すべての値の往復・表示名の有無）に使う。
pub trait EnumText: Copy + Sized + 'static {
    /// すべての値（宣言の順）。
    const ALL: &'static [Self];
    /// すべての値の綴り（[`EnumText::ALL`] と同じ順）。
    const NAMES: &'static [&'static str];
    /// 誤りの文で使う型の名前（「成熟度」など）。
    const WHAT: &'static str;
    /// 綴り（serde の名前と同じ）。
    fn as_str(self) -> &'static str;
}

/// 綴りがその列挙のどの値でもない（`FromStr` の誤り）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{text}` is not a valid {what} ({choices})")]
pub struct EnumTextError {
    /// 読めなかった文字列
    pub text: String,
    /// 型の名前（[`EnumText::WHAT`]）
    pub what: &'static str,
    /// 選べる綴り（`a / b / c`）
    pub choices: String,
}

/// 綴りから値を読む（[`enum_text!`] の `FromStr` の中身）。前後の空白は許さない（ファイルと同じ）。
pub fn parse<T: EnumText>(text: &str) -> Result<T, EnumTextError> {
    T::ALL
        .iter()
        .copied()
        .find(|v| v.as_str() == text)
        .ok_or_else(|| EnumTextError {
            text: text.to_owned(),
            what: T::WHAT,
            choices: T::NAMES.join(" / "),
        })
}

/// 値ごとの綴りを 1 回だけ書いて列挙を定義する。
///
/// ```ignore
/// enum_text! {
///     /// 成熟度。
///     #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
///     pub enum Maturity: "成熟度" {
///         OffTheShelf = "off_the_shelf",
///         Modified = "modified",
///         New = "new",
///     }
/// }
/// ```
///
/// 値に `#[serde(rename = "<綴り>")]` を付け（`rename_all` は書かない）、`ALL`・`NAMES`・`as_str`（const）、
/// [`EnumText`]、`Display`、`FromStr`（誤りは [`EnumTextError`]）を実装する。serde の derive は呼び出し側に書く。
macro_rules! enum_text {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident : $what:tt {
            $( $(#[$vmeta:meta])* $variant:ident = $text:tt ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $( $(#[$vmeta])* #[serde(rename = $text)] $variant, )+
        }

        impl $name {
            /// すべての値（宣言の順）。
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            /// すべての値の綴り（`ALL` と同じ順）。
            pub const NAMES: &'static [&'static str] = &[$($text),+];

            /// 綴り（ファイル・API・CLI で同じ。serde の名前）。
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text,)+
                }
            }
        }

        impl $crate::enum_text::EnumText for $name {
            const ALL: &'static [Self] = $name::ALL;
            const NAMES: &'static [&'static str] = $name::NAMES;
            const WHAT: &'static str = $what;
            fn as_str(self) -> &'static str {
                $name::as_str(self)
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = $crate::enum_text::EnumTextError;
            fn from_str(text: &str) -> ::std::result::Result<Self, Self::Err> {
                $crate::enum_text::parse(text)
            }
        }
    };
}

pub(crate) use enum_text;
