//! Op の型。編集の唯一の入口。CLI のコマンドも GUI の操作もこれに変換する。
//!
//! JSON では `{ "op": "set_value", ... }` の形（`op` が種類）。

use fune_model::design::{
    AttributeDef, Case, CatalogField, Harness, KindDef, Maturity, Net, NetField, PartField, Pose,
    Source,
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
    /// `catalog` か `kind` のどちらかが要る。モードには `mode_state`（既定は種別の `off`）で 1 行ずつ足す。
    CreatePart {
        id: String,
        #[serde(default)]
        group: Option<String>,
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
    /// 部品を消す（モードの行と配分も消す）。取付・ネット・ハーネスから参照されていれば拒む。
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
    MovePart {
        id: String,
        #[serde(default)]
        to: Option<String>,
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
    // ---- 定義
    /// 属性定義を足す（次元と許可単位を検査する）。
    CreateAttribute {
        def: AttributeDef,
    },
    /// 種別定義を足す。
    CreateKind {
        def: KindDef,
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
    // ---- 解析ケース・出典・ハーネス（無ければ作り、あれば変わった行だけを書き換える）
    /// 解析ケースを書く（`case.id` が対象）。
    SetCase {
        case: Case,
    },
    RemoveCase {
        id: String,
    },
    /// 出典を書く（`source.id` が対象）。値から参照されている出典は消せない（検証で拒む）。
    SetSource {
        source: Source,
    },
    RemoveSource {
        id: String,
    },
    /// ハーネス区間を書く（`harness.id` が対象）。
    SetHarness {
        harness: Harness,
    },
    RemoveHarness {
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
