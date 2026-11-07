//! 設計データの表示用の型（`GET /project`・`/design`・`/parts/{id}`・`/defs`）。
//! ファイルの分け方に関係なく、画面が見たい単位（部品・モード表・ネット）にまとめる。

use super::{
    CatalogItemView, DiagnosticCounts, DiagnosticView, QuantityView, ValueData, ValueView,
};
use fune_model::builtin::Labels;
use fune_model::design::{Source, StateAssign, ViewLayout};
use fune_model::value::Vector3;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// プロジェクトの概要（`GET /project`）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProjectInfo {
    pub name: String,
    /// プロジェクトのルート（絶対パス）
    pub root: String,
    /// 楽観ロック用の revision（`POST /ops` の `base_revision` に渡す）
    pub revision: String,
    /// git のブランチ（git でなければ null）
    pub branch: Option<String>,
    pub schema: u32,
    pub counts: DesignCounts,
    pub diagnostics: DiagnosticCounts,
}

/// エンティティの件数。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct DesignCounts {
    pub parts: u32,
    pub groups: u32,
    pub modes: u32,
    pub nets: u32,
    pub harnesses: u32,
    pub cases: u32,
}

/// 画面表示用の設計全体（`GET /design`）。1 回で画面に要るものがすべて揃う。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DesignView {
    pub project: ProjectInfo,
    /// 製品ツリー（機能の階層）。親から子の順
    pub tree: Vec<TreeNode>,
    pub parts: Vec<PartRow>,
    pub modes: ModeTable,
    pub nets: Vec<NetView>,
    pub harnesses: Vec<HarnessView>,
    pub defs: DefsView,
    pub settings: Vec<SettingView>,
    pub cases: Vec<CaseView>,
    /// 出典の台帳（`sources/<ID>.toml`）
    pub sources: Vec<Source>,
    /// カタログ品（`"local/rw_10mNms"` の順。値は部品と同じ形で、パスと使っている部品も品の中に）
    pub catalog: Vec<CatalogItemView>,
    /// 図の配置（`view/<名前>.toml`。系統図のノードの位置）
    pub views: Vec<ViewLayout>,
    pub diagnostics: Vec<DiagnosticView>,
}

/// 製品ツリーのノード（Group か Part）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TreeNode {
    pub id: String,
    pub node: TreeNodeKind,
    /// 表示名（無ければ ID）
    pub label: String,
    pub parent: Option<String>,
    pub children: Vec<String>,
}

/// ノードの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TreeNodeKind {
    Group,
    Part,
}

/// 部品の一覧の 1 行（解決済みの値付き）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PartRow {
    /// 属性 → 状態（空文字なら属性全体）→ パラメーターパス。
    pub paths: BTreeMap<String, BTreeMap<String, String>>,
    pub id: String,
    /// 製品ツリーの親（Group か組立品の Part）
    pub parent: Option<String>,
    /// 最上位の Group（例: `"AOCS"`）。未所属ならモデルの role_subsystem の既定
    pub subsystem: Option<String>,
    pub kind: String,
    pub role: Option<String>,
    /// 選択中のカタログ（例: `"lab/rw_18mNms"`）
    pub catalog: Option<String>,
    pub candidates: Vec<String>,
    pub count: u32,
    pub maturity: Option<String>,
    /// 属性 ID → 解決済みの値
    pub values: BTreeMap<String, ValueView>,
    pub ports: Vec<PortView>,
    pub mount: Option<MountView>,
    /// 機器の状態の語彙（kind から）
    pub states: Vec<String>,
    /// 部品ファイル（ルートからの相対パス）
    pub file: String,
}

/// 1 部品の全情報（`GET /parts/{id}`）。全系の属性を 1 か所に。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PartView {
    pub part: PartRow,
    /// 系ごとの属性の並び（基本・質量・電源・熱・姿勢・通信…）
    pub sections: Vec<AttrSection>,
    /// モード ID → その部品の状態
    pub modes: BTreeMap<String, StateAssign>,
    /// この部品のポートを通るハーネス
    pub harnesses: Vec<String>,
    /// この部品に関する診断
    pub diagnostics: Vec<DiagnosticView>,
}

/// 系ごとの属性のまとまり。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AttrSection {
    /// 例: `"power"`
    pub key: String,
    /// 例: `"電源"`
    pub label: String,
    /// 英語の見出し（例: `"Power"`）
    pub label_en: Option<String>,
    /// 属性 ID の並び
    pub attrs: Vec<String>,
}

/// ポートと所属ネット。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PortView {
    pub id: String,
    pub net: Option<String>,
}

/// 取付先と、機体座標での位置・姿勢。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct MountView {
    pub on: Option<String>,
    pub point: Option<String>,
    /// 取付点に対する相対位置 [m]
    pub position: Option<[f64; 3]>,
    /// 取付点に対する相対位置の書かれたまま（値と単位）。GUI はこの単位で見せて送る（ファイルの単位を保つ。CLI で同じ単位を打つのと同じ）
    pub authored_position: Option<Vector3>,
    /// 取付点に対する相対姿勢 `[w, x, y, z]`
    pub orientation: Option<[f64; 4]>,
    /// 機体座標での位置 [m]（取付の連鎖をたどったもの）
    pub body_position: Option<[f64; 3]>,
    /// 機体座標での姿勢 `[w, x, y, z]`
    pub body_orientation: Option<[f64; 4]>,
}

/// 部品 × モードの表。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct ModeTable {
    pub modes: Vec<ModeHeader>,
    pub rows: Vec<ModeRow>,
}

/// モードの列見出し。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ModeHeader {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub file: String,
}

/// モード表の 1 行（1 部品）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ModeRow {
    pub part: String,
    pub kind: String,
    /// 選べる状態（kind の語彙）
    pub states: Vec<String>,
    /// モード ID → 状態（そのモードの表に書かれていなければ無い）
    pub cells: BTreeMap<String, StateAssign>,
    /// モード ID → セルの表示（例: `"rx（tx 10.0 %）"`。割合の整形は sat-units）
    pub display: BTreeMap<String, String>,
}

/// ネット（電気的な接続）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct NetView {
    pub id: String,
    /// `"power"` / `"data"` / `"rf"`
    pub kind: fune_model::design::NetKind,
    pub label: Option<String>,
    /// 供給元のポート（part = PCDU、port = OUT3）
    pub source: Option<fune_model::path::PortRef>,
    pub rail: Option<QuantityView>,
    pub limit: Option<QuantityView>,
    pub protocol: Option<String>,
    pub bitrate: Option<QuantityView>,
    pub master: Option<fune_model::path::PortRef>,
    /// このネットに属するポート（part = RW_X、port = PWR）。部品側の ports から集めたもの
    pub members: Vec<fune_model::path::PortRef>,
    pub file: String,
}

/// ハーネス（ケーブル区間）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct HarnessView {
    pub id: String,
    pub label: Option<String>,
    pub from: String,
    pub to: String,
    pub nets: Vec<String>,
    pub wire: Option<String>,
    pub cores: u32,
    pub length: Option<QuantityView>,
    pub file: String,
}

/// 属性定義と種別定義（`GET /defs`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct DefsView {
    pub attributes: Vec<AttributeDefView>,
    pub kinds: Vec<KindDefView>,
    /// 組み込みの語彙の表示名（系の順と見出し・機器の状態・列挙の値。`fune_model::builtin::labels`）
    pub labels: Labels,
}

/// 属性定義。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AttributeDefView {
    pub id: String,
    pub label: String,
    /// 英語の表示名（無ければ `label` を使う）
    pub label_en: Option<String>,
    /// 値の型（`"scalar"` `"vec3"` …。状態ごとの値は by_state の旗）
    pub value_type: fune_model::value::ValueType,
    /// モデルが定める系。
    pub section: String,
    /// 次元の名前（量でなければ null）
    pub dimension: Option<String>,
    pub units: Vec<String>,
    pub display_unit: Option<String>,
    pub by_state: bool,
    /// 組み込み（`fune_model::builtin`）なら true、プロジェクト独自（`defs/`）なら false
    pub builtin: bool,
    pub description: Option<String>,
}

/// 種別定義。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct KindDefView {
    pub id: String,
    pub label: String,
    /// 英語の表示名（無ければ `label` を使う）
    pub label_en: Option<String>,
    pub role: Option<String>,
    pub parent: Option<String>,
    pub states: Vec<String>,
    pub required: Vec<String>,
    pub optional: Vec<String>,
    pub ports: Vec<String>,
    pub builtin: bool,
}

/// 設定 1 項目（`mission/`・`analysis/`・`budget/` の値）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SettingView {
    /// 設定ファイルごとの表示グループのパス接頭辞。
    pub group: String,
    /// ファイル内のキー（表示用）。
    pub field: String,
    /// 例: `"analysis.power.dod_max"`
    pub key: String,
    pub value: ValueData,
    pub display: String,
    pub file: Option<String>,
}

/// 解析ケース。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CaseView {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    /// 使うモード（モードの時系列なら先頭から順に）
    pub modes: Vec<String>,
    pub file: String,
}

/// 部品の一覧の絞り込み（`GET /parts?group=AOCS&kind=reaction_wheel`、`fune part ls`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct PartsQuery {
    /// この Group（の子孫）に属する部品だけ
    pub group: Option<String>,
    /// この種別の部品だけ
    pub kind: Option<String>,
}

/// 1 つの値の問い合わせ（`GET /values/{path}?state=on`、`fune get`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct ValueQuery {
    /// 状態で引く値の状態（例: `"on"`）
    pub state: Option<String>,
}

/// 1 つの値の問い合わせの結果。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ValueAnswer {
    /// 例: `"RW_X.power"`
    pub path: String,
    pub value: ValueView,
    /// `state` を指定したときのその状態の値
    pub selected: Option<QuantityView>,
}
