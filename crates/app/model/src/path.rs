//! パラメーターのパス記法（CLI と API で共通）。
//!
//! | 例 | 意味 |
//! |---|---|
//! | `RW_X.mass` | 部品 RW_X の属性 mass |
//! | `RW_X.power.on` | 状態ごとの属性 power の状態 on のセル |
//! | `RW_X.label` | 部品のフィールド（[`PartFieldName`]） |
//! | `conditions.orbit.altitude` | `conditions/orbit.toml` の `altitude` |
//! | `conditions.ground.TOKYO_GS.min_elevation` | `conditions/ground/TOKYO_GS.toml` の `min_elevation` |
//! | `analysis.power.dod_max` | `analysis/power.toml` の `dod_max` |
//! | `budget.mass.limit`・`budget.policy.growth.new` | `budget/allocation.toml`・`budget/policy.toml` |
//!
//! ポートの参照（`PCDU.OUT3`。部品 ID と、`.` を含まないポート名）は [`PortRef`]（ファイルと Op の上の文字列の形は [`port_text`]）。
//! 属性だけのパス（`power.on`）は [`AttrPath`]、カタログ品の属性（`local/rw_10mNms.power.on`）は [`parse_catalog_attr`]。
//! CLI と service はこれらを使い、自前で分割しない。

use crate::enum_text::enum_text;
use crate::field_names::PartFieldName;
use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;

pub mod port_text;

enum_text! {
    /// 設定の置き場所の種類。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
    pub enum SettingScope: "設定の置き場所" {
        Conditions = "conditions",
        Analysis = "analysis",
        Budget = "budget",
    }
}

/// 系に属さない解析の共通設定の区切り（`analysis.common.<キー>`）。綴りはここだけ。
pub const ANALYSIS_COMMON: &str = "common";

/// 設定のパス（先頭の種類と、残りの区切り）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
pub struct SettingPath {
    pub scope: SettingScope,
    pub segments: Vec<String>,
}

impl SettingPath {
    /// 種類と区切りの列から作る（`SettingPath::new(SettingScope::Conditions, &["orbit", "altitude"])`）。
    pub fn new(scope: SettingScope, segments: &[&str]) -> Self {
        Self {
            scope,
            segments: segments.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    /// `conditions.<…>`。
    pub fn conditions(segments: &[&str]) -> Self {
        Self::new(SettingScope::Conditions, segments)
    }

    /// `analysis.<…>`。
    pub fn analysis(segments: &[&str]) -> Self {
        Self::new(SettingScope::Analysis, segments)
    }

    /// 系に属さない解析の共通設定 `analysis.common`（`analysis/common.toml`）。
    pub fn analysis_common() -> Self {
        Self::analysis(&[ANALYSIS_COMMON])
    }

    /// `budget.<…>`。
    pub fn budget(segments: &[&str]) -> Self {
        Self::new(SettingScope::Budget, segments)
    }

    /// 区切りを後ろに足したパス（`analysis.thermal.case.hot` → `analysis.thermal.case.hot.beta`）。
    pub fn child(&self, segments: &[&str]) -> Self {
        let mut out = self.clone();
        out.segments
            .extend(segments.iter().map(|s| (*s).to_owned()));
        out
    }
}

impl fmt::Display for SettingPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.scope.as_str())?;
        for s in &self.segments {
            write!(f, ".{s}")?;
        }
        Ok(())
    }
}

/// 解析済みのパラメーターのパス。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(tag = "target", rename_all = "snake_case")]
pub enum ParamPath {
    Attr {
        part: String,
        attr: String,
        state: Option<String>,
    },
    Field {
        part: String,
        field: String,
    },
    Setting(SettingPath),
}

impl fmt::Display for ParamPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamPath::Attr {
                part,
                attr,
                state: Some(state),
            } => write!(f, "{part}.{attr}.{state}"),
            ParamPath::Attr { part, attr, .. } => write!(f, "{part}.{attr}"),
            ParamPath::Field { part, field } => write!(f, "{part}.{field}"),
            ParamPath::Setting(s) => s.fmt(f),
        }
    }
}

/// パスの誤り。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    #[error(
        "パス `{0}` を読めません（例: `RW_X.mass`・`RW_X.power.on`・`analysis.power.dod_max`）"
    )]
    Syntax(String),
    #[error("ポート `{0}` は `部品.ポート` の形で書きます（例: `RW_X.PWR`）")]
    Port(String),
}

/// パスを解析する。
pub fn parse(path: &str) -> Result<ParamPath, PathError> {
    let segments: Vec<&str> = path.trim().split('.').collect();
    let bad = || PathError::Syntax(path.to_string());
    if segments.iter().any(|s| s.is_empty()) || segments.len() < 2 {
        return Err(bad());
    }
    if let Ok(scope) = segments[0].parse::<SettingScope>() {
        return Ok(ParamPath::Setting(SettingPath {
            scope,
            segments: segments[1..].iter().map(|s| s.to_string()).collect(),
        }));
    }
    let part = segments[0].to_string();
    if segments.len() == 2 && segments[1].parse::<PartFieldName>().is_ok() {
        return Ok(ParamPath::Field {
            part,
            field: segments[1].to_string(),
        });
    }
    let AttrPath { attr, state } = attr_from_segments(&segments[1..]).ok_or_else(bad)?;
    Ok(ParamPath::Attr { part, attr, state })
}

/// 属性のパス（`mass`・`power.on`）。部品の中・カタログ品の中の属性（と状態ごとのセル）を指す。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
pub struct AttrPath {
    pub attr: String,
    pub state: Option<String>,
}

fn attr_from_segments(segments: &[&str]) -> Option<AttrPath> {
    match segments {
        [attr] => Some(AttrPath {
            attr: attr.to_string(),
            state: None,
        }),
        [attr, state] => Some(AttrPath {
            attr: attr.to_string(),
            state: Some(state.to_string()),
        }),
        _ => None,
    }
}

impl AttrPath {
    /// `"mass"`・`"power.on"` を解析する。
    pub fn parse(text: &str) -> Result<AttrPath, PathError> {
        let segments: Vec<&str> = text.trim().split('.').collect();
        if segments.iter().any(|s| s.is_empty()) {
            return Err(PathError::Syntax(text.to_string()));
        }
        attr_from_segments(&segments).ok_or_else(|| PathError::Syntax(text.to_string()))
    }
}

impl fmt::Display for AttrPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.state {
            Some(state) => write!(f, "{}.{state}", self.attr),
            None => f.write_str(&self.attr),
        }
    }
}

/// カタログ品の属性のパス（`"local/rw_10mNms.power.on"` → (`"local/rw_10mNms"`, `power.on`)）。
/// カタログ品の ID には `.` が使えないので、最初の `.` で分ける。
pub fn parse_catalog_attr(text: &str) -> Result<(String, AttrPath), PathError> {
    let (item, rest) = text
        .trim()
        .split_once('.')
        .ok_or_else(|| PathError::Syntax(text.to_string()))?;
    if split_catalog_ref(item).is_none() {
        return Err(PathError::Syntax(text.to_string()));
    }
    Ok((item.to_string(), AttrPath::parse(rest)?))
}

/// ポートの参照（`"PCDU.OUT3"` → 部品 PCDU のポート OUT3）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
pub struct PortRef {
    pub part: String,
    pub port: String,
}

impl PortRef {
    /// 部品 ID とポート名から作る。
    pub fn new(part: impl Into<String>, port: impl Into<String>) -> Self {
        Self {
            part: part.into(),
            port: port.into(),
        }
    }

    /// `"PART.PORT"` を解析する。
    pub fn parse(text: &str) -> Option<PortRef> {
        let (part, port) = text.split_once('.')?;
        (!part.is_empty() && !port.is_empty() && !port.contains('.')).then(|| PortRef {
            part: part.to_string(),
            port: port.to_string(),
        })
    }
}

impl fmt::Display for PortRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.part, self.port)
    }
}

impl std::str::FromStr for PortRef {
    type Err = PathError;
    /// [`PortRef::parse`] の誤りを文で返す版（CLI の引数・ファイルの読み込み）。
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        PortRef::parse(text).ok_or_else(|| PathError::Port(text.to_string()))
    }
}

/// `"local/rw_10mNms"` → (`"local"`, `"rw_10mNms"`)。
pub fn split_catalog_ref(reference: &str) -> Option<(&str, &str)> {
    let (source, id) = reference.split_once('/')?;
    (!source.is_empty() && !id.is_empty() && !id.contains('/')).then_some((source, id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_attribute_and_setting_paths() {
        assert_eq!(
            parse("RW_X.mass"),
            Ok(ParamPath::Attr {
                part: "RW_X".into(),
                attr: "mass".into(),
                state: None
            })
        );
        assert_eq!(
            parse("RW_X.power.on"),
            Ok(ParamPath::Attr {
                part: "RW_X".into(),
                attr: "power".into(),
                state: Some("on".into())
            })
        );
        assert!(matches!(parse("RW_X.label"), Ok(ParamPath::Field { .. })));
        let setting = parse("analysis.power.dod_max");
        assert!(matches!(&setting, Ok(ParamPath::Setting(s)) if s.scope == SettingScope::Analysis));
        assert_eq!(
            setting.map(|p| p.to_string()).ok(),
            Some("analysis.power.dod_max".into())
        );
        assert!(parse("RW_X").is_err());
        assert!(parse("RW_X..mass").is_err());
    }

    #[test]
    fn parses_attr_and_catalog_paths() {
        let p = AttrPath::parse("power.on").ok();
        assert_eq!(p.as_ref().map(|p| p.to_string()), Some("power.on".into()));
        assert_eq!(AttrPath::parse("mass").ok().and_then(|p| p.state), None);
        assert!(AttrPath::parse("a.b.c").is_err());
        let (item, attr) = parse_catalog_attr("local/rw_10mNms.power.on").unwrap();
        assert_eq!(
            (item.as_str(), attr.attr.as_str()),
            ("local/rw_10mNms", "power")
        );
        assert!(parse_catalog_attr("rw.mass").is_err());
    }

    #[test]
    fn parses_port_refs() {
        assert_eq!(
            PortRef::parse("PCDU.OUT3"),
            Some(PortRef {
                part: "PCDU".into(),
                port: "OUT3".into()
            })
        );
        assert_eq!(PortRef::parse("PCDU"), None);
        assert_eq!(
            "PCDU".parse::<PortRef>(),
            Err(PathError::Port("PCDU".into()))
        );
    }
}
