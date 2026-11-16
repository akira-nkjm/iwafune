//! 診断と不足入力の対象（何についての話か）。**部品 ID などは構造で持ち、文字列のパスは `Display` だけが作る。**
//!
//! 受け取る側（service・CLI・Web）は文字列を読み直さず、この構造を見る（どの部品の話か は [`Target::part`]）。
//! 部品の属性・フィールドと設定は、パラメーターのパス記法 [`ParamPath`]（`fune set` と同じ）で持つ。
//!
//! | 対象 | `Display` の例 |
//! |---|---|
//! | 部品そのもの | `RW_X` |
//! | 部品の属性・フィールド、設定 | `RW_X.mass`・`RW_X.power.on`・`RW_X.count`・`analysis.power.dod_max` |
//! | 部品のポート・取付・姿勢 | `RW_X.PWR`・`RW_X.mount`・`RW_X.pose`・`PANEL_MZ.mount_points.RW_SLOT_1` |
//! | どれかの部品（役割で絞る） | `<部品>.power`・`<solar_panel>`・`<obc>.storage_capacity` |
//! | モード・ネット・ハーネス・ケース・マヌーバ | `modes.NOMINAL.TRX`・`nets.P12V.rail`・`harness.H1.length`・`cases.HOT.beta`・`maneuvers.RAISE.thruster` |
//! | カタログ品・定義・図・出典 | `local/rw_10mNms.mass`・`mass`・`reaction_wheel`・`view.electrical.POWER`・`sources.em_test` |

use crate::path::{AttrPath, ParamPath, PortRef, SettingPath};
use crate::role::Role;
use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;

/// 診断・不足入力の対象。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(rename = "DesignTarget")]
pub enum Target {
    /// 部品そのもの（ID の誤り・定義の欠け・ファイルの読み込みの誤り）
    Part { id: String },
    /// 部品の属性（と状態のセル）・部品のフィールド・設定（パス記法は [`crate::path`]）
    Param(ParamPath),
    /// 部品のポート（所属するネット）
    Port(PortRef),
    /// 部品の取付（取付先・取付点・循環）
    Mount { part: String },
    /// 部品の姿勢（`mount_point` があれば、自分に他の部品を付ける取付点の姿勢）
    Pose {
        part: String,
        mount_point: Option<String>,
    },
    /// どれかの部品（特定の部品を指さない）。`roles` が空でなければ、その役割の部品のどれか
    AnyPart {
        roles: Vec<Role>,
        attr: Option<String>,
    },
    /// 機能のまとまり（系のディレクトリなど、ID の誤り）
    Group { id: String },
    /// 運用モード（`part` があれば、モードの表のその部品の行）
    Mode { id: String, part: Option<String> },
    /// 運用モードの全体（1 つも無いとき）
    Modes,
    /// マヌーバの全体（1 つも無いとき）
    Maneuvers,
    /// ネット（`field` はネットのキー。`rail`・`source` など）
    Net { id: String, field: Option<String> },
    /// ハーネス区間（`field` は `from`・`to`・`length` など）
    Harness { id: String, field: Option<String> },
    /// 解析ケース（`field` は `beta` など）
    Case { id: String, field: Option<String> },
    /// マヌーバ（`field` は `thruster`・`target.altitude` など）
    Maneuver { id: String, field: Option<String> },
    /// カタログ品（`attr` があれば、その品の属性）
    Catalog { id: String, attr: Option<AttrPath> },
    /// 属性定義（`defs/attribute/<id>.toml`）
    AttributeDef { id: String },
    /// 種別定義（`defs/kind/<id>.toml`）
    KindDef { id: String },
    /// 出典の台帳（`sources/<id>.toml`）
    Source { id: String },
    /// 図の配置（`node` があれば、その図のノード）
    View { id: String, node: Option<String> },
    /// 別の解析の結果（解析 ID。入力の一部をその解析で求めるとき）
    Analysis { id: String },
}

impl Target {
    /// 部品の属性（`RW_X.mass`）。
    pub fn attr(part: impl Into<String>, attr: impl Into<String>) -> Self {
        Target::Param(ParamPath::Attr {
            part: part.into(),
            attr: attr.into(),
            state: None,
        })
    }

    /// 部品のフィールド（`RW_X.count`）。
    pub fn field(part: impl Into<String>, field: impl Into<String>) -> Self {
        Target::Param(ParamPath::Field {
            part: part.into(),
            field: field.into(),
        })
    }

    /// どれかの部品の属性（`<部品>.power`）。
    pub fn any_part(attr: impl Into<String>) -> Self {
        Target::AnyPart {
            roles: Vec::new(),
            attr: Some(attr.into()),
        }
    }

    /// 役割がどれかの部品（`<battery>`・`<obc>.storage_capacity`）。
    pub fn any_of_roles(roles: &[Role], attr: Option<&str>) -> Self {
        Target::AnyPart {
            roles: roles.to_vec(),
            attr: attr.map(str::to_owned),
        }
    }

    /// 運用モード（`part` があれば、その表の部品の行。`modes.NOMINAL.TRX`）。
    pub fn mode(id: impl Into<String>, part: Option<&str>) -> Self {
        Target::Mode {
            id: id.into(),
            part: part.map(str::to_owned),
        }
    }

    /// ネット（`field` があればそのキー。`nets.P12V.rail`）。
    pub fn net(id: impl Into<String>, field: Option<&str>) -> Self {
        Target::Net {
            id: id.into(),
            field: field.map(str::to_owned),
        }
    }

    /// ハーネス区間（`field` があればそのキー。`harness.H1.length`）。
    pub fn harness(id: impl Into<String>, field: Option<&str>) -> Self {
        Target::Harness {
            id: id.into(),
            field: field.map(str::to_owned),
        }
    }

    /// 解析ケース（`field` があればそのキー。`cases.HOT.beta`）。
    pub fn case(id: impl Into<String>, field: Option<&str>) -> Self {
        Target::Case {
            id: id.into(),
            field: field.map(str::to_owned),
        }
    }

    /// マヌーバ（`field` があればそのキー。`maneuvers.RAISE.thruster`）。
    pub fn maneuver(id: impl Into<String>, field: Option<&str>) -> Self {
        Target::Maneuver {
            id: id.into(),
            field: field.map(str::to_owned),
        }
    }

    /// 図の配置（`node` があればそのノード。`view.electrical.POWER`）。
    pub fn view(id: impl Into<String>, node: Option<&str>) -> Self {
        Target::View {
            id: id.into(),
            node: node.map(str::to_owned),
        }
    }

    /// 対象が部品のファイルの中にあれば、その部品 ID（部品そのもの・属性・フィールド・ポート・取付・姿勢）。
    /// モードの行・ネットの端・どれかの部品などは、部品のファイルの外なので None。
    pub fn part(&self) -> Option<&str> {
        match self {
            Target::Part { id } => Some(id),
            Target::Param(ParamPath::Attr { part, .. } | ParamPath::Field { part, .. })
            | Target::Mount { part }
            | Target::Pose { part, .. } => Some(part),
            Target::Port(port) => Some(&port.part),
            _ => None,
        }
    }
}

impl From<ParamPath> for Target {
    fn from(p: ParamPath) -> Self {
        Target::Param(p)
    }
}

impl From<SettingPath> for Target {
    fn from(p: SettingPath) -> Self {
        Target::Param(ParamPath::Setting(p))
    }
}

impl From<PortRef> for Target {
    fn from(p: PortRef) -> Self {
        Target::Port(p)
    }
}

/// `head` に、あれば `.tail` を続ける。
fn with_tail(
    f: &mut fmt::Formatter<'_>,
    head: fmt::Arguments<'_>,
    tail: Option<&str>,
) -> fmt::Result {
    f.write_fmt(head)?;
    match tail {
        Some(t) => write!(f, ".{t}"),
        None => Ok(()),
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Target::Part { id }
            | Target::Group { id }
            | Target::AttributeDef { id }
            | Target::KindDef { id }
            | Target::Analysis { id } => f.write_str(id),
            Target::Param(p) => p.fmt(f),
            Target::Port(p) => p.fmt(f),
            Target::Mount { part } => write!(f, "{part}.mount"),
            Target::Pose {
                part,
                mount_point: None,
            } => write!(f, "{part}.pose"),
            Target::Pose {
                part,
                mount_point: Some(point),
            } => write!(f, "{part}.mount_points.{point}"),
            Target::AnyPart { roles, attr } => {
                let head = if roles.is_empty() {
                    "部品".to_owned()
                } else {
                    roles
                        .iter()
                        .map(|r| r.as_str())
                        .collect::<Vec<_>>()
                        .join("|")
                };
                with_tail(f, format_args!("<{head}>"), attr.as_deref())
            }
            Target::Mode { id, part } => with_tail(f, format_args!("modes.{id}"), part.as_deref()),
            Target::Modes => f.write_str("modes"),
            Target::Maneuvers => f.write_str("maneuvers"),
            Target::Net { id, field } => with_tail(f, format_args!("nets.{id}"), field.as_deref()),
            Target::Harness { id, field } => {
                with_tail(f, format_args!("harness.{id}"), field.as_deref())
            }
            Target::Case { id, field } => {
                with_tail(f, format_args!("cases.{id}"), field.as_deref())
            }
            Target::Maneuver { id, field } => {
                with_tail(f, format_args!("maneuvers.{id}"), field.as_deref())
            }
            Target::Catalog { id, attr: None } => f.write_str(id),
            Target::Catalog { id, attr: Some(a) } => write!(f, "{id}.{a}"),
            Target::Source { id } => write!(f, "sources.{id}"),
            Target::View { id, node } => with_tail(f, format_args!("view.{id}"), node.as_deref()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some(s: &str) -> Option<String> {
        Some(s.to_owned())
    }

    /// `Display` は今までの文字列のパスと同じ綴り。
    #[test]
    fn display_spells_the_parameter_paths() {
        let cases = [
            (Target::Part { id: "RW_X".into() }, "RW_X"),
            (Target::attr("RW_X", "mass"), "RW_X.mass"),
            (Target::field("RW_X", "count"), "RW_X.count"),
            (
                SettingPath::analysis(&["power", "dod_max"]).into(),
                "analysis.power.dod_max",
            ),
            (
                PortRef {
                    part: "PCDU".into(),
                    port: "OUT3".into(),
                }
                .into(),
                "PCDU.OUT3",
            ),
            (
                Target::Mount {
                    part: "RW_X".into(),
                },
                "RW_X.mount",
            ),
            (
                Target::Pose {
                    part: "PANEL_MZ".into(),
                    mount_point: some("RW_SLOT_1"),
                },
                "PANEL_MZ.mount_points.RW_SLOT_1",
            ),
            (Target::any_part("power"), "<部品>.power"),
            (Target::any_of_roles(&[Role::Battery], None), "<battery>"),
            (
                Target::any_of_roles(&[Role::Obc], Some("storage_capacity")),
                "<obc>.storage_capacity",
            ),
            (
                Target::Mode {
                    id: "NOMINAL".into(),
                    part: some("TRX"),
                },
                "modes.NOMINAL.TRX",
            ),
            (Target::Modes, "modes"),
            (
                Target::Net {
                    id: "P12V".into(),
                    field: some("rail"),
                },
                "nets.P12V.rail",
            ),
            (
                Target::Harness {
                    id: "H1".into(),
                    field: None,
                },
                "harness.H1",
            ),
            (
                Target::Case {
                    id: "HOT".into(),
                    field: some("beta"),
                },
                "cases.HOT.beta",
            ),
            (
                Target::maneuver("RAISE", Some("thruster")),
                "maneuvers.RAISE.thruster",
            ),
            (
                Target::Catalog {
                    id: "local/rw".into(),
                    attr: AttrPath::parse("power.on").ok(),
                },
                "local/rw.power.on",
            ),
            (
                Target::View {
                    id: "electrical".into(),
                    node: some("POWER"),
                },
                "view.electrical.POWER",
            ),
        ];
        for (target, text) in cases {
            assert_eq!(target.to_string(), text);
        }
    }

    /// 部品の ID を持つのは、部品のファイルの中の対象だけ。
    #[test]
    fn part_is_only_for_targets_in_a_part_file() {
        assert_eq!(Target::attr("RW_X", "mass").part(), Some("RW_X"));
        assert_eq!(
            Target::Mount {
                part: "RW_X".into()
            }
            .part(),
            Some("RW_X")
        );
        assert_eq!(Target::any_part("power").part(), None);
        let case = Target::Case {
            id: "HOT".into(),
            field: some("beta"),
        };
        assert_eq!(case.part(), None);
        let mode = Target::Mode {
            id: "NOMINAL".into(),
            part: some("TRX"),
        };
        assert_eq!(mode.part(), None);
        assert_eq!(
            Target::from(SettingPath::conditions(&["orbit"])).part(),
            None
        );
    }

    /// JSON は `kind` で見分ける（部品の属性はパス記法の型 `ParamPath` のまま入る）。
    #[test]
    fn serializes_with_a_kind_tag() {
        let v = serde_json::to_value(Target::attr("RW_X", "mass")).unwrap();
        assert_eq!(
            v,
            serde_json::json!({ "kind": "param", "target": "attr", "part": "RW_X", "attr": "mass", "state": null })
        );
        let v = serde_json::to_value(Target::any_part("power")).unwrap();
        assert_eq!(
            v,
            serde_json::json!({ "kind": "any_part", "roles": [], "attr": "power" })
        );
        let back: Target = serde_json::from_value(v).unwrap();
        assert_eq!(back, Target::any_part("power"));
    }
}
