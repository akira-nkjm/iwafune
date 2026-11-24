//! 前提条件の入力（`conditions/`）: 軌道・地上局・打ち上げ条件。serde の形がそのままファイルの形。
//!
//! 値はすべて任意（`Option`）。足りない入力は projection が `conditions.orbit.altitude` のようなパスで列挙する。

use super::configuration::Configuration;
use crate::enum_text::enum_text;
use crate::id::Id;
use crate::value::{Quantity, Vector3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// `conditions/` 全体。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
pub struct ConditionsSettings {
    /// `conditions/orbit.toml`
    pub orbit: Option<OrbitSettings>,
    /// `conditions/ground/<ID>.toml`
    pub ground: BTreeMap<String, GroundStationSettings>,
    /// `conditions/launch.toml`
    pub launch: Option<LaunchSettings>,
}

enum_text! {
    /// 軌道の伝搬方法。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
    #[ts(rename = "OrbitPropagatorKind")]
    pub enum PropagatorKind: "orbit propagation method" {
        Kepler = "kepler",
        J2 = "j2",
        Numerical = "numerical",
    }
}

enum_text! {
    /// 軌道の出どころ（`[orbit]` の `source`。書かなければ `elements`）。解析が軌道から読むのは時刻での状態・中心天体・元期・
    /// 繰り返しの長さで、出どころごとに射影（`fune_projection::Context::trajectory`）がそれを作る。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
    pub enum OrbitSource: "orbit source" {
        /// ケプラー要素（`altitude`・`semi_major_axis` 以下。既定）
        Elements = "elements",
        /// 日付つきの遷移のマヌーバ（`maneuver`）の巡航の軌道。元期は出発、中心は共通の親、期間は飛行時間
        Transfer = "transfer",
        /// 状態ベクトル（`epoch`・`central`・`position`・`velocity`・`span`）を多体の力で数値伝搬する軌道。位置・速度は
        /// 原点の天体の中心から、向きは ECI（J2000 平均赤道の慣性系）
        State = "state",
        /// ラグランジュ点（`primary`・`secondary`・`point`・`epoch`）、`amplitude_z` を書けばそのまわりのハロー軌道（`branch`）。
        /// 円制限三体問題の軌道を暦の 2 天体の位置に載せたもの。原点は副天体
        Libration = "libration",
    }
}

enum_text! {
    /// ラグランジュ点（`source = "libration"` の `point`）。`sat_astro::orbit::cr3bp::LibrationPoint` に射影する。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
    pub enum LibrationPointKind: "libration point" {
        L1 = "l1",
        L2 = "l2",
        L3 = "l3",
        L4 = "l4",
        L5 = "l5",
    }
}

enum_text! {
    /// ハロー軌道の枝（`source = "libration"` の `branch`）。北は元期に z > 0、南はその z → −z の鏡像。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
    pub enum HaloBranchKind: "halo branch" {
        North = "north",
        South = "south",
    }
}

/// 状態ベクトルの軌道（`source = "state"`）だけが使う欄。
const STATE_KEYS: &[&str] = &["position", "velocity", "span"];

/// ラグランジュ点の軌道（`source = "libration"`）だけが使う欄（`epoch` は状態ベクトルの軌道も使う）。
const LIBRATION_KEYS: &[&str] = &["primary", "secondary", "point", "amplitude_z", "branch"];

/// ケプラー要素の軌道（`source = "elements"`）だけが使う欄（`epoch`・`central` は状態ベクトルの軌道も使う）。
const ELEMENT_ONLY_KEYS: &[&str] = &[
    "propagator",
    "altitude",
    "semi_major_axis",
    "eccentricity",
    "inclination",
    "sun_synchronous",
    "ltdn",
    "raan",
    "arg_perigee",
    "mean_anomaly",
    "eclipse_season",
    "decay",
];

impl OrbitSource {
    /// 設定の表（設計の軌道・フェーズの `[orbit]`）に、まだ書かれていなくても欄として出すキー（編集の入口）。
    /// `source` は常に。既定の出どころ（`elements`）と `transfer` は、遷移への切り替えに備えて `maneuver` も出す。
    /// `state` は、状態ベクトルの軌道が使う欄（`unused_keys` の補集合）を出し、ケプラー要素の欄は出さない。
    /// **どの出どころで何を出すかはここだけ**（Web は API の返した欄を並べるだけで、出どころごとの欄の一覧を持たない）。
    pub fn offered_keys(self) -> Vec<&'static str> {
        match self {
            Self::Elements | Self::Transfer => vec!["source", "maneuver"],
            Self::State => {
                let unused = self.unused_keys();
                ["source", "epoch", "central", "position", "velocity", "span"]
                    .into_iter()
                    .filter(|k| !unused.contains(k))
                    .collect()
            }
            Self::Libration => {
                let unused = self.unused_keys();
                ["source", "epoch"]
                    .into_iter()
                    .chain(LIBRATION_KEYS.iter().copied())
                    .filter(|k| !unused.contains(k))
                    .collect()
            }
        }
    }

    /// この出どころでは使わない欄（書かれていても解析に効かない。検査が `setting.not_applied` で知らせる）。
    /// 出どころを足すときは、その出どころの欄をほかの出どころの「使わない欄」に足す。
    pub fn unused_keys(self) -> Vec<&'static str> {
        let (elements, shared): (&[&str], &[&str]) = (ELEMENT_ONLY_KEYS, &["epoch", "central"]);
        match self {
            Self::Elements => ["maneuver"]
                .iter()
                .chain(STATE_KEYS)
                .chain(LIBRATION_KEYS)
                .copied()
                .collect(),
            Self::Transfer => shared
                .iter()
                .chain(elements)
                .chain(STATE_KEYS)
                .chain(LIBRATION_KEYS)
                .copied()
                .collect(),
            Self::State => ["maneuver"]
                .iter()
                .chain(elements)
                .chain(LIBRATION_KEYS)
                .copied()
                .collect(),
            // 原点の天体（`central`）は副天体で決まるので書かせない
            Self::Libration => ["maneuver", "central"]
                .iter()
                .chain(elements)
                .chain(STATE_KEYS)
                .copied()
                .collect(),
        }
    }
}

enum_text! {
    /// 軌道の中心天体（`sat_astro::ephem::CentralBody` に射影する。μ・半径・環境の値は sat-astro と解析が持つ）。
    /// マヌーバの目標の天体（`target.body`）にも使う。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
    pub enum CentralBodyKind: "central body" {
        Earth = "earth",
        Moon = "moon",
        Sun = "sun",
        Mercury = "mercury",
        Venus = "venus",
        Mars = "mars",
        Jupiter = "jupiter",
        Saturn = "saturn",
        Uranus = "uranus",
        Neptune = "neptune",
    }
}

impl CentralBodyKind {
    /// 暦で回っている先の天体（月は地球、太陽以外の惑星は太陽。太陽は無し）。ラグランジュ点の軌道の主天体と副天体の組の検査に使う。
    /// `sat_astro::ephem::CentralBody::parent` と同じ（projection のテストで一致を確かめる。model は sat-astro に依存しない）。
    pub fn parent(self) -> Option<Self> {
        match self {
            Self::Sun => None,
            Self::Moon => Some(Self::Earth),
            _ => Some(Self::Sun),
        }
    }
}

/// 軌道（`conditions/orbit.toml`）。円軌道は `altitude`、一般の軌道は `semi_major_axis` 以下の要素で書く。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OrbitSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// 軌道の出どころ（省略時は `elements`。`elements` は書かない）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<OrbitSource>,
    /// `source = "transfer"` の巡航の軌道を取るマヌーバ（`maneuvers/<ID>.toml`。日付つきの `[transfer]` を持つもの）。
    /// 元期は出発の日、中心は共通の親、期間は飛行時間
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maneuver: Option<String>,
    /// 元期（ISO 8601、UTC。例: `"2027-04-01T00:00:00Z"`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epoch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub propagator: Option<PropagatorKind>,
    /// 中心天体（省略時は地球）。高度はこの天体の赤道半径から数える。`source = "state"` では位置・速度の原点の天体
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub central: Option<CentralBodyKind>,
    /// `source = "libration"` の主天体（副天体の親。太陽–地球なら `sun`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<CentralBodyKind>,
    /// `source = "libration"` の副天体（位置・速度の原点の天体。太陽–地球なら `earth`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary: Option<CentralBodyKind>,
    /// `source = "libration"` のラグランジュ点（`l1`〜`l5`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point: Option<LibrationPointKind>,
    /// `source = "libration"` のハロー軌道の z 方向の振幅（長さ。書けばハロー軌道、書かなければ点そのもの。`l1`・`l2` だけ）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amplitude_z: Option<Quantity>,
    /// `source = "libration"` のハロー軌道の枝（`north`・`south`。`amplitude_z` を書いたときだけ。省略は北）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<HaloBranchKind>,
    /// `source = "state"` の元期の位置（`{ x, y, z, unit }`。原点の天体の中心から。向きは ECI = J2000 平均赤道の慣性系。座標系を `frame` で選ぶことはできない）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Vector3>,
    /// `source = "state"` の元期の速度（`{ x, y, z, unit }`。位置と同じ座標系）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub velocity: Option<Vector3>,
    /// `source = "state"` の期間（時間。元期から。解析はこの期間を評価する）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<Quantity>,
    /// 円軌道の高度
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub altitude: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semi_major_axis: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eccentricity: Option<f64>,
    /// 傾斜角（太陽同期なら省略でき、高度から求める）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inclination: Option<Quantity>,
    /// 太陽同期軌道か
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sun_synchronous: Option<bool>,
    /// 降交点の地方時（`"10:30"`）。RAAN の代わりに書ける
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ltdn: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raan: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arg_perigee: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_anomaly: Option<Quantity>,
    /// **旧い設定（ミッション期間は要求へ移った。読まない）**。ミッション期間は、`input = "mission_duration"` の
    /// `at_least` の要求が持つ（`Design::mission_duration`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<Quantity>,
    /// 軌道の季節解析の設定。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eclipse_season: Option<EclipseSeasonSettings>,
    /// 大気抵抗による高度低下と軌道維持の設定（`[decay]`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decay: Option<OrbitDecaySettings>,
}

/// 季節解析の設定（刻みの既定値は解析が持つ）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct EclipseSeasonSettings {
    /// 季節の標本の間隔（時間）。
    #[serde(default)]
    pub step: Option<Quantity>,
}

/// 高度低下と軌道維持の設定（`conditions/orbit.toml` の `[decay]`）。どれも任意で、既定値は解析が持つ。
/// 抗力係数は `analysis.aocs.drag_coefficient`、質量は部品、外形は構体の `size` から取る。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OrbitDecaySettings {
    /// 積分の期間の上限（時間）。
    #[serde(default)]
    pub span: Option<Quantity>,
    /// 再突入とみなす高度（長さ）。
    #[serde(default)]
    pub reentry_altitude: Option<Quantity>,
    /// 抗力の面積（面積。無ければ構体の外形の箱を向きでならした面積）。
    #[serde(default)]
    pub drag_area: Option<Quantity>,
    /// 軌道維持の高度の帯（長さ。無ければ抗力を連続して打ち消す）。
    #[serde(default)]
    pub band: Option<Quantity>,
    /// **旧い設定（基準は要求へ移った。読まない）**。寿命の上限（廃棄の要求）は、`verify.analysis = "orbit.decay"`・
    /// `verify.check = "max_lifetime"` の `at_most` の要求が持つ。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_lifetime: Option<Quantity>,
    /// 以前の軌道維持の推薬（質量）。**使わない**（軌道維持の推薬は推進系の解析 `propulsion.propellant` の使える推薬）。
    /// 古い設計を読めるように残し、書かれていれば `check` が外すよう warning（`setting.moved`）を出す。新しく書かない。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub propellant: Option<Quantity>,
}

/// 地上局（`conditions/ground/<ID>.toml`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct GroundStationSettings {
    /// ファイル名から決まる
    #[serde(default)]
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latitude: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub longitude: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub altitude: Option<Quantity>,
    /// 運用できる最低仰角
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_elevation: Option<Quantity>,
    /// 帯域ごとの地上側の性能（`[link.uhf]`・`[link.s_band]`）
    #[serde(default, rename = "link", skip_serializing_if = "BTreeMap::is_empty")]
    pub links: BTreeMap<String, GroundLinkSettings>,
}

/// 地上局の 1 つの帯域の性能。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct GroundLinkSettings {
    /// アンテナ利得（dBi）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub antenna_gain: Option<Quantity>,
    /// 受信系の雑音温度
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise_temperature: Option<Quantity>,
    /// G/T（dB/K。sat-units は `dB/K` を式にできないので単位は `dB` と書く）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub g_over_t: Option<Quantity>,
    /// アップリンクの送信電力
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_power: Option<Quantity>,
    /// 給電線などの損失（dB）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_loss: Option<Quantity>,
}

/// 打ち上げ条件（`conditions/launch.toml`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct LaunchSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vehicle: Option<String>,
    /// 打ち上げ機のカタログ品（部品の `catalog` と同じ `<カタログ>/<ID>` の形。種別は `launcher`）。
    /// ここに書かない荷重などは、この品の値（ディスペンサーがあればその値が先）から取る
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launcher: Option<String>,
    /// ディスペンサーのカタログ品（同じ形。種別は `dispenser`）。打ち上げ機より衛星に近い接点なので、同じ属性があれば打ち上げ機より先に使う
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispenser: Option<String>,
    /// 打ち上げの解析が使う形態。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration: Option<Id<Configuration>>,
    /// 準静的荷重（機軸方向。単位は `g0`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_axial: Option<Quantity>,
    /// 準静的荷重（機軸に直交する方向）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_lateral: Option<Quantity>,
    /// **旧い設定（基準は要求へ移った。読まない）**。最低固有振動数（機軸方向）は、`verify.analysis = "structure.modal_estimate"`・
    /// `verify.check = "axial"` の `at_least` の要求が持つ
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_frequency_axial: Option<Quantity>,
    /// **旧い設定（基準は要求へ移った。読まない）**。最低固有振動数（直交方向）は、`verify.check = "lateral"` の要求が持つ
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_frequency_lateral: Option<Quantity>,
    /// 打ち上げ時の拘束の種類（`structure.modal_fem`。語彙は属性 `launch_constraint` の `symbols`）。
    /// 書けばディスペンサー・打ち上げ機の属性より先に使う
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_constraint: Option<String>,
    /// 拘束する面（`base_fixed`・`tabs`・`fixed` が使う。語彙は属性 `launch_constraint_face` の `symbols`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_constraint_face: Option<String>,
    /// 安全率（降伏）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety_factor_yield: Option<f64>,
    /// 安全率（終局）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety_factor_ultimate: Option<f64>,
}

impl LaunchSettings {
    /// 欄の名前 → 値の形の見本（文字列で書く欄。数に見える ID も文字列として読むための型で、既定値ではない。
    /// `fune set conditions.launch.<欄>` が使う）。
    pub fn field_shape(key: &str) -> Option<crate::value::SettingValue> {
        match key {
            "label"
            | "vehicle"
            | "launcher"
            | "dispenser"
            | "configuration"
            | "launch_constraint"
            | "launch_constraint_face" => Some(crate::value::SettingValue::Text(String::new())),
            _ => None,
        }
    }
}

enum_text! {
    /// 前提条件の解析の話題。解析ライブラリとの綴りは projection で照合する。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
    pub enum ConditionTopic: "condition topic" {
        Orbit = "orbit",
    }
}

impl ConditionTopic {
    /// 話題の設定ファイル全体のパス（その中に解析ごとの表を置く）。
    pub fn setting_path(self) -> crate::path::SettingPath {
        crate::path::SettingPath::conditions(&[self.as_str()])
    }

    /// 日本語の表示名。
    pub fn label(self) -> &'static str {
        match self {
            Self::Orbit => "軌道",
        }
    }

    /// 英語の表示名。
    pub fn label_en(self) -> &'static str {
        match self {
            Self::Orbit => "Orbit",
        }
    }
}

impl OrbitSettings {
    /// 効く出どころ（書かれていなければ `elements`）。**出どころの既定はここだけ。**
    pub fn source_kind(&self) -> OrbitSource {
        self.source.unwrap_or(OrbitSource::Elements)
    }

    /// 書かれている欄のうち、出どころが使わない欄（`OrbitSource::unused_keys`）。
    pub fn unused_keys(&self) -> Vec<&'static str> {
        let written = serde_json::to_value(self).unwrap_or_default();
        let unused = self.source_kind().unused_keys();
        unused
            .iter()
            .copied()
            .filter(|k| written.get(k).is_some_and(|v| !v.is_null()))
            .collect()
    }

    /// 欄の名前 → 値の形の見本（文字列で書く欄と、ベクトルの欄。数値に見える文字列も文字列として、`"1 0 0 km"` はベクトルとして
    /// 読むための型で、既定値ではない。まだ書かれていない欄を文字列から読むとき（`fune set conditions.orbit.<欄>`・
    /// `fune phase set-field`）に使う）。
    pub fn field_shape(key: &str) -> Option<crate::value::SettingValue> {
        use crate::value::SettingValue;
        match key {
            "label" | "source" | "maneuver" | "epoch" | "ltdn" | "central" | "propagator"
            | "primary" | "secondary" | "point" | "branch" => {
                Some(SettingValue::Text(String::new()))
            }
            "position" | "velocity" => Some(SettingValue::Vector(Vector3::unitless(0.0, 0.0, 0.0))),
            _ => None,
        }
    }
}
