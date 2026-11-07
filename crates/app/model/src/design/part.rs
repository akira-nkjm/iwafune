//! 部品（製品ツリー上の使用）・取付・ポート・カタログ品（docs/model.md §4・§6）。

use super::defs::{AttributeDef, KindDef, NetKind};
use super::net::Net;
use crate::id::Id;
pub use crate::path::split_catalog_ref;
use crate::value::{Authored, Quaternion, Value, Vector3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// カタログ品の参照（`<カタログ名>/<ID>`。例: `"local/rw_10mNms"`）。
pub type CatalogRef = String;

/// 製品ツリーのノード。Group と Part の ID は同じ名前空間で一意。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum NodeRef {
    Group(Id<Group>),
    Part(Id<Part>),
}

impl NodeRef {
    /// ノードの ID（文字列）。
    pub fn id(&self) -> &str {
        match self {
            NodeRef::Group(id) => id.as_str(),
            NodeRef::Part(id) => id.as_str(),
        }
    }
}

/// 機能のまとまり（`parts/` 以下の、同名ファイルを持たないディレクトリ）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct Group {
    pub id: Id<Group>,
    /// ディレクトリから決まる親（最上位なら None）
    pub parent: Option<NodeRef>,
    /// `fune.toml` の `[groups]` の表示名
    pub label: Option<String>,
}

/// 成熟度（成長余裕と不確かさの既定幅を決める）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Maturity {
    OffTheShelf,
    Modified,
    New,
}

impl Maturity {
    /// ファイルに書く綴り。
    pub fn as_str(self) -> &'static str {
        match self {
            Maturity::OffTheShelf => "off_the_shelf",
            Maturity::Modified => "modified",
            Maturity::New => "new",
        }
    }
}

/// 取付先（取付先の部品と、その部品の取付点）。`point` が無ければ取付先の原点。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Mount {
    pub on: Id<Part>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point: Option<String>,
}

/// 親の座標系に対する相対位置・姿勢。`orientation` は子の座標系のベクトルを親の座標系へ回す回転。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Pose {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Vector3>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<Quaternion>,
}

/// ポートの所属ネット。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct PortBinding {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub net: Option<Id<Net>>,
}

/// 製品ツリー上の使用（ElementUsage）。ファイルは `parts/<…>/<ID>.toml`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct Part {
    pub id: Id<Part>,
    /// ディレクトリから決まる親（ファイルには書かない）
    pub parent: Option<NodeRef>,
    pub label: Option<String>,
    pub description: Option<String>,
    /// 選択中のカタログ品
    pub catalog: Option<CatalogRef>,
    /// 種別（カタログを使わないとき、またはカタログの種別を明示するとき）
    pub kind: Option<Id<KindDef>>,
    /// トレード候補。`catalog` が選択中の 1 つ
    pub candidates: Vec<CatalogRef>,
    /// 員数
    pub count: u32,
    pub maturity: Option<Maturity>,
    pub mount: Option<Mount>,
    pub pose: Option<Pose>,
    /// 自分に他の部品を付ける点
    pub mount_points: BTreeMap<String, Pose>,
    /// カタログ値への上書き
    pub attr: BTreeMap<Id<AttributeDef>, Authored<Value>>,
    /// ポート → 所属ネット
    pub ports: BTreeMap<String, PortBinding>,
}

impl Part {
    /// 空の部品（員数 1）。
    pub fn new(id: impl Into<Id<Part>>) -> Self {
        Self {
            id: id.into(),
            parent: None,
            label: None,
            description: None,
            catalog: None,
            kind: None,
            candidates: Vec::new(),
            count: 1,
            maturity: None,
            mount: None,
            pose: None,
            mount_points: BTreeMap::new(),
            attr: BTreeMap::new(),
            ports: BTreeMap::new(),
        }
    }
}

/// カタログ品（ElementDefinition）。ファイルは `<カタログのパス>/<ID>.toml`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct CatalogItem {
    /// 出所付きの ID（`"local/rw_10mNms"`）
    pub id: CatalogRef,
    pub kind: Id<KindDef>,
    pub label: Option<String>,
    /// 英語の表示名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_en: Option<String>,
    pub manufacturer: Option<String>,
    pub part_number: Option<String>,
    pub maturity: Option<Maturity>,
    /// データシートの場所（URL かリポジトリ内のパス）
    pub datasheet: Option<String>,
    /// 出典を書いていない値に当てる既定の出典（`sources/<ID>`）
    pub source: Option<String>,
    pub attr: BTreeMap<Id<AttributeDef>, Authored<Value>>,
    /// ポートの雛形（ポート名 → ネットの種類）
    pub ports: BTreeMap<String, NetKind>,
}

/// 部品の型付きのフィールド（`SetField` の Op と store の書き込みが共有する）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "field", content = "value", rename_all = "snake_case")]
pub enum PartField {
    Label(Option<String>),
    Description(Option<String>),
    Catalog(Option<CatalogRef>),
    Kind(Option<String>),
    Count(u32),
    Maturity(Option<Maturity>),
    Candidates(Vec<CatalogRef>),
}

impl PartField {
    /// フィールド名（ファイルのキー・パス記法の名前。[`crate::path::PART_FIELDS`] と同じ）。
    pub fn name(&self) -> &'static str {
        match self {
            PartField::Label(_) => "label",
            PartField::Description(_) => "description",
            PartField::Catalog(_) => "catalog",
            PartField::Kind(_) => "kind",
            PartField::Count(_) => "count",
            PartField::Maturity(_) => "maturity",
            PartField::Candidates(_) => "candidates",
        }
    }
}

/// カタログ品の型付きのフィールド（`SetCatalogField` の Op と store の書き込みが共有する）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "field", content = "value", rename_all = "snake_case")]
pub enum CatalogField {
    Kind(String),
    Label(Option<String>),
    LabelEn(Option<String>),
    Manufacturer(Option<String>),
    PartNumber(Option<String>),
    Maturity(Option<Maturity>),
    Datasheet(Option<String>),
    Source(Option<String>),
}

impl CatalogField {
    /// フィールド名（ファイルのキー。[`crate::path::CATALOG_FIELDS`] と同じ）。
    pub fn name(&self) -> &'static str {
        match self {
            CatalogField::Kind(_) => "kind",
            CatalogField::Label(_) => "label",
            CatalogField::LabelEn(_) => "label_en",
            CatalogField::Manufacturer(_) => "manufacturer",
            CatalogField::PartNumber(_) => "part_number",
            CatalogField::Maturity(_) => "maturity",
            CatalogField::Datasheet(_) => "datasheet",
            CatalogField::Source(_) => "source",
        }
    }
}

impl std::str::FromStr for Maturity {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "off_the_shelf" => Ok(Self::OffTheShelf),
            "modified" => Ok(Self::Modified),
            "new" => Ok(Self::New),
            _ => Err(format!("Unknown Maturity: {text}")),
        }
    }
}
