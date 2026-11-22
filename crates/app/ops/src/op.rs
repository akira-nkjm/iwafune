//! Op の型。編集の唯一の入口。CLI のコマンドも GUI の操作もこれに変換する。
//!
//! JSON では `{ "op": "set_value", ... }` の形（`op` が種類）。

use fune_model::design::{
    AttributeDef, Case, CatalogField, Harness, KindDef, Maneuver, Maturity, ModeSegment, ModeWhen,
    Net, NetField, PartField, Pose, Requirement, Source, StartRef,
};
use fune_model::value::{Provenance, Quaternion, SettingValue, Value, Vector3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// 編集 1 件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    // ---- 部品
    /// 部品を足す。`group` は親のグループ（無ければ作る）か組立品の部品。無ければ `parts/` の直下。
    /// `group_under` は `group` の祖先の ID の列（外側から。例: 系 `aocs` の下のグループ `RW` なら `["aocs"]`）。
    /// 空でなければ、`group` はその連鎖の下のグループとして（無ければ作って）使う。
    /// `catalog` か `kind` のどちらかが要る。モードには `mode_state`（既定は種別の `off`）で 1 行ずつ足す。
    CreatePart {
        id: String,
        #[serde(default)]
        group: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        group_under: Vec<String>,
        #[serde(default)]
        catalog: Option<String>,
        #[serde(default)]
        kind: Option<String>,
        #[serde(default)]
        count: Option<u32>,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        mode_state: Option<String>,
    },
    /// 部品を消す。モードの行（`[states]`・`[eclipse]`）と配分のキーは一緒に消す。子部品を持つ組立品は拒む。
    /// ネットの端・ハーネス・取付・図・解析の設定などの参照は書き換えず、参照先が無くなる検査の誤り（`ref.*`）で計画ごと拒む。
    RemovePart {
        id: String,
    },
    /// 部品の名前を変える（modes・nets の source / master・harness・取付・配分・設定の参照も書き換える）。
    RenamePart {
        id: String,
        to: String,
    },
    /// 系の下の入れ子のフォルダ（グループ）の名前を変える。`parts/` の下のディレクトリを移し、図の配置の参照も書き換える。
    /// 最上位のフォルダは系の ID なので、系の ID 以外には変えられない（`fune check` の error）。
    RenameGroup {
        id: String,
        to: String,
    },
    /// 部品を別のグループ（または組立品）へ移す。`to` が None なら `parts/` の直下。
    /// `under` は `to` の祖先の ID の列（外側から。例: 系 `aocs` の下のグループ `RW` なら `["aocs"]`）。
    /// 空でなければ `to` はその連鎖の下のグループとして使い、無ければ作る（途中の段も）。
    /// 連鎖の先頭は既存のノードで、新しいグループは別のグループの下にしか作れない。
    MovePart {
        id: String,
        #[serde(default)]
        to: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        under: Vec<String>,
    },
    /// 属性の値を書く（`state` があれば状態ごとの値の 1 セル）。
    SetValue {
        part: String,
        attr: String,
        #[serde(default)]
        state: Option<String>,
        value: Value,
        #[serde(default)]
        from: Option<Provenance>,
    },
    /// 属性の上書きを消す（`state` があればそのセルだけ）。
    UnsetValue {
        part: String,
        attr: String,
        #[serde(default)]
        state: Option<String>,
    },
    /// 型付きのフィールドを書く。
    SetField {
        part: String,
        field: PartField,
    },
    /// 取付と姿勢。`on` が None なら取付を外す（機体の根になる）。
    Place {
        part: String,
        #[serde(default)]
        on: Option<String>,
        #[serde(default)]
        point: Option<String>,
        #[serde(default)]
        position: Option<Vector3>,
        #[serde(default)]
        orientation: Option<Quaternion>,
    },
    /// ポートのネット所属（`net` が None なら外す）。
    SetPort {
        part: String,
        port: String,
        #[serde(default)]
        net: Option<String>,
    },
    /// 取付点を書く（`pose` が None なら消す）。
    SetMountPoint {
        part: String,
        point: String,
        #[serde(default)]
        pose: Option<Pose>,
    },
    // ---- カタログ品（このプロジェクトの `catalog/` だけ。共有カタログは読み取り専用）
    /// カタログ品を足す。`id` は `local/rw_30mNms`（カタログ名を省けば最初の `path` のカタログ）。
    /// `copy_from_part` があれば、その部品の今の値（カタログ値＋上書き）とポートの雛形を写す。
    CreateCatalogItem {
        id: String,
        kind: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        label_en: Option<String>,
        #[serde(default)]
        maturity: Option<Maturity>,
        #[serde(default)]
        copy_from_part: Option<String>,
        /// 置くディレクトリ（カタログのディレクトリからの相対。`a/b`。省けばカタログの直下。読み方は `fune_store::layout::parse_subdir`）
        #[serde(default)]
        #[ts(optional = nullable)]
        dir: Option<String>,
    },
    /// カタログ品の属性の値を書く（`state` があれば状態ごとの値の 1 セル）。
    SetCatalogValue {
        item: String,
        attr: String,
        #[serde(default)]
        state: Option<String>,
        value: Value,
        #[serde(default)]
        from: Option<Provenance>,
    },
    /// カタログ品の属性の値を消す。
    UnsetCatalogValue {
        item: String,
        attr: String,
        #[serde(default)]
        state: Option<String>,
    },
    /// カタログ品の型付きのフィールドを書く。
    SetCatalogField {
        item: String,
        field: CatalogField,
    },
    /// カタログ品を消す（部品・ハーネスから参照されていれば拒む）。
    RemoveCatalogItem {
        id: String,
    },
    /// カタログ品の名前を変える（部品の catalog・candidates、ハーネスの wire も書き換える）。
    RenameCatalogItem {
        from: String,
        to: String,
    },
    /// カタログ品を別のディレクトリへ移す（`dir` はカタログのディレクトリからの相対。`.` ならカタログの直下）。ID・参照は変わらない。
    MoveCatalogItem {
        item: String,
        dir: String,
    },
    // ---- カタログの出所（fune.toml の `[[catalog]]`）
    /// 共有カタログ（`[[catalog]] name = … git = … rev = …`）を足す。取得はしない（`fune catalog fetch`）。
    /// 名前は ID の規則に従い、既にあるカタログ（`path` のものも）と重ならないこと。`git` は空にできない。
    AddCatalogSource {
        name: String,
        git: String,
        #[serde(default)]
        rev: Option<String>,
    },
    /// 共有カタログを外す（`git = …` のものだけ。部品・ハーネスがその品を参照していれば拒む）。
    /// catalog.lock の記録は次の `fune catalog fetch` が外す。
    RemoveCatalogSource {
        name: String,
    },
    // ---- 定義
    /// 属性定義を足す（次元と許可単位を検査する）。
    CreateAttribute {
        def: AttributeDef,
    },
    /// 種別定義を足す。
    CreateKind {
        def: KindDef,
    },
    /// プロジェクト独自の属性定義を丸ごと置き換える（`def.id` が対象。組み込みは不可）。既存の値が合わなくなる編集は検査で拒む。
    UpdateAttribute {
        def: AttributeDef,
    },
    /// プロジェクト独自の種別定義を丸ごと置き換える（`def.id` が対象。組み込みは不可）。
    UpdateKind {
        def: KindDef,
    },
    /// プロジェクト独自の属性定義を消す（種別が挙げているか、部品・カタログ品が値を書いていれば拒む。組み込みは不可）。
    DeleteAttribute {
        id: String,
    },
    /// プロジェクト独自の種別定義を消す（部品・カタログ品が使っているか、ほかの種別が継承していれば拒む。組み込みは不可）。
    DeleteKind {
        id: String,
    },
    /// プロジェクト独自の属性定義の名前を変える（種別の `required` / `optional`・部品とカタログ品の属性の値のキーも書き換える）。
    /// 組み込みは不可。共有カタログの品がその属性の値を持っていれば、書き換えられないので拒む。
    RenameAttribute {
        id: String,
        to: String,
    },
    /// プロジェクト独自の種別定義の名前を変える（部品とカタログ品の `kind`・ほかの種別の `extends` も書き換える）。
    /// 組み込みは不可。共有カタログの品がその種別を使っていれば、書き換えられないので拒む。
    RenameKind {
        id: String,
        to: String,
    },
    // ---- モード
    /// モードを足す（`copy_from` があればその表を写す）。
    CreateMode {
        id: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        copy_from: Option<String>,
    },
    RemoveMode {
        id: String,
    },
    /// モードの名前を変える（ケース・解析の設定の参照も書き換える）。
    RenameMode {
        id: String,
        to: String,
    },
    /// モードを指定した相対位置へ移す。
    MoveMode {
        id: String,
        position: ModePosition,
    },
    /// モードの最長継続時間（None で削除）。
    SetModeDuration {
        mode: String,
        duration: Option<fune_model::value::Quantity>,
    },
    /// モードの時刻の指定（最長継続時間の評価で使う時間帯 `when` と、開始の基準 `start`）。どちらも None なら指定を消す。
    SetModeTiming {
        mode: String,
        #[serde(default)]
        when: Option<ModeWhen>,
        #[serde(default)]
        start: Option<StartRef>,
    },
    /// 電源のモードの並びを名前付きで丸ごと書く（`analysis/power.toml` の `[sequences.<名前>]`）。無ければ作る。
    /// 従来の名前の無い `[[sequence]]` は `default` の並びなので、`default` を書くと `[[sequence]]` は消える（移行）。
    /// 使う並びの選択は `SetSetting` の `analysis.power.active_sequence`。
    SetSequence {
        name: String,
        /// 並びの先頭の位置（食の出入り + オフセット）。None なら解析の始め
        #[serde(default)]
        anchor: Option<StartRef>,
        segments: Vec<ModeSegment>,
    },
    /// 電源のモードの並びを消す（使う並びだった場合は選択も外す）。
    RemoveSequence {
        name: String,
    },
    /// モード表のセルを一括で書く（GUI の表の編集）。
    SetModeStates {
        cells: Vec<ModeCell>,
    },
    // ---- ネット
    CreateNet {
        net: Net,
    },
    /// ネットの性質を丸ごと置き換える（`net.id` が対象）。
    UpdateNet {
        net: Net,
    },
    /// ネットのフィールドを 1 つ書く（ほかの行は触らない）。`UpdateNet` は丸ごとの置き換え。
    SetNetField {
        net: String,
        field: NetField,
    },
    /// ネットを消す（部品のポートとハーネスからも外す）。
    RemoveNet {
        id: String,
    },
    // ---- 解析ケース・出典・ハーネス・マヌーバ（無ければ作り、あれば変わった行だけを書き換える）
    /// 解析ケースを書く（`case.id` が対象）。
    SetCase {
        case: Case,
    },
    RemoveCase {
        id: String,
    },
    /// 出典を書く（`source.id` が対象）。値から参照されている出典は消せない（検証で拒む）。
    /// `dir`（`sources/` からの相対。`a/b`。省けば直下。読み方は `fune_store::layout::parse_subdir`）は新しい出典の置き場所
    /// で、既にある出典は今のファイルに書く（今と違う `dir` は拒む。移すのは `MoveSource`）。
    SetSource {
        source: Source,
        #[serde(default)]
        #[ts(optional = nullable)]
        dir: Option<String>,
    },
    RemoveSource {
        id: String,
    },
    /// 出典を別のディレクトリへ移す（`dir` は `sources/` からの相対。`.` なら直下）。ID・参照は変わらない。
    MoveSource {
        id: String,
        dir: String,
    },
    /// 地上局を足す（`conditions/ground/<id>.toml`。同じ ID があれば拒む）。帯域ごとの性能（`[link.<帯域>]`）は `SetSetting` で書く。
    /// 緯度・経度・高度・最低仰角は角度・長さの量（次元は検査が見る）。
    CreateGroundStation {
        id: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        latitude: Option<fune_model::value::Quantity>,
        #[serde(default)]
        longitude: Option<fune_model::value::Quantity>,
        #[serde(default)]
        altitude: Option<fune_model::value::Quantity>,
        #[serde(default)]
        min_elevation: Option<fune_model::value::Quantity>,
    },
    /// 地上局を消す（通信の解析の `stations` に挙げられていれば拒む）。
    RemoveGroundStation {
        id: String,
    },
    /// 地上局の名前を変える（通信の解析の `stations` も書き換える）。
    RenameGroundStation {
        id: String,
        to: String,
    },
    /// ハーネス区間を書く（`harness.id` が対象）。
    SetHarness {
        harness: Harness,
    },
    RemoveHarness {
        id: String,
    },
    /// マヌーバを足す（`maneuver.id` が対象。同じ ID があれば拒む）。
    CreateManeuver {
        maneuver: Maneuver,
    },
    /// マヌーバを書く（`maneuver.id` が対象。無ければ作り、あれば変わった行だけを書き換える）。
    SetManeuver {
        maneuver: Maneuver,
    },
    /// マヌーバを消す（ほかのマヌーバの `after` が指していれば、消したマヌーバの `after` につなぎ直す）。
    RemoveManeuver {
        id: String,
    },
    /// 要求を足す（`requirement.id` が対象。同じ ID があれば拒む）。
    CreateRequirement {
        requirement: Requirement,
    },
    /// 要求を書く（`requirement.id` が対象。無ければ作り、あれば丸ごと置き換える）。
    SetRequirement {
        requirement: Requirement,
    },
    /// 要求を消す。
    RemoveRequirement {
        id: String,
    },
    // ---- 図の配置（見た目だけ）
    /// 図の配置を一括で書く（`view/<view>.toml`。挙げたノードだけを書き換え、`remove` は消す）。
    SetViewLayout {
        view: String,
        #[serde(default)]
        nodes: Vec<ViewNodePos>,
        #[serde(default)]
        remove: Vec<String>,
    },
    /// 線の経路と表示上の端点側を書く。
    SetViewEdge {
        view: String,
        key: String,
        edge: fune_model::design::ViewEdge,
    },
    /// 線を自動経路に戻す。
    UnsetViewEdge {
        view: String,
        key: String,
    },
    /// 図の全ノード位置・経路をリセットする。
    ResetView {
        view: String,
    },
    // ---- 設定
    /// `conditions/`・`analysis/`・`budget/` の値（`path` は `analysis.power.dod_max` の形。None なら消す）。
    SetSetting {
        path: String,
        #[serde(default)]
        value: Option<SettingValue>,
    },
}

/// 図のノード 1 つの位置と大きさ（画面の整数座標）。`id` は部品・グループ・ネットの ID。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ViewNodePos {
    pub id: String,
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub w: Option<i32>,
    #[serde(default)]
    pub h: Option<i32>,
}

/// モード表の 1 セル。`state` が None なら行を消す。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct ModeCell {
    pub mode: String,
    pub part: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub duty: BTreeMap<String, f64>,
    #[serde(default)]
    pub active: Option<u32>,
    #[serde(default)]
    pub spare_state: Option<String>,
    #[serde(default)]
    pub eclipse: bool,
}

/// モードの移動先。相対指定は CLI と GUI とで同じ意味を持つ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ModePosition {
    /// 指定モードの直前。
    Before(String),
    /// 指定モードの直後。
    After(String),
    /// 先頭。
    First,
    /// 末尾。
    Last,
}
