//! 型付き ID と ID の規則（docs/model.md §4・§10.2）。
//!
//! ID はファイル名（またはディレクトリ名）の stem と一致し、全体で一意。比較は大文字小文字を区別するが、
//! 一意性の検査（[`same_id`]・[`clash`]・[`duplicates`]）は大文字小文字を無視する。JSON・TS では文字列。

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use smol_str::SmolStr;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// 型付き ID。`T` は指す先のエンティティの型（`Id<Part>` など）。
pub struct Id<T> {
    name: SmolStr,
    _t: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// ID を作る（規則の検査はしない。検査は [`validate_entity_id`] と `check`）。
    pub fn new(name: impl AsRef<str>) -> Self {
        Self {
            name: SmolStr::new(name.as_ref()),
            _t: PhantomData,
        }
    }

    /// 文字列としての ID。
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// 別の種類の ID に付け替える（Group と Part のように同じ名前空間を共有するとき）。
    pub fn cast<U>(&self) -> Id<U> {
        Id::new(self.as_str())
    }
}

impl<T> Default for Id<T> {
    fn default() -> Self {
        Self::new("")
    }
}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self::new(self.as_str())
    }
}
impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl<T> Eq for Id<T> {}
impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}
impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
impl<T> Borrow<str> for Id<T> {
    fn borrow(&self) -> &str {
        &self.name
    }
}
impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.name.as_str())
    }
}
impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}
impl<T> From<&str> for Id<T> {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}
impl<T> From<String> for Id<T> {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
impl<T> Serialize for Id<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.name)
    }
}
impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::new)
    }
}

/// TS では string。
impl<T: 'static> ts_rs::TS for Id<T> {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(_: &ts_rs::Config) -> String {
        String::from("string")
    }
    fn inline(cfg: &ts_rs::Config) -> String {
        <Self as ts_rs::TS>::name(cfg)
    }
}

/// 一意性の検査に使う正規形（大文字小文字を無視）。比べるのは [`same_id`] を通す。
fn fold(id: &str) -> String {
    id.to_ascii_lowercase()
}

/// 大文字小文字を無視して同じ ID か。**ID の一意性の比べ方はここだけ**（大文字小文字を区別しないファイルシステムでも
/// ファイルが重ならないように）。部品とグループ（同じ名前空間）・モード・ネット・ハーネス・解析ケース・出典・
/// カタログ品（出所ごと）に同じ規則を当てる（検査は [`duplicates`]、新しい ID の事前の確かめは [`clash`]）。
pub fn same_id(a: &str, b: &str) -> bool {
    fold(a) == fold(b)
}

/// `existing` のうち、`id` と大文字小文字を無視して同じもの（新しく作る・名前を変える先の ID の確かめ）。
pub fn clash<'a>(existing: impl IntoIterator<Item = &'a str>, id: &str) -> Option<&'a str> {
    existing.into_iter().find(|e| same_id(e, id))
}

/// 一覧の中で、先に出てきた ID と大文字小文字を無視して重なるもの（重なった ID の番号, 先に出てきた ID の番号）。検査が使う。
pub fn duplicates<'a>(ids: impl IntoIterator<Item = &'a str>) -> Vec<(usize, usize)> {
    let mut seen: Vec<(usize, &'a str)> = Vec::new();
    let mut out = Vec::new();
    for (i, id) in ids.into_iter().enumerate() {
        match seen.iter().find(|(_, s)| same_id(s, id)) {
            Some((first, _)) => out.push((i, *first)),
            None => seen.push((i, id)),
        }
    }
    out
}

/// パスの先頭に来る予約語（部品 ID に使えない）。`mission.orbit.altitude` などと区別するため。
pub const RESERVED_ROOTS: &[&str] = &["mission", "analysis", "budget", "modes", "nets"];

const WINDOWS_RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// ID の規則に反した理由。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdError {
    #[error("ID が空です")]
    Empty,
    #[error("ID `{0}` は `_` で始められません")]
    LeadingUnderscore(String),
    #[error("ID `{0}` に使えない文字があります（英数字・`_`・`-` だけ。先頭は英字）")]
    InvalidChar(String),
    #[error("ID `{0}` は Windows の予約名です")]
    WindowsReserved(String),
    #[error("ID `{0}` は予約語です")]
    Reserved(String),
}

/// エンティティ（部品・グループ・モード・ネットなど）の ID の規則を検査する。
/// 英字で始まり、英数字・`_`・`-` だけ。`.` はパス記法の区切りなので使えない。
pub fn validate_entity_id(id: &str) -> Result<(), IdError> {
    let first = id.chars().next().ok_or(IdError::Empty)?;
    if first == '_' {
        return Err(IdError::LeadingUnderscore(id.to_string()));
    }
    let ok_rest = id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !first.is_ascii_alphabetic() || !ok_rest {
        return Err(IdError::InvalidChar(id.to_string()));
    }
    if WINDOWS_RESERVED.contains(&fold(id).as_str()) {
        return Err(IdError::WindowsReserved(id.to_string()));
    }
    Ok(())
}

/// 製品ツリーのノード（部品・グループ）の ID の規則。エンティティの規則に加え、パスの先頭の予約語（[`RESERVED_ROOTS`]）を使えない。
pub fn validate_node_id(id: &str) -> Result<(), IdError> {
    validate_entity_id(id)?;
    if RESERVED_ROOTS.contains(&fold(id).as_str()) {
        return Err(IdError::Reserved(id.to_string()));
    }
    Ok(())
}

/// 局所的な名前（属性 ID・状態・ポート・取付点）の規則。英字で始まり、英数字と `_` だけ。
pub fn validate_local_name(name: &str) -> Result<(), IdError> {
    let first = name.chars().next().ok_or(IdError::Empty)?;
    let ok_rest = name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !first.is_ascii_alphabetic() || !ok_rest {
        return Err(IdError::InvalidChar(name.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_ids_follow_the_rules() {
        assert!(validate_entity_id("RW_X").is_ok());
        assert!(validate_entity_id("P12V-AOCS").is_ok());
        assert!(validate_entity_id("_hidden").is_err());
        assert!(validate_entity_id("RW.X").is_err());
        assert!(validate_entity_id("con").is_err());
        assert!(validate_entity_id("MISSION").is_ok());
        assert!(validate_node_id("Mission").is_err());
        assert!(validate_entity_id("").is_err());
    }

    #[test]
    fn uniqueness_ignores_case() {
        assert!(same_id("RW_X", "rw_x"));
        assert!(!same_id("RW_X", "RW_Y"));
        assert_eq!(clash(["SAFE", "NOMINAL"], "safe"), Some("SAFE"));
        assert_eq!(clash(["SAFE"], "MISSION"), None);
        assert_eq!(duplicates(["hot", "cold", "Hot", "HOT"]), [(2, 0), (3, 0)]);
        assert!(duplicates(["a", "b"]).is_empty());
    }

    #[test]
    fn ids_borrow_as_str_for_map_lookup() {
        let mut map = std::collections::BTreeMap::<Id<()>, u32>::new();
        map.insert(Id::new("RW_X"), 1);
        assert_eq!(map.get("RW_X"), Some(&1));
    }
}
