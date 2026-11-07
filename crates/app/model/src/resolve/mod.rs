//! Design → Resolved（カタログ・上書き・既定値を重ねた各部品の値。SI・出所・根拠付き）。docs/model.md §5。
//!
//! 純粋関数 [`resolve`]。欠けた値は [`Cell::Unset`] として持ち、エラーにしない（診断は `check` だけが出す）。
//! 取付の姿勢は合成せず、取付ごとの相対姿勢（[`ResolvedMount`]）と連鎖（[`Resolved::pose_chain`]）を返す。
//! 合成（クォータニオンの積）は projection が sat-astro の数学で行う（数学はそこだけに置くため）。

mod compute;

pub use compute::{evidence_of, pose_si, resolve, resolve_catalog_item, si_value};

use crate::design::{CatalogRef, Loc, Maturity, NetKind, NodeRef};
use crate::id::Id;
use crate::value::{Provenance, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// 解決済みの設計。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
pub struct Resolved {
    pub parts: BTreeMap<Id<crate::design::Part>, ResolvedPart>,
}

/// 解決済みの部品。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct ResolvedPart {
    pub id: String,
    pub label: Option<String>,
    /// 製品ツリーの親
    pub parent: Option<NodeRef>,
    /// 最上位の Group（サブシステム）
    pub subsystem: Option<String>,
    /// 選択中のカタログ品
    pub catalog: Option<CatalogRef>,
    pub kind: Option<String>,
    /// 種別の役割（`reaction_wheel` など。継承を含む）
    pub role: Option<String>,
    /// 機器の状態の語彙
    pub states: Vec<String>,
    pub count: u32,
    pub maturity: Option<Maturity>,
    /// 属性 ID → 値。種別の必須・任意の属性は書かれていなくても Unset で入る
    pub attr: BTreeMap<String, ResolvedValue>,
    pub mount: Option<ResolvedMount>,
    pub ports: BTreeMap<String, ResolvedPort>,
    /// 属性の根拠の最弱値
    pub evidence: Evidence,
}

/// 値の出所。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename = "AttrOrigin")]
pub enum Origin {
    Catalog,
    Override,
    /// 属性定義の既定値
    KindDefault,
    Derived,
    Unset,
}

/// 根拠の強さ（弱い順）。値の根拠は出典（`sources/<ID>`）の種類から決まる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Evidence {
    Placeholder,
    /// 出典が書かれていない
    Unsourced,
    Judgement,
    Analysis,
    Datasheet,
    Measured,
}

/// 値があるか。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
#[ts(rename = "ResolvedCell")]
pub enum Cell<T> {
    Set(T),
    Unset,
}

impl<T> Cell<T> {
    /// 値があればその参照。
    pub fn get(&self) -> Option<&T> {
        match self {
            Cell::Set(v) => Some(v),
            Cell::Unset => None,
        }
    }
}

/// SI のスカラー（nominal と任意の不確かさ）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
pub struct SiScalar {
    pub value: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// SI に直した値。ベクトルの位置は m、角度は rad、温度は K、dB は線形の比。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum SiValue {
    Scalar(SiScalar),
    ByState(BTreeMap<String, SiScalar>),
    Vec3([f64; 3]),
    /// w, x, y, z
    Quat([f64; 4]),
    Tensor3([[f64; 3]; 3]),
    Enum(String),
    Bool(bool),
    Text(String),
    Range {
        min: f64,
        max: f64,
    },
    Table(String),
}

impl SiValue {
    /// 状態での値。状態ごとでない値はどの状態でも同じ値を返す。
    pub fn at_state(&self, state: &str) -> Option<SiScalar> {
        match self {
            SiValue::Scalar(s) => Some(*s),
            SiValue::ByState(m) => m.get(state).copied(),
            _ => None,
        }
    }
}

/// 解決済みの値 1 つ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct ResolvedValue {
    pub value: Cell<SiValue>,
    pub origin: Origin,
    pub evidence: Evidence,
    /// 次元名（属性定義から。分からなければ None）
    pub dimension: Option<String>,
    /// 表示単位（属性定義から）
    pub display_unit: String,
    /// 採用された書かれたままの値（上書きがあれば上書き、無ければカタログ）
    pub authored: Option<Value>,
    pub from: Option<Provenance>,
    /// 値が書かれている位置
    pub loc: Option<Loc>,
}

/// SI の相対姿勢。`orientation`（w, x, y, z）は子の座標系のベクトルを親の座標系へ回す回転、
/// `position` は親の座標系での子の原点 [m]。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
pub struct PoseSi {
    pub position: [f64; 3],
    pub orientation: [f64; 4],
}

impl PoseSi {
    /// 恒等。
    pub const IDENTITY: PoseSi = PoseSi {
        position: [0.0; 3],
        orientation: [1.0, 0.0, 0.0, 0.0],
    };
}

/// 取付 1 段（取付先・取付点の姿勢・取付点に対する自分の姿勢）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct ResolvedMount {
    pub on: String,
    pub point: Option<String>,
    /// 取付先の座標系での取付点の姿勢（取付点が無ければ恒等）
    pub point_pose: PoseSi,
    /// 取付点の座標系での自分の姿勢
    pub pose: PoseSi,
}

/// 解決済みのポート。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct ResolvedPort {
    pub net: Option<String>,
    /// ネットの種類（ネットから。無ければ種別・カタログのポートの雛形から）
    pub kind: Option<NetKind>,
}

impl ResolvedPart {
    /// 属性の値（設定されていれば）。
    pub fn value(&self, attr: &str) -> Option<&SiValue> {
        self.attr.get(attr).and_then(|v| v.value.get())
    }

    /// スカラーの nominal（SI）。
    pub fn scalar(&self, attr: &str) -> Option<f64> {
        match self.value(attr)? {
            SiValue::Scalar(s) => Some(s.value),
            _ => None,
        }
    }

    /// 状態での値（SI）。
    pub fn at_state(&self, attr: &str, state: &str) -> Option<SiScalar> {
        self.value(attr)?.at_state(state)
    }

    /// ベクトル（SI）。
    pub fn vec3(&self, attr: &str) -> Option<[f64; 3]> {
        match self.value(attr)? {
            SiValue::Vec3(v) => Some(*v),
            _ => None,
        }
    }
}

impl Resolved {
    /// 機体（取付の根）から部品までの相対姿勢の列。
    /// `body_T_part = chain[0] ∘ chain[1] ∘ …` の順に合成すると機体座標での部品の姿勢になる。
    /// 各取付は (取付点の姿勢, 取付点に対する姿勢) の 2 つを積む。循環していれば途中で止める。
    pub fn pose_chain(&self, part: &str) -> Vec<PoseSi> {
        let mut reversed: Vec<PoseSi> = Vec::new();
        let mut seen: Vec<&str> = Vec::new();
        let mut current = self.parts.get(part);
        while let Some(p) = current {
            if seen.contains(&p.id.as_str()) {
                break;
            }
            seen.push(&p.id);
            let Some(mount) = &p.mount else { break };
            reversed.push(mount.pose);
            reversed.push(mount.point_pose);
            current = self.parts.get(mount.on.as_str());
        }
        reversed.reverse();
        reversed
    }

    /// 取付の根（取付先を持たない部品）までたどった ID（循環なら None）。
    pub fn mount_root(&self, part: &str) -> Option<&str> {
        let mut seen: Vec<&str> = Vec::new();
        let mut current = self.parts.get(part)?;
        loop {
            if seen.contains(&current.id.as_str()) {
                return None;
            }
            seen.push(&current.id);
            match &current.mount {
                Some(m) => current = self.parts.get(m.on.as_str())?,
                None => return Some(&current.id),
            }
        }
    }
}

impl std::fmt::Display for Origin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Catalog => "catalog",
            Self::Override => "override",
            Self::KindDefault => "kind_default",
            Self::Derived => "derived",
            Self::Unset => "unset",
        })
    }
}
impl std::str::FromStr for Origin {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "catalog" => Ok(Self::Catalog),
            "override" => Ok(Self::Override),
            "kind_default" => Ok(Self::KindDefault),
            "derived" => Ok(Self::Derived),
            "unset" => Ok(Self::Unset),
            _ => Err(format!("Unknown Origin: {text}")),
        }
    }
}

impl std::fmt::Display for Evidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Placeholder => "placeholder",
            Self::Unsourced => "unsourced",
            Self::Judgement => "judgement",
            Self::Analysis => "analysis",
            Self::Datasheet => "datasheet",
            Self::Measured => "measured",
        })
    }
}
impl std::str::FromStr for Evidence {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "placeholder" => Ok(Self::Placeholder),
            "unsourced" => Ok(Self::Unsourced),
            "judgement" => Ok(Self::Judgement),
            "analysis" => Ok(Self::Analysis),
            "datasheet" => Ok(Self::Datasheet),
            "measured" => Ok(Self::Measured),
            _ => Err(format!("Unknown Evidence: {text}")),
        }
    }
}
