//! 設計データの表示用の型（`GET /project`・`/design`・`/parts/{id}`・`/defs`）。
//! ファイルの分け方に関係なく、画面が見たい単位（部品・モード表・ネット）にまとめる。

use super::{
    BodyPoseView, CatalogItemView, CatalogSourceView, DiagnosticCounts, DiagnosticView,
    MountPointView, MountView, QuantityView, ValueData, ValueView,
};
use fune_model::builtin::{Labels, Role};
use fune_model::design::{Maturity, StateAssign, ViewLayout};
use fune_ops::Op;
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
    /// カタログの出所（fune.toml の `[[catalog]]` の順。書き込めるかの印付き）
    pub catalogs: Vec<CatalogSourceView>,
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
    /// マヌーバ（実行の順）
    pub maneuvers: Vec<super::ManeuverView>,
    /// フェーズ（並びの順）
    pub phases: Vec<super::PhaseView>,
    /// 形態（`configurations/<ID>.toml`。ID 順。部品の姿勢は `GET /parts?configuration=` で形態ごとに読む）
    pub configurations: Vec<super::ConfigurationView>,
    /// 地上局（`conditions/ground/<ID>.toml`。ID 順。値は `settings`）
    pub stations: Vec<super::GroundStationView>,
    /// 要求（`requirements/<ID>.toml`。ID 順）
    pub requirements: Vec<super::RequirementView>,
    /// 出典の台帳（`sources/<ID>.toml`。ID 順）
    pub sources: Vec<super::SourceView>,
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
    /// 英語の系の表示名（部品や入れ子のフォルダは None）
    pub label_en: Option<String>,
    /// ノードの系（色の表示にも使う）
    pub system: Option<sat_analysis::spec::System>,
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
    /// 部品のフィールド名（`fune_model::design::PartFieldName`）→ パラメーターパス（`"RW_X.count"`）。
    /// GUI の基本の欄は、打った文字列とこのパスをそのまま `POST /ops/from_text` に送る
    pub field_paths: BTreeMap<String, String>,
    pub id: String,
    /// 製品ツリーの親（Group か組立品の Part）
    pub parent: Option<String>,
    /// 部品の系（置き場所 → 役割 → None）
    pub system: Option<sat_analysis::spec::System>,
    /// 種別（kind の無い部品は null）
    pub kind: Option<String>,
    pub role: Option<Role>,
    /// マヌーバのスラスタに選べる部品か（`Design::is_maneuver_thruster`。検査と同じ判定）
    pub maneuver_thruster: bool,
    /// 構造の部品か（系が構造のもの。3D で外板として薄く描くのに使う。判定は `query::parts::part_row` だけ）
    pub structural: bool,
    /// この部品を対象にした診断の件数（`DiagnosticView.part` が部品 ID のもの）
    pub diagnostics: DiagnosticCounts,
    /// 未設定の値（`values` のうち `origin` が `unset`）の数
    pub unset_count: u32,
    /// 選択中のカタログ（例: `"local/rw_18mNms"`）
    pub catalog: Option<String>,
    pub candidates: Vec<String>,
    pub count: u32,
    pub redundancy: fune_model::design::Redundancy,
    pub maturity: Option<Maturity>,
    /// 属性 ID → 解決済みの値
    pub values: BTreeMap<String, ValueView>,
    pub ports: Vec<PortView>,
    pub mount: Option<MountView>,
    /// 自分に他の部品を付ける点（名前の順。GUI の配置の欄は取付先のこの一覧から取付点を選ぶ）
    pub mount_points: Vec<MountPointView>,
    /// 機体座標での位置・姿勢（取付のある部品と、取付の根。どちらでもない未配置の部品は null）
    pub body: Option<BodyPoseView>,
    /// 機器の状態の語彙（kind から）
    pub states: Vec<String>,
    /// 部品ファイル（ルートからの相対パス）
    pub file: String,
    /// 部品ファイルの置き場所の `parts/` からのディレクトリの区間（例: `["power", "SAP_PY"]`）。
    /// 各区間は製品ツリーのノードの ID（表示名は `DesignView.tree`）。`parts/` の直下なら空
    pub dir: Vec<String>,
}

/// 1 部品の表示の依頼（`POST /parts/{id}`、`fune part show <ID> --ops`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct PartViewRequest {
    /// 保存していない編集。書き込まずに当てた設計で表示する（空なら保存済みの設計。`GET` と同じ）。
    /// GUI のパラメーター追加は、同じ編集で定義を作る属性のパス（`PartRow.paths`）をここから引く
    pub ops: Vec<Op>,
    /// この形態での取付・姿勢を返す（省略は基準。無い形態は拒む）
    pub configuration: Option<String>,
}

/// 1 部品の全情報（`GET /parts/{id}`）。全系の属性を 1 か所に。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PartView {
    pub part: PartRow,
    /// 系ごとの属性の並び（基本・質量・姿勢・C&DH・通信・ミッション機器・電源・推進・構造・熱）
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

/// 部品 × モードの表。列は fune.toml の表示順（未指定のモードは後ろに ID 順）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct ModeTable {
    pub modes: Vec<ModeHeader>,
    pub rows: Vec<ModeRow>,
    /// 電源のモードの並び（名前の順。`default` があれば最初）。無ければ空
    pub sequences: Vec<ModeSequenceView>,
    /// 解析に使う並びの名前（`PowerSettings::active_sequence_name`。並びが無ければ null）
    pub active_sequence: Option<String>,
}

/// 食の出入りからの位置の指定（開始の基準・並びの先頭の位置）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ModeStartView {
    pub from: fune_model::design::StartFrom,
    /// 書かれたオフセット（無ければ null = 0）
    pub offset: Option<QuantityView>,
}

/// モードの並び 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ModeSequenceView {
    pub name: String,
    /// 名前の無い従来の `[[sequence]]` の形で書かれているか
    pub legacy: bool,
    /// 並びの先頭の位置（無ければ解析の始め）
    pub anchor: Option<ModeStartView>,
    pub segments: Vec<ModeSegmentView>,
}

/// 並びの区間 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ModeSegmentView {
    pub mode: String,
    pub duration: QuantityView,
}

/// モードの列見出し。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ModeHeader {
    pub max_duration: Option<QuantityView>,
    /// 最長継続時間の評価で使う時間帯（書かれていなければ null = 指定なし）
    pub when: Option<fune_model::design::ModeWhen>,
    /// 最長継続時間の評価の開始の指定（無ければ null）
    pub start: Option<ModeStartView>,
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub file: String,
}

/// モード表の 1 行（1 部品）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ModeRow {
    pub part: String,
    /// 種別（kind の無い部品は null）
    pub kind: Option<String>,
    /// 選べる状態（kind の語彙）
    pub states: Vec<String>,
    /// 停止の状態（語彙にある `off`。無ければ null。セルの切り替えと記号に使う。選び方は `query::views::stop_and_main_states`）
    pub off_state: Option<String>,
    /// 主な「オン」の状態（語彙に `on` があればそれ、無ければ停止以外の最初の状態、無ければ null）
    pub main_state: Option<String>,
    /// モード ID → 状態（そのモードの表に書かれていなければ無い）
    pub cells: BTreeMap<String, StateAssign>,
    /// 蝕の上書き（無ければ cells を継承）。
    pub eclipse_cells: BTreeMap<String, StateAssign>,
    /// モード ID → そのセルのほかの状態の時間の割合（状態 → `%` の表示。`cells` の `duty` を sat-units で整形したもの）。
    /// 割合の無いセルは入れない。セルの文言（`rx（tx 10.0 %）` など）は CLI と Web が状態とこれから作る
    pub shares: BTreeMap<String, BTreeMap<String, QuantityView>>,
}

/// ネット（電気的な接続）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct NetView {
    pub id: String,
    /// `"power"` / `"data"` / `"rf"`
    pub kind: fune_model::design::NetKind,
    /// 戻り（GND）のネットか（判定は `fune_model::design::Net::is_return`。系統図の GND の層）
    pub is_return: bool,
    /// 電源のネットの系統（電源以外は null。分類は `query::net_class::power_class`）。系統図の線の色と凡例が使う
    pub power_class: Option<super::PowerClass>,
    /// 星形の中心にするポート（無ければ null。選び方は `query::net_class::hub`）。系統図が線の向きを決めるのに使う
    pub hub: Option<super::NetHub>,
    /// 表示名（無ければ ID）
    pub label: String,
    /// 供給元のポート（part = PCDU、port = OUT3）
    pub source: Option<fune_model::path::PortRef>,
    pub rail: Option<QuantityView>,
    pub limit: Option<QuantityView>,
    pub protocol: Option<String>,
    pub bitrate: Option<QuantityView>,
    pub master: Option<fune_model::path::PortRef>,
    /// このネットに属するポート（part = RW_X、port = PWR）。部品側の ports から集めたもの
    pub members: Vec<fune_model::path::PortRef>,
    /// フィールド名（`NetFieldName::NAMES`）→ 対象のパス（`"nets.P12V.rail"`。診断の `DiagnosticView.path` と同じ
    /// `fune_model::Target::net` の表示）。GUI の欄の名前で、検証で弾かれた理由をその欄の下に出す鍵。
    /// 編集はパスではなく `CommandText::NetSet`（`fune net set` の引数）で送る
    pub field_paths: BTreeMap<String, String>,
    pub file: String,
}

/// ハーネス（ケーブル区間）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct HarnessView {
    pub id: String,
    /// 表示名（無ければ ID）
    pub label: String,
    /// 端のポート（part = PCDU、port = OUT3）
    pub from: fune_model::path::PortRef,
    pub to: fune_model::path::PortRef,
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
    /// 解析の系の一覧と並び（`fune_model::design::AnalysisSystem` の宣言の順。表示名は `labels.section`）
    pub systems: Vec<sat_analysis::spec::System>,
    /// バジェットの種類の一覧と並び（`fune_model::design::BudgetKind` の宣言の順。表示名は `labels.enum.budget_kind`）
    pub budget_kinds: Vec<fune_model::design::BudgetKind>,
    /// モードの見方（`POST /system_modes/{system}`）を持つ系（`systems` の部分集合で、同じ並び。`system_modes::mode_systems`）
    pub mode_systems: Vec<sat_analysis::spec::System>,
    /// モードの表の部品ごとの列の出し方（系ごとと、`system` が null の設計のモードのページ。`mode_systems::mode_levels`）
    pub mode_levels: Vec<super::ModeLevelsView>,
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
    /// 方向の属性か（部品座標の単位ベクトル。3D はこれが true の属性を軸の矢印として描く）
    pub unit_vector: bool,
    /// 視野の半角の属性なら、そのまわりの方向の属性の ID（3D の視野の円錐）。そうでなければ null
    pub fov_axis: Option<String>,
    /// 組み込み（`fune_model::builtin`）なら true、プロジェクト独自（`defs/`）なら false
    pub builtin: bool,
    pub description: Option<String>,
    /// 値を欄で直すときの入力の形（量・選択・参照・文字列・直せない理由。決めるのは `fixes::attr_input`）
    pub input: super::AttrInput,
}

/// 種別定義。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct KindDefView {
    pub id: String,
    pub label: String,
    /// 英語の表示名（無ければ `label` を使う）
    pub label_en: Option<String>,
    pub role: Option<Role>,
    pub parent: Option<String>,
    pub states: Vec<String>,
    pub required: Vec<String>,
    pub optional: Vec<String>,
    pub ports: Vec<String>,
    pub builtin: bool,
}

/// 設定 1 項目（`conditions/`・`analysis/`・`budget/` の値）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SettingView {
    /// 置き場所の種類（`conditions` / `analysis` / `budget`。設定のページの区分）
    pub scope: fune_model::path::SettingScope,
    /// この設定を見せる系（系のページの設定のタブ）。どの系にも属さなければ null
    pub system: Option<sat_analysis::spec::System>,
    /// 設定ファイルごとの表示のまとまり（例: `"conditions.orbit"`・`"conditions.ground.<局の ID>"`。`key` = `<group>.<field>`）
    pub group: String,
    /// ファイル内のキー（表示用。例: `"altitude"`）
    pub field: String,
    /// 例: `"analysis.power.dod_max"`
    pub key: String,
    pub value: ValueData,
    /// 値の表示文字列（未設定なら null。文言は CLI と Web が作る）
    pub display: Option<String>,
    pub file: Option<String>,
    /// 選んで入れる欄の選択肢（`conditions.orbit.source`・`conditions.orbit.maneuver`。空なら自由入力）
    pub choices: Vec<fune_projection::Choice>,
}

/// 解析ケース。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CaseView {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    /// 使うモード（モードの時系列なら先頭から順に）
    pub modes: Vec<String>,
    /// フェーズ（`phases/<ID>.toml`。軌道・姿勢則がそのフェーズのものになる）
    pub phase: Option<String>,
    /// 解析を始める時刻（ISO 8601。無ければ軌道の元期から）
    pub epoch: Option<String>,
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
    /// この形態での取付・姿勢（`mount`・`body`）を返す（省略は基準。無い形態は拒む）
    pub configuration: Option<String>,
}

/// 1 部品の表示の依頼（`GET /parts/{id}?configuration=STOWED`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct PartQuery {
    /// この形態での取付・姿勢を返す（省略は基準。無い形態は拒む）
    pub configuration: Option<String>,
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

/// 系別のモード表示の依頼（`POST /system_modes/{system}`、`fune mode view <系>`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct SystemModesRequest {
    /// 保存していない編集。書き込まずに当てた設計で表示する（空なら保存済みの設計）
    pub ops: Vec<Op>,
    /// 解析ケースの ID（`cases/<ID>.toml`）。`analyzed` の解析に使う（省略時は各解析と同じ規則で決める）。部品別の値はケースを取らない
    pub case: Option<String>,
    /// 系の解析も回して、解析に基づく表示を足す（電源: モードごとの電力の行き先・判定・グラフ `SystemModeView.power`）。
    /// 解析を回すので、表だけ欲しいときは偽にする
    pub analyzed: bool,
    /// 表の足もとに並べる行の範囲
    pub summary: ModeSummaryScope,
}

/// 足もとの行の範囲（どの行を返すかは service が決める）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ModeSummaryScope {
    /// 系の全部の行（電源は合計・使用可能電力・余裕）
    #[default]
    All,
    /// 合計の行だけ（判定は付けない）。設計のモードのページが全部品の表の足もとに合計だけ出すのに使う
    Totals,
}

/// 系別のモード表示。状態の定義・編集は ModeTable と共通。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SystemModesView {
    pub system: sat_analysis::spec::System,
    pub modes: Vec<SystemModeView>,
    /// 表を作るのに足りなかった入力（解析の `missing` と同じ形。電力の表で、消費電力を持つ部品や運用モードが無いとき）
    pub missing: Vec<fune_projection::MissingInput>,
    /// 表の足もとに並べる合計・収支の行（上から順。どの行を出すかは系ごとに service が決める。数値の行列を持たない系は空）
    pub summary: Vec<ModeSummaryRow>,
}

/// 系別のモード表の足もとの行 1 つ（値は `ModeValuesView` の `field`。文言は見出し `label` から Web が選ぶ）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ModeSummaryRow {
    pub label: ModeSummaryLabel,
    pub field: ModeSummaryField,
    /// この行に判定（`ModeValuesView.verdict`）を添えるか
    pub verdict: bool,
}

/// 足もとの行の見出しの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ModeSummaryLabel {
    /// 消費電力の合計（成長余裕込み）
    Power,
    /// データの発生レート（圧縮・パケット化前）
    DataRate,
    /// 内部発熱
    Heat,
    /// 使用可能電力
    Available,
    /// 余裕（システム余裕確保後）
    Margin,
}

/// 足もとの行が読む `ModeValuesView` の欄。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ModeSummaryField {
    Total,
    Available,
    Margin,
}

/// 1 モードの系別表示。AOCS は専用カード、数値の行列はほかの系。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SystemModeView {
    pub id: String,
    pub sunlit: ModeValuesView,
    pub eclipse: ModeValuesView,
    pub aocs: Option<super::AocsModeView>,
    /// 電源のモードごとの電力の行き先・判定・グラフ（依頼の `analyzed` が真の電源の見方だけ。ほかは null）
    pub power: Option<super::PowerModeView>,
}

/// 部品別の値と合計・収支。未割り当てや不足は None、警告は verdict に反映。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct ModeValuesView {
    pub parts: BTreeMap<String, Option<QuantityView>>,
    pub total: Option<QuantityView>,
    /// 電源の配分上限からシステム余裕を引いた値。日照・蝕とも同じ設計上限。
    pub available: Option<QuantityView>,
    pub margin: Option<QuantityView>,
    pub verdict: Option<sat_analysis::spec::Verdict>,
}
