//! Design → Resolved（カタログ・上書き・既定値を重ねた各部品の値。SI・出所・根拠付き）。docs/model.md §5。
//!
//! 純粋関数 [`resolve`]。欠けた値は [`Cell::Unset`] として持ち、エラーにしない（診断は `check` だけが出す）。
//! 取付の姿勢は合成せず、取付ごとの相対姿勢（[`ResolvedMount`]）と連鎖（[`Resolved::pose_chain`]）を返す。
//! 合成（クォータニオンの積）は projection が sat-astro の数学で行う（数学はそこだけに置くため）。

mod compute;

pub use compute::{evidence_of, pose_si, resolve, resolve_catalog_item, si_value};

use crate::chain::{Chain, chain};
use crate::design::{CatalogRef, Loc, Maturity, NetKind, NodeRef, UncertaintySide};
use crate::enum_text::enum_text;
use crate::id::Id;
use crate::role::Role;
use crate::value::{Provenance, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// 解決済みの設計。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
pub struct Resolved {
    pub parts: BTreeMap<Id<crate::design::Part>, ResolvedPart>,
    /// 形態 ID → その形態で解決した設計（基準の `parts` と、姿勢を上書きした部品の取付だけが違う）。入れ子はここで終わる
    /// （形態の中の `configurations` は空）。API には出さない。引くのは [`Resolved::in_configuration`] だけ
    #[serde(skip)]
    #[ts(skip)]
    pub configurations: BTreeMap<String, Resolved>,
}

impl Resolved {
    /// 形態 `id` で解決した設計。`None` と、無い形態の ID は基準（自分）。**形態の選び方の既定（無い ID は基準）はここだけ。**
    pub fn in_configuration(&self, id: Option<&str>) -> &Resolved {
        id.and_then(|id| self.configurations.get(id))
            .unwrap_or(self)
    }
}

/// 解決済みの部品。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct ResolvedPart {
    pub id: String,
    pub label: Option<String>,
    /// 製品ツリーの親
    pub parent: Option<NodeRef>,
    /// 部品の系（`Design::system_of`）
    pub system: Option<crate::design::AnalysisSystem>,
    /// 選択中のカタログ品
    pub catalog: Option<CatalogRef>,
    pub kind: Option<String>,
    /// 種別の役割（`reaction_wheel` など。継承を含む）
    pub role: Option<Role>,
    /// 機器の状態の語彙
    pub states: Vec<String>,
    pub count: u32,
    pub redundancy: crate::design::Redundancy,
    pub maturity: Option<Maturity>,
    /// 属性 ID → 値。種別の必須・任意の属性は書かれていなくても Unset で入る
    pub attr: BTreeMap<String, ResolvedValue>,
    pub mount: Option<ResolvedMount>,
    pub ports: BTreeMap<String, ResolvedPort>,
    /// 属性の根拠の最弱値
    pub evidence: Evidence,
}

enum_text! {
    /// 値の出所。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
    #[ts(rename = "AttrOrigin")]
    pub enum Origin: "value origin" {
        Catalog = "catalog",
        Override = "override",
        /// 属性定義の既定値
        KindDefault = "kind_default",
        Derived = "derived",
        Unset = "unset",
    }
}

enum_text! {
    /// 根拠の強さ（弱い順）。値の根拠は出典（`sources/<ID>`）の種類から決まる。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
    pub enum Evidence: "evidence strength" {
        Placeholder = "placeholder",
        /// 出典が書かれていない
        Unsourced = "unsourced",
        Judgement = "judgement",
        Analysis = "analysis",
        Datasheet = "datasheet",
        Measured = "measured",
    }
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

impl SiScalar {
    /// 解析ケースの `side` が指す側の値。`max` ならその値の `max`、`min` なら `min`（その側が書かれていなければ nominal）、
    /// `nominal` か未指定（ケースが無いときを含む）なら nominal。**どちら側を読むかの選択はここだけ**（解析・バジェットはこれを通る）。
    pub fn read(&self, side: Option<UncertaintySide>) -> f64 {
        match side {
            Some(UncertaintySide::Max) => self.max.unwrap_or(self.value),
            Some(UncertaintySide::Min) => self.min.unwrap_or(self.value),
            Some(UncertaintySide::Nominal) | None => self.value,
        }
    }
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
    /// 1 変数の関数（SI。引数と値は同じ長さで、引数は増える順）
    Curve {
        at: Vec<f64>,
        values: Vec<f64>,
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
    /// 設計の員数（0 は未指定と同じく 1）。解析入力の count と示量の合計に使う。
    pub fn count_or_one(&self) -> u32 {
        self.count.max(1)
    }

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
        let mut reversed: Vec<PoseSi> = self
            .mount_chain(part)
            .filter_map(|p| p.mount.as_ref())
            .flat_map(|mount| [mount.pose, mount.point_pose])
            .collect();
        reversed.reverse();
        reversed
    }

    /// 取付の根（取付先を持たない部品）までたどった ID（循環か、取付先の部品が無ければ None）。
    pub fn mount_root(&self, part: &str) -> Option<&str> {
        let mut parts = self.mount_chain(part);
        let last = parts.by_ref().last()?;
        (!parts.cycled() && last.mount.is_none()).then_some(last.id.as_str())
    }

    /// 部品の取付先をたどった ID（近いほうから。自分は含まない。循環か、取付先の部品が無いところで止まる）。
    pub fn mount_ancestors(&self, part: &str) -> Vec<&str> {
        self.mount_chain(part)
            .skip(1)
            .map(|p| p.id.as_str())
            .collect()
    }

    /// 取付の根か（取付先を持たず、ほかの部品の取付の連鎖がここで終わる）。
    pub fn is_mount_root(&self, part: &str) -> bool {
        self.parts.get(part).is_some_and(|p| p.mount.is_none())
            && self
                .parts
                .values()
                .any(|p| p.mount.is_some() && self.mount_root(&p.id) == Some(part))
    }

    /// 部品から取付先をたどる連鎖（自分 → 取付先 → … の順。循環すればそこで止まる）。
    fn mount_chain<'a>(
        &'a self,
        part: &str,
    ) -> Chain<
        &'a ResolvedPart,
        &'a str,
        impl Fn(&&'a ResolvedPart) -> &'a str,
        impl FnMut(&&'a ResolvedPart) -> Option<&'a ResolvedPart>,
    > {
        chain(
            self.parts.get(part),
            |p: &&'a ResolvedPart| p.id.as_str(),
            move |p: &&'a ResolvedPart| {
                p.mount.as_ref().and_then(|m| self.parts.get(m.on.as_str()))
            },
        )
    }
}
