//! 編集の API の型（`POST /ops`、`fune apply` と編集系のコマンド）。

use super::{DiagnosticView, ProjectInfo};
use fune_model::value::{Quaternion, Vector3};
use fune_ops::Op;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Op の一括適用の依頼。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ApplyRequest {
    /// 適用する Op（先頭から順に。全部が通るか、何も書かないか）
    pub ops: Vec<Op>,
    /// 読んだときの revision。現在と違えば競合として何も書かない（省略すると確認しない）
    #[serde(default)]
    pub base_revision: Option<String>,
    /// true なら書き込まずに差分だけ返す
    #[serde(default)]
    pub dry_run: bool,
}

/// Op の一括適用の結果。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ApplyResult {
    /// 適用後の revision（dry_run なら適用前のまま）
    pub revision: String,
    pub previous_revision: String,
    pub dry_run: bool,
    pub changed: Vec<ChangedFile>,
    /// 適用後の検証結果（warning 以下。error があれば書き込まずにエラーになる）
    pub diagnostics: Vec<DiagnosticView>,
    /// 同じ操作をする CLI のコマンド（「CLI としてコピー」用）
    pub cli: Vec<String>,
}

/// 変わったファイル 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ChangedFile {
    /// ルートからの相対パス
    pub path: String,
    pub change: FileChangeKind,
    /// 名前を変えたときの元のパス
    pub from: Option<String>,
    /// unified diff の本文
    pub diff: String,
}

/// ファイルの変わり方。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeKind {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// プロジェクトの新規作成の結果（`fune init`）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct InitResult {
    pub project: ProjectInfo,
    pub created: Vec<String>,
}

/// 機体座標での配置の依頼（`POST /parts/{id}/place_body`、`fune part place <ID> --body-pos …`）。
/// 3D 配置の画面で動かした結果をそのまま送る。service が取付点に対する相対位置・姿勢（`Place` の Op）に直して適用する。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PlaceBodyRequest {
    /// 機体座標での部品の原点（単位付き。例: `{ x: 30, y: 0, z: -100, unit: "mm" }`）
    pub position: Vector3,
    /// 機体座標での姿勢（部品 → 機体、w x y z）。省略すると今の姿勢のまま
    #[serde(default)]
    pub orientation: Option<Quaternion>,
    #[serde(default)]
    pub base_revision: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
}

/// 人が打った値の文字列（`fune set <PATH> <VALUE>`・`fune catalog set <品> <PATH> <VALUE>` の引数、`POST /ops/from_text`）。
/// 値の型（整数・小数・量・真偽・一覧など）は service が属性定義と設定の今の型から決める。CLI も GUI もこれを通す。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct SetText {
    /// カタログ品の ID（`local/rw_10mNms`）。あれば `path` は品の中のパス（`mass`・`power.on`・`label`）か、
    /// 品のパラメーターパス（`local/rw_10mNms.power.on`。`CatalogItemView.paths` の値）
    #[serde(default)]
    pub item: Option<String>,
    /// `RW_X.mass`・`RW_X.power.on`・`RW_X.label`・`analysis.power.dod_max`（`item` があれば `mass`・`power.on`・`label`）
    pub path: String,
    /// 打ったままの値（`"620 g"`・`"3"`・`"true"`）
    pub value: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    /// 不確かさの下限・上限（スカラーのとき）
    #[serde(default)]
    pub min: Option<String>,
    #[serde(default)]
    pub max: Option<String>,
}

/// 人が打った消す先のパス（`fune unset <PATH>`・`fune catalog unset <品> <PATH>` の引数、`POST /ops/unset_from_text`）。
/// 属性の値・フィールド・設定のどれを消すかは service がパスから決める。CLI も GUI もこれを通す。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct UnsetText {
    /// カタログ品の ID（`local/rw_10mNms`）。あれば `path` は品の中のパス（`hk_rate`・`power.peak`・`label_en`）か、
    /// 品のパラメーターパス（`local/rw_10mNms.power.peak`。`CatalogItemView.paths` の値）
    #[serde(default)]
    pub item: Option<String>,
    /// `RW_X.mass`・`RW_X.power.on`・`RW_X.label`・`analysis.power.dod_max`（`item` があれば品の中のパス）
    pub path: String,
}
