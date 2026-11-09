//! パラメーターのパス記法（CLI と API で共通）。
//!
//! | 例 | 意味 |
//! |---|---|
//! | `RW_X.mass` | 部品 RW_X の属性 mass |
//! | `RW_X.power.on` | 状態ごとの属性 power の状態 on のセル |
//! | `RW_X.label` | 部品のフィールド（[`PART_FIELDS`]） |
//! | `mission.orbit.altitude` | `mission/orbit.toml` の `altitude` |
//! | `mission.ground.TOKYO_GS.min_elevation` | `mission/ground/TOKYO_GS.toml` の `min_elevation` |
//! | `analysis.power.dod_max` | `analysis/power.toml` の `dod_max` |
//! | `budget.mass.limit`・`budget.policy.growth.new` | `budget/allocation.toml`・`budget/policy.toml` |
//!
//! ポートの参照（`PCDU.OUT3`。部品 ID と、`.` を含まないポート名）は [`PortRef`]。属性だけのパス（`power.on`）は [`AttrPath`]、
//! カタログ品の属性（`local/rw_10mNms.power.on`）は [`parse_catalog_attr`]。CLI と service はこれらを使い、自前で分割しない。

use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;

/// 部品のフィールド名（属性 ID と重ねてはいけない。検査の `def.reserved` が拒む）。
pub const PART_FIELDS: &[&str] = &[
    "label",
    "description",
    "catalog",
    "kind",
    "count",
    "maturity",
    "candidates",
];

/// カタログ品のフィールド名（`fune catalog set <品> <フィールド>`。属性 ID と重ねてはいけない。検査の `def.reserved` が拒む）。
pub const CATALOG_FIELDS: &[&str] = &[
    "kind",
    "label",
    "label_en",
    "manufacturer",
    "part_number",
    "maturity",
    "datasheet",
    "source",
];

/// 設定の置き場所の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SettingScope {
    Mission,
    Analysis,
    Budget,
}

impl SettingScope {
    /// パスの先頭の綴り。
    pub fn as_str(self) -> &'static str {
        match self {
            SettingScope::Mission => "mission",
            SettingScope::Analysis => "analysis",
            SettingScope::Budget => "budget",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "mission" => Some(SettingScope::Mission),
            "analysis" => Some(SettingScope::Analysis),
            "budget" => Some(SettingScope::Budget),
            _ => None,
        }
    }
}

/// 設定のパス（先頭の種類と、残りの区切り）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
pub struct SettingPath {
    pub scope: SettingScope,
    pub segments: Vec<String>,
}

impl SettingPath {
    /// 種類と区切りの列から作る（`SettingPath::new(SettingScope::Mission, &["orbit", "altitude"])`）。
    pub fn new(scope: SettingScope, segments: &[&str]) -> Self {
        Self {
            scope,
            segments: segments.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    /// `mission.<…>`。
    pub fn mission(segments: &[&str]) -> Self {
        Self::new(SettingScope::Mission, segments)
    }

    /// `analysis.<…>`。
    pub fn analysis(segments: &[&str]) -> Self {
        Self::new(SettingScope::Analysis, segments)
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
}

/// パスを解析する。
pub fn parse(path: &str) -> Result<ParamPath, PathError> {
    let segments: Vec<&str> = path.trim().split('.').collect();
    let bad = || PathError::Syntax(path.to_string());
    if segments.iter().any(|s| s.is_empty()) || segments.len() < 2 {
        return Err(bad());
    }
    if let Some(scope) = SettingScope::parse(segments[0]) {
        return Ok(ParamPath::Setting(SettingPath {
            scope,
            segments: segments[1..].iter().map(|s| s.to_string()).collect(),
        }));
    }
    let part = segments[0].to_string();
    if segments.len() == 2 && PART_FIELDS.contains(&segments[1]) {
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
    if crate::design::split_catalog_ref(item).is_none() {
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
    }
}
