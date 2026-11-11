//! 部品（製品ツリー上の使用）・取付・ポート・カタログ品（docs/model.md §4・§6）。

use super::defs::{AttributeDef, KindDef, NetKind};
use super::net::Net;
use crate::enum_text::enum_text;
pub use crate::field_names::{CatalogFieldName, PartFieldName};
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
}

enum_text! {
    /// 成熟度（成長余裕と不確かさの既定幅を決める）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
    pub enum Maturity: "成熟度" {
        OffTheShelf = "off_the_shelf",
        Modified = "modified",
        New = "new",
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

/// 位置（`pose.position`・取付点の位置）に単位を書かなかったときの単位。**位置の既定の単位はここだけ**
/// （解決・検査・機体座標での配置が [`position_unit`] で引く）。
pub const DEFAULT_POSITION_UNIT: &str = "m";

/// 位置のベクトルの単位（書かれていなければ [`DEFAULT_POSITION_UNIT`]）。
pub fn position_unit(position: &Vector3) -> &str {
    position.unit.as_deref().unwrap_or(DEFAULT_POSITION_UNIT)
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
    /// フィールド名（[`PartFieldName`]）。
    pub fn key(&self) -> PartFieldName {
        match self {
            PartField::Label(_) => PartFieldName::Label,
            PartField::Description(_) => PartFieldName::Description,
            PartField::Catalog(_) => PartFieldName::Catalog,
            PartField::Kind(_) => PartFieldName::Kind,
            PartField::Count(_) => PartFieldName::Count,
            PartField::Maturity(_) => PartFieldName::Maturity,
            PartField::Candidates(_) => PartFieldName::Candidates,
        }
    }

    /// フィールド名の綴り（ファイルのキー・パス記法の名前）。
    pub fn name(&self) -> &'static str {
        self.key().as_str()
    }

    /// 部品のそのフィールドの今の値（新しいファイルの書き出しが、1 フィールドずつの書き込みと同じ形で書くため）。
    pub fn of(part: &Part, key: PartFieldName) -> Self {
        match key {
            PartFieldName::Label => PartField::Label(part.label.clone()),
            PartFieldName::Description => PartField::Description(part.description.clone()),
            PartFieldName::Catalog => PartField::Catalog(part.catalog.clone()),
            PartFieldName::Kind => PartField::Kind(part.kind.as_ref().map(ToString::to_string)),
            PartFieldName::Candidates => PartField::Candidates(part.candidates.clone()),
            PartFieldName::Count => PartField::Count(part.count),
            PartFieldName::Maturity => PartField::Maturity(part.maturity),
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
    /// フィールド名（[`CatalogFieldName`]）。
    pub fn key(&self) -> CatalogFieldName {
        match self {
            CatalogField::Kind(_) => CatalogFieldName::Kind,
            CatalogField::Label(_) => CatalogFieldName::Label,
            CatalogField::LabelEn(_) => CatalogFieldName::LabelEn,
            CatalogField::Manufacturer(_) => CatalogFieldName::Manufacturer,
            CatalogField::PartNumber(_) => CatalogFieldName::PartNumber,
            CatalogField::Maturity(_) => CatalogFieldName::Maturity,
            CatalogField::Datasheet(_) => CatalogFieldName::Datasheet,
            CatalogField::Source(_) => CatalogFieldName::Source,
        }
    }

    /// フィールド名の綴り（ファイルのキー）。
    pub fn name(&self) -> &'static str {
        self.key().as_str()
    }

    /// カタログ品のそのフィールドの今の値。
    pub fn of(item: &CatalogItem, key: CatalogFieldName) -> Self {
        match key {
            CatalogFieldName::Kind => CatalogField::Kind(item.kind.to_string()),
            CatalogFieldName::Label => CatalogField::Label(item.label.clone()),
            CatalogFieldName::LabelEn => CatalogField::LabelEn(item.label_en.clone()),
            CatalogFieldName::Manufacturer => CatalogField::Manufacturer(item.manufacturer.clone()),
            CatalogFieldName::PartNumber => CatalogField::PartNumber(item.part_number.clone()),
            CatalogFieldName::Maturity => CatalogField::Maturity(item.maturity),
            CatalogFieldName::Datasheet => CatalogField::Datasheet(item.datasheet.clone()),
            CatalogFieldName::Source => CatalogField::Source(item.source.clone()),
        }
    }
}
