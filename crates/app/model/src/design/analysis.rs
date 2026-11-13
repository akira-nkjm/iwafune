//! 解析の設定（`analysis/<系>.toml`）。serde の形がそのままファイルの形。
//!
//! 値はすべて任意（`Option`）。足りない入力は projection が `analysis.power.dod_max` のようなパスで列挙する。
//! モードや部品の ID を参照する箇所（`mode_share` のキー・`law` のキー・`nodes` の値など）は `check` が検査し、
//! 名前の変更（RenamePart・RenameMode）で一緒に書き換わる。

use crate::design::thermal_settings::ThermalSettings;
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
    pub enum AnalysisSystem: "解析の系" {
        Power = "power",
        Aocs = "aocs",
        Cdh = "cdh",
        Comm = "comm",
        Thermal = "thermal",
        Structure = "structure",
        Payload = "payload",
        Propulsion = "propulsion",
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
    /// 許容する最大放電深度（0〜1）
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
    /// 軌道平均で各モードにいる時間の割合（モード ID → 割合。合計 1）
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub mode_share: BTreeMap<String, f64>,
    /// 時系列の解析で使うモードの並び（1 周回分。繰り返す）。`power.energy_balance` が並びで回す収支に使う
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sequence: Vec<ModeSegment>,
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
    pub enum AttitudeLawKind: "姿勢則の種類" {
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
    /// 空力の圧力中心 − 重心（機体座標）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cp_offset_aero: Option<Vector3>,
    /// 太陽輻射圧の圧力中心 − 重心（機体座標）
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
    /// 姿勢変更の時間の要求上限（省略時は判定しない）
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
    pub enum LinkDirection: "回線の向き" {
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
}

/// 通信の解析（`analysis/comm.toml`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CommSettings {
    /// 使う地上局（`conditions/ground/<ID>`）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stations: Vec<String>,
    /// 回線余裕の判定のしきい値（dB）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_margin: Option<Quantity>,
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
}
