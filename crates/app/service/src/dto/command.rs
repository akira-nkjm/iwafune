//! 名詞のコマンドに打った引数（文字列のまま）の型（`POST /ops/from_command`）。CLI はコマンドの引数から、GUI は欄に打った
//! 文字列から作り、どちらも service の `edit::command_ops` が同じ読み方で Op にする。パスと値の 1 組は `SetText`。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// 名詞のコマンドの引数（`POST /ops/from_command` の本文）。`command` でコマンドを選ぶ。
/// 書かなかった旗（省略・null）は CLI で旗を書かないのと同じ。読み方（量・ベクトル・割合・一覧・`名前=種類`）は service が持つ。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum CommandText {
    /// `fune net set <ID> --rail "5 V" …`（書いた旗ごとに `SetNetField`）
    NetSet(NetSetText),
    /// `fune net unset <ID> <フィールド>`
    NetUnset(NetUnsetText),
    /// `fune part place <ID> --on … --point … --position "0 0 12 mm" --orientation "1 0 0 0"`（`--root` は `on` を省く）
    PartPlace(PlaceText),
    /// `fune mode set <モード> <部品> <状態> --duty tx=0.1`・`fune mode unset <モード> <部品>`。
    /// GUI のモード表は変えたセルをまとめて送り、1 つの `SetModeStates` になる
    ModeSet(ModeSetText),
    /// `fune mode move <ID> --before|--after|--first|--last`。
    ModeMove {
        id: String,
        #[serde(default)]
        before: Option<String>,
        #[serde(default)]
        after: Option<String>,
        #[serde(default)]
        first: bool,
        #[serde(default)]
        last: bool,
    },
    /// モードの最長継続時間（null なら削除）。
    ModeDuration {
        mode: String,
        duration: Option<String>,
    },
    /// `fune mode timing <モード> --when sunlit|eclipse|any --start eclipse_exit|eclipse_entry --offset "5 min"`。
    /// 旗を 1 つも書かなければ時刻の指定を消す。`offset` は `start` があるときだけ書ける。
    ModeTiming {
        mode: String,
        #[serde(default)]
        when: Option<String>,
        #[serde(default)]
        start: Option<String>,
        #[serde(default)]
        offset: Option<String>,
    },
    /// `fune sequence set <名前> --segments "NOMINAL=76 min,MISSION=5 min" --anchor eclipse_exit --offset "0 min"`
    /// （並びを丸ごと書く。`anchor` が無ければ解析の始めから）。
    SequenceSet {
        name: String,
        /// `モード=長さ` のカンマ区切り（`--duty` と同じ書き方）
        segments: String,
        #[serde(default)]
        anchor: Option<String>,
        #[serde(default)]
        offset: Option<String>,
    },
    /// `fune sequence rm <名前>`。
    SequenceRemove { name: String },
    /// `fune attr add <ID> --dimension … --units …`
    AttrAdd(AttrAddText),
    /// `fune kind add <ID> --states off,on --required mass --port PWR=power`
    KindAdd(KindAddText),
    /// `fune attr replace <ID> …`（`attr add` と同じ旗。既にある属性定義を丸ごと置き換える）
    AttrReplace(AttrAddText),
    /// `fune kind replace <ID> …`（`kind add` と同じ旗。既にある種別定義を丸ごと置き換える）
    KindReplace(KindAddText),
    /// `fune maneuver add <ID> --category … --method … --altitude "600 km" …`（同じ ID があれば拒む）
    ManeuverAdd(super::ManeuverText),
    /// `fune maneuver set <ID> …`（無ければ作り、あれば丸ごと置き換える）
    ManeuverSet(super::ManeuverText),
    /// `fune phase add <ID> --label … --order 1 --duration "30 d" --modes SAFE,NOMINAL`（同じ ID があれば拒む）
    PhaseAdd(super::PhaseText),
    /// `fune phase set <ID> …`（無ければ作り、あれば見出しの欄を置き換える。軌道と姿勢則は今のまま残す）
    PhaseSet(super::PhaseText),
    /// `fune phase set-field <ID> orbit.altitude "550 km"`（`value` が無ければ `unset-field` で消す）
    PhaseField {
        phase: String,
        path: String,
        #[serde(default)]
        value: Option<String>,
    },
    /// `fune phase copy-orbit <ID>`（設計の軌道の今の値をフェーズの `[orbit]` へ丸ごと写す。上書きの出発点）
    PhaseCopyOrbit { phase: String },
    /// `fune phase unset-orbit <ID>`（フェーズの軌道の上書きを丸ごと消す）
    PhaseUnsetOrbit { phase: String },
    /// `fune phase unset-law <ID> <モード>`（そのモードの姿勢則の上書きを丸ごと消す）
    PhaseUnsetLaw { phase: String, mode: String },
    /// `fune case set-phase <ケース> <フェーズ>`（`phase` が無ければ `unset-phase` で外す）
    CasePhase {
        case: String,
        #[serde(default)]
        phase: Option<String>,
    },
    /// `fune case set-epoch <ケース> <時刻>`（`epoch` が無い・空なら `unset-epoch` で外す。時刻の読み方はサービスだけが持つ）
    CaseEpoch {
        case: String,
        #[serde(default)]
        epoch: Option<String>,
    },
    /// `fune configuration add <ID> --label … --label-en … --description …`（同じ ID があれば拒む）
    ConfigurationAdd(super::ConfigurationText),
    /// `fune configuration set <ID> …`（無ければ作り、あれば見出しの欄を置き換える。部品ごとの姿勢は今のまま残す）
    ConfigurationSet(super::ConfigurationText),
    /// `fune ground add <ID> --label … --latitude "35.7 deg" …`（`conditions/ground/<ID>.toml` を作る。同じ ID があれば拒む）
    GroundAdd(super::GroundAddText),
    /// `fune requirement add <ID> --statement … --quantity Mass --kind at_most --value "12 kg" …`（同じ ID があれば拒む）
    RequirementAdd(super::RequirementText),
    /// `fune requirement set <ID> …`（無ければ作り、あれば丸ごと置き換える）
    RequirementSet(super::RequirementText),
    /// `fune catalog source add <名前> --git <URL> [--rev <rev>]`（fune.toml に共有カタログを足す。取得はしない）
    CatalogSourceAdd(CatalogSourceAddText),
    /// `fune catalog source add-local <名前> --path <ディレクトリ>`（fune.toml にこのプロジェクトの中のカタログを足す）
    CatalogLocalAdd(super::CatalogLocalAddText),
    /// `fune link add <ID> --direction down --part STX --band s_band`（`analysis/comm.toml` に回線を足す。同じ名前があれば拒む）
    LinkAdd(super::LinkAddText),
    /// `fune project set <フィールド> <値>`・`fune project unset <フィールド>`（fune.toml の `[project]`）
    ProjectSet(super::ProjectSetText),
}

/// `fune catalog source add` の引数。前後の空白は除き、空の `rev` は省いたのと同じ（既定のブランチ）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(optional_fields = nullable)]
pub struct CatalogSourceAddText {
    /// カタログの名前（品は `<名前>/<ID>` で入る）
    pub name: String,
    /// 取得元の git の URL（https・ssh・file://）
    pub git: String,
    /// ブランチ・タグ・commit
    pub rev: Option<String>,
}

/// `fune net set` の引数。旗を書いたフィールドだけを書く（宣言の順に 1 フィールド 1 つの `SetNetField`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(optional_fields = nullable)]
pub struct NetSetText {
    /// ネットの ID
    pub net: String,
    /// `power` / `data` / `rf`
    pub kind: Option<String>,
    pub label: Option<String>,
    /// 供給元のポート（`PCDU.OUT3`）
    pub source: Option<String>,
    /// レール電圧（`"5 V"`）
    pub rail: Option<String>,
    /// 電流制限（`"2 A"`）
    pub limit: Option<String>,
    pub protocol: Option<String>,
    /// ビットレート（`"1 Mbit/s"`）
    pub bitrate: Option<String>,
    /// バスのマスターのポート（`OBC.CAN1`）
    pub master: Option<String>,
}

/// `fune net unset <ID> <フィールド>` の引数。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct NetUnsetText {
    pub net: String,
    /// `label`・`source`・`rail`・`limit`・`protocol`・`bitrate`・`master`（`fune_model::design::NetFieldName::NAMES`。`kind` は消せない）
    pub field: String,
}

/// `fune part place` の引数（取付と、取付点に対する位置・姿勢を丸ごと置き換える）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(optional_fields = nullable)]
pub struct PlaceText {
    /// 部品 ID
    pub part: String,
    /// 取付先の部品。省略か空なら取付を外して機体の根にする（`--root`）
    pub on: Option<String>,
    /// 取付点（取付先があるときだけ書く）
    pub point: Option<String>,
    /// 取付点に対する位置（`"0 0 12 mm"`。読み方は `edit::vector_from_text`）。省略か空なら書かない
    pub position: Option<String>,
    /// 取付点に対する姿勢のクォータニオン（`"1 0 0 0"`、w x y z。`edit::quaternion_from_text`）。省略か空なら書かない
    pub orientation: Option<String>,
    /// 取付点に対する姿勢の X・Y・Z 軸まわりの回転角（`"0 90 0 deg"`。固定軸で X → Y → Z の順。`edit::rotation_from_text`）。
    /// 省略か空なら書かない。`orientation` と両方は書けない
    #[serde(default)]
    pub rotation: Option<String>,
    /// この形態に姿勢を書く（`--configuration`。取付は変えない。取付先・取付点は書かなくてよく、書くなら部品の取付と同じ）。
    /// 省略か空なら部品自身の取付と姿勢
    #[serde(default)]
    pub configuration: Option<String>,
}

/// モード表のセルの編集（`fune mode set|unset` の引数の列）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct ModeSetText {
    pub cells: Vec<ModeCellText>,
}

/// モード表の 1 セル（`fune mode set <モード> <部品> <状態> --duty …`・`fune mode unset <モード> <部品>`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct ModeCellText {
    pub mode: String,
    pub part: String,
    /// 基本の状態。null ならそのモードの表から部品の行を消す（`fune mode unset`）
    #[serde(default)]
    pub state: Option<String>,
    /// ほかの状態 → 周回のうちその状態で過ごす割合の打った文字列（`"0.1"`）。空の文字列の状態は書かない。
    /// CLI の `--duty tx=0.1,rx=0.2` は `edit::duty_from_text` でこの形にする
    #[serde(default)]
    pub duty: BTreeMap<String, String>,
    #[serde(default)]
    pub active: Option<String>,
    #[serde(default)]
    pub spare_state: Option<String>,
    #[serde(default)]
    pub eclipse: bool,
}

/// `fune attr add` の引数。表示名を省くと ID、値の型を省くと scalar。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(optional_fields = nullable)]
pub struct AttrAddText {
    pub id: String,
    pub label: Option<String>,
    pub label_en: Option<String>,
    pub description: Option<String>,
    /// 値の型（`scalar`・`vec3`・`quat`・`tensor3`・`enum`・`bool`・`text`・`range`・`table`）
    #[serde(rename = "type")]
    pub value_type: Option<String>,
    /// 状態ごとに値を持つか
    pub by_state: bool,
    /// 次元名（`Pressure`）
    pub dimension: Option<String>,
    /// 許可する単位（GUI は選んだ単位。CLI は `--units Pa,kPa` を `edit::list_from_text` で分けたもの）
    pub units: Vec<String>,
    pub display_unit: Option<String>,
    /// 値域の下限・上限（SI の数値の打った文字列。`"-40"`）
    pub min: Option<String>,
    pub max: Option<String>,
    /// 既定値（SI の数値の打った文字列）
    pub default: Option<String>,
    /// 列挙の語彙（CLI は `--symbols a,b` を分けたもの）
    pub symbols: Vec<String>,
    /// どのバジェットに入るか（`mass` / `power` / `data`）
    pub budget: Option<String>,
    /// 方向ベクトル（単位ベクトルであるべきもの）
    pub unit_vector: bool,
    /// 視野の半角のとき、どの方向の属性のまわりの視野か
    pub fov_axis: Option<String>,
    /// 画面で並べる系（`basic`・`mass`・`power`・…）
    pub section: Option<String>,
}

/// `fune kind add` の引数。表示名を省くと ID。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(optional_fields = nullable)]
pub struct KindAddText {
    pub id: String,
    pub label: Option<String>,
    pub label_en: Option<String>,
    pub description: Option<String>,
    pub extends: Option<String>,
    pub role: Option<String>,
    /// 機器の状態の打った一覧（`"off, on"`。読み方は `edit::list_from_text`）
    pub states: Option<String>,
    /// 必須の属性の打った一覧（`"mass, power"`）
    pub required: Option<String>,
    /// 任意の属性の打った一覧
    pub optional: Option<String>,
    /// ポートの雛形（`"PWR=power"` の列。`--port` の繰り返し。読み方は `edit::port_template_from_text`）
    pub ports: Vec<String>,
}

/// 季節解析のケースにする標本。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum EclipseCaseAt {
    Longest,
    BetaMin,
}

/// 太陽距離のケースにする標本（太陽を回る軌道の、ミッション期間の中の最小・最大）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SunDistanceCaseAt {
    Min,
    Max,
}

/// `fune case from-sun-distance` と HTTP が共有する要求。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CaseFromSunDistanceRequest {
    pub id: String,
    pub at: SunDistanceCaseAt,
    #[serde(default)]
    pub mode: Option<String>,
}

/// `fune case from-geometry` と HTTP が共有する要求。フェーズの軌道のミッション幾何（`orbit.geometry` の区間）の、
/// 太陽距離の最大・最小と地球距離の最大の日付から、`epoch` を持つ解析ケースを作る（なければ作り、あれば `epoch` などを更新する）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct CaseFromGeometryRequest {
    /// 読むフェーズ（フェーズが無い設計は省略。フェーズがある設計では必須）。ケースの `phase` にもなる
    #[serde(default)]
    pub phase: Option<String>,
    /// ケース ID の前置き（`<prefix>_sun_far`・`<prefix>_sun_near`・`<prefix>_earth_far`）。省略時はフェーズの ID（小文字）、
    /// フェーズも無ければ前置きなし
    #[serde(default)]
    pub prefix: Option<String>,
    /// ケースのモード（省略すると、新しいケースにはモードを書かず、既にあるケースのモードは変えない）
    #[serde(default)]
    pub mode: Option<String>,
}

/// 日付の探索で選ぶ点（`propulsion.launch_window` の最小の点）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum WindowObjective {
    /// マヌーバの ΔV が最小の点
    Dv,
    /// 出発の C3 が最小の点
    C3,
}

/// `fune maneuver set-window` と HTTP が共有する要求（マヌーバの日付を探索の最小の点の日付にする）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ManeuverWindowRequest {
    /// マヌーバの ID
    pub id: String,
    pub min: WindowObjective,
}

/// `fune case from-eclipse` と HTTP が共有する要求。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CaseFromEclipseRequest {
    pub id: String,
    pub at: EclipseCaseAt,
    #[serde(default)]
    pub mode: Option<String>,
}
