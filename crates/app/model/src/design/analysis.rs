//! 解析の設定（`analysis/<系>.toml`）。serde の形がそのままファイルの形。
//!
//! 値はすべて任意（`Option`）。足りない入力は projection が `analysis.power.initial_soc` のようなパスで列挙する。
//! **判定の基準（限度・しきい値）は設定に置かず、要求が持つ**（`design::criteria`）。基準だった旧い設定のキーは読み込めるが読まず、
//! 書かれていれば `check` が移し先を知らせる（`MOVED_SETTINGS`）。
//! モードや部品の ID を参照する箇所（`mode_share` のキー・`law` のキー・`nodes` の値など）は `check` が検査し、
//! 名前の変更（RenamePart・RenameMode）で一緒に書き換わる。

use crate::design::acknowledged::AcknowledgedFinding;
use crate::design::modal_settings::ModalSettings;
use crate::design::thermal_settings::ThermalSettings;
use crate::design::timing::StartRef;
use crate::enum_text::enum_text;
use crate::path::SettingPath;
use crate::value::{Quantity, Quaternion, Vector3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

enum_text! {
    /// 解析の系。綴りは設定のファイル名（`analysis/<系>.toml`）・設定のパス（`analysis.<系>`）・解析 ID の接頭辞
    /// （`power.energy_balance`）と同じ。**系の一覧はここだけ**（store のファイルの規約・読み込み・参照の書き換えもこれを回す）。
    /// 解析の側の `sat_analysis::spec::System` は依存の向きで共有できないので、fune-projection（`system`）の対応とテストで一致を確かめる。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
    pub enum AnalysisSystem: "analysis subsystem" {
        Aocs = "aocs",
        Cdh = "cdh",
        Comm = "comm",
        Payload = "payload",
        Power = "power",
        Propulsion = "propulsion",
        Structure = "structure",
        Thermal = "thermal",
    }
}

impl AnalysisSystem {
    /// 系の設定のパス（`analysis.<系>`。`analysis/<系>.toml` の全体）。
    pub fn setting_path(self) -> SettingPath {
        SettingPath::analysis(&[self.as_str()])
    }
}

/// `analysis/` 全体。`common` は共通設定、それ以外の欄は [`AnalysisSystem`] の綴り。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AnalysisSettings {
    pub common: Option<CommonSettings>,
    pub power: Option<PowerSettings>,
    pub aocs: Option<AocsSettings>,
    pub cdh: Option<CdhSettings>,
    pub comm: Option<CommSettings>,
    pub thermal: Option<ThermalSettings>,
    pub structure: Option<StructureSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<PayloadSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub propulsion: Option<PropulsionSettings>,
}

/// ミッション機器の設定。現在は解析も設定項目も持たない。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct PayloadSettings {}

/// 推進系の解析（`analysis/propulsion.toml`）。省略した値の既定は sat-analysis（`propulsion::input`）が持つ。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct PropulsionSettings {
    /// ΔV のマージン（合計 ΔV に対する割合。既定 10 %）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dv_margin: Option<f64>,
    /// 使い切れずに残る推薬の割合（使う推薬に対する割合。既定 3 %）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub residual_fraction: Option<f64>,
    /// 乾燥質量の上書き（省略時は部品の質量特性の総質量）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dry_mass: Option<Quantity>,
    /// 日付の探索（`propulsion.launch_window`）の設定（`[launch_window]`）。どれも任意で、既定値は解析が持つ。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_window: Option<PropulsionLaunchWindow>,
}

/// 日付の探索の設定（`analysis/propulsion.toml` の `[launch_window]`）。どれも任意で、既定値は解析が天体の組から決める
/// （出発の窓 = 2 天体の会合周期、始め = 最初の軌道の元期（日付が書かれていればその出発の日が真ん中）、
/// 飛行時間 = ホーマンの 0.5〜1.5 倍、格子 = 軸ごとに 100 点）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct PropulsionLaunchWindow {
    /// 出発の窓の始め（ISO 8601、UTC。例: `"2028-01-01T00:00:00Z"`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub departure_start: Option<String>,
    /// 出発の窓の長さ（時間）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub departure_span: Option<Quantity>,
    /// 飛行時間の下限（時間）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_of_flight_min: Option<Quantity>,
    /// 飛行時間の上限（時間）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_of_flight_max: Option<Quantity>,
    /// 出発の軸の点の数（2 以上）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub departure_steps: Option<u32>,
    /// 到着の軸の点の数（2 以上）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrival_steps: Option<u32>,
}

/// 系に属さない解析の共通設定（`analysis/common.toml`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CommonSettings {
    /// 解析ケースを明示しなかったときに使うケース ID。
    pub default_case: Option<String>,
}

/// 電源の解析（`analysis/power.toml`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct PowerSettings {
    /// 逆算サイジングの任意の上書き。省略値は sat-analysis が持つ。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sizing: Option<PowerSizingSettings>,
    /// **旧い設定（基準は要求へ移った。読まない）**。許容する最大放電深度は、`verify.analysis = "power.energy_balance"`・
    /// `verify.check = "dod"` の `at_most` の要求が持つ。書かれていれば `check` が `setting.moved_to_requirement` で知らせる
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dod_max: Option<f64>,
    /// 解析開始時の SOC（0〜1）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_soc: Option<f64>,
    /// 太陽電池 → バス → 負荷の変換効率
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversion_efficiency: Option<f64>,
    /// 時系列の解析で回す周回数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orbits: Option<u32>,
    /// 太陽電池の発電に、ほかの部品の影（部品の箱から出す面の日照割合）を数えるか。省略時は数える
    /// （部品に寸法があるとき。`false` で従来の式のまま）。熱の外面は `analysis.thermal.shadowing`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadowing: Option<bool>,
    /// 軌道平均で各モードにいる時間の割合（モード ID → 割合。合計 1）
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub mode_share: BTreeMap<String, f64>,
    /// 時系列の解析で使うモードの並び（1 周回分。繰り返す）。`power.energy_balance` が並びで回す収支に使う。
    /// 名前の無い並び（読み書きの形は従来のまま）で、`default` という名前の並びとして扱う（[`PowerSettings::sequence`]）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sequence: Vec<ModeSegment>,
    /// 名前付きのモードの並び（通常運用・撮像日など。`[sequences.<名前>]`）
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sequences: BTreeMap<String, SequenceSetting>,
    /// 解析に使う並びの名前（省略時は `default`、それも無ければ名前の順で最初の並び）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_sequence: Option<String>,
    /// 最長継続時間の評価で下に敷く基本のモード（モード ID）。省略時は並びの最初の区間のモード
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_mode: Option<String>,
}

/// 電源の逆算設定（`analysis/power.toml` の `[sizing]`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct PowerSizingSettings {
    pub sunlit_efficiency: Option<f64>,
    pub eclipse_efficiency: Option<f64>,
    pub battery_efficiency: Option<f64>,
    pub annual_degradation: Option<f64>,
    pub inherent_degradation: Option<f64>,
    pub battery_strings: Option<u32>,
    pub bus_voltage: Option<Quantity>,
    pub sun_distance: Option<Quantity>,
}

/// 名前付きのモードの並び 1 つ。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SequenceSetting {
    /// 並びの先頭を置く位置（食の出入り + オフセット）。省略時は解析の始め
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<StartRef>,
    /// 区間（1 周期分。繰り返す）
    #[serde(default)]
    pub segments: Vec<ModeSegment>,
}

/// モードの時系列の 1 区間。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ModeSegment {
    pub mode: String,
    pub duration: Quantity,
}

enum_text! {
    /// 姿勢則の種類（sat_astro::attitude::AttitudeLaw に射影する）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
    pub enum AttitudeLawKind: "attitude law kind" {
        SunPointing = "sun_pointing",
        NadirPointing = "nadir_pointing",
        Inertial = "inertial",
    }
}

/// モードごとの姿勢則。軸は機体座標の単位ベクトル。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AttitudeLawSetting {
    pub law: AttitudeLawKind,
    /// 太陽（sun_pointing）・地心（nadir_pointing）に向ける軸
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<Vector3>,
    /// sun_pointing で軌道面の法線に合わせる軸
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary: Option<Vector3>,
    /// nadir_pointing で速度方向に合わせる軸
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub along_track: Option<Vector3>,
    /// inertial の機体 → ECI の回転
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<Quaternion>,
}

impl AttitudeLawSetting {
    /// 欄の名前 → 値の形の見本（まだ書かれていない欄を文字列から読むときの型だけに使う。既定値ではない）。
    pub fn field_shape(field: &str) -> Option<crate::value::SettingValue> {
        use crate::value::SettingValue;
        match field {
            "axis" | "secondary" | "along_track" => {
                Some(SettingValue::Vector(Vector3::unitless(0.0, 0.0, 0.0)))
            }
            "orientation" => Some(SettingValue::Quaternion(Quaternion::IDENTITY)),
            _ => None,
        }
    }
}

/// AOCS の解析（`analysis/aocs.toml`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AocsSettings {
    /// 抗力係数 Cd
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drag_coefficient: Option<f64>,
    /// 太陽輻射圧の反射係数 Cr
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub srp_coefficient: Option<f64>,
    /// 空力の受圧面積（書けば、部品の箱から向きごとに出す面積に代えてこの値を使う）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aero_area: Option<Quantity>,
    /// 太陽輻射圧の受圧面積（書けば、部品の箱から向きごとに出す面積に代えてこの値を使う）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub srp_area: Option<Quantity>,
    /// 空力の圧力中心 − 重心（機体座標。書けば、部品の箱から出す値に代えてこの値を使う）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cp_offset_aero: Option<Vector3>,
    /// 太陽輻射圧の圧力中心 − 重心（機体座標。書けば、部品の箱から出す値に代えてこの値を使う）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cp_offset_srp: Option<Vector3>,
    /// 残留磁気双極子（機体座標）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub residual_dipole: Option<Vector3>,
    /// RW の角運動量のうち使ってよい割合（これを超えたらアンロード）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usable_momentum_fraction: Option<f64>,
    /// MTQ の磁気双極子のうちアンロードに使える割合
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unload_dipole_fraction: Option<f64>,
    /// 姿勢変更（`aocs.slew_time`）の角度（固有軸まわり、180° 以下）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slew_angle: Option<Quantity>,
    /// **旧い設定（基準は要求へ移った。読まない）**。姿勢変更の時間の上限は、`verify.analysis = "aocs.slew_time"`・
    /// `verify.check = "slew_time"` の `at_most` の要求が持つ
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_slew_time: Option<Quantity>,
    /// モード ID → 姿勢則
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub law: BTreeMap<String, AttitudeLawSetting>,
}

/// C&DH の解析（`analysis/cdh.toml`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CdhSettings {
    /// 地上に降ろすまで保持する期間
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention: Option<Quantity>,
    /// ストレージに残す余裕（容量に対する割合）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_margin: Option<f64>,
    /// ミッションデータの圧縮率（圧縮後 / 圧縮前）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compression_ratio: Option<f64>,
    /// HK に掛けるパケットの余分（割合）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packet_overhead: Option<f64>,
}

enum_text! {
    /// 回線の向き。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
    #[ts(rename = "CommLinkDirection")]
    pub enum LinkDirection: "link direction" {
        Down = "down",
        Up = "up",
    }
}

/// 回線 1 本の設定（`[link.<名前>]`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CommLinkSettings {
    pub direction: LinkDirection,
    /// 衛星側の送受信機（部品 ID）
    pub part: String,
    /// 地上局側の帯域（`conditions/ground/<ID>.toml` の `[link.<名前>]`）
    pub band: String,
    /// 衛星側のアンテナ（部品 ID。省略すると送受信機の `antenna_gain` を使う）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub antenna: Option<String>,
    /// 変調方式（表示用）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modulation: Option<String>,
    /// 要求 Eb/N0（dB）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ebn0_required: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pointing_loss: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polarization_loss: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub atmospheric_loss: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation_loss: Option<Quantity>,
    /// 衛星側の給電線の損失
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_loss: Option<Quantity>,
    /// **旧い設定（基準は要求へ移った。読まない）**。地表の電力束密度（PFD）の限度は、`verify.analysis = "comm.link_budget"`・
    /// `verify.check = "pfd:<回線>@<局>"` の `at_most` の要求が持つ（dBW/m²（基準帯域あたり）。単位は `dB` と書く）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pfd_limit: Option<Quantity>,
    /// PFD の基準帯域（Hz。省略すると解析の既定 4 kHz）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pfd_reference_bandwidth: Option<Quantity>,
    /// 信号の占有帯域（Hz。PFD を基準帯域あたりに直すのに使う。省略すると割らない）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occupied_bandwidth: Option<Quantity>,
}

/// 通信の解析（`analysis/comm.toml`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CommSettings {
    /// 使う地上局（`conditions/ground/<ID>`）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stations: Vec<String>,
    /// **旧い設定（基準は要求へ移った。読まない）**。回線余裕の下限は、`verify.analysis = "comm.link_budget"`・
    /// `verify.check = "margin"`（すべての回線 × 局）か `margin:<回線>@<局>` の `at_least` の要求が持つ
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_margin: Option<Quantity>,
    /// 1 日の送信量（`comm.data_throughput`）に数える下り回線の名前（省略すると名前順の先頭の下り回線）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_downlink: Option<String>,
    /// 回線の名前 → 設定
    #[serde(default, rename = "link", skip_serializing_if = "BTreeMap::is_empty")]
    pub links: BTreeMap<String, CommLinkSettings>,
}

/// 材料の物性（`[material.<名前>]`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct MaterialSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub density: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub youngs_modulus: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poisson_ratio: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yield_strength: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ultimate_strength: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thermal_conductivity: Option<Quantity>,
}

/// 構造の解析（`analysis/structure.toml`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct StructureSettings {
    /// 材料の名前（部品の `material` の値）→ 物性
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub material: BTreeMap<String, MaterialSettings>,
    /// 取付部の締結の許容荷重
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fastener_allowable: Option<Quantity>,
    /// 部品の箱の重なりを干渉に数えない深さ（`structure.interference`。書かなければ解析の既定）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interference_tolerance: Option<Quantity>,
    /// 噴流・視野の円錐を見る、頂点からの距離の上限（`structure.cone_clearance`。書かなければ機体の大きさから決める）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cone_clearance_range: Option<Quantity>,
    /// 粗い有限要素の固有値解析（`structure.modal_fem`）の設定
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modal: Option<ModalSettings>,
    /// 承知済みの組（`structure.interference` と `structure.cone_clearance` に共通。`[[acknowledged]]`）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acknowledged: Vec<AcknowledgedFinding>,
}
