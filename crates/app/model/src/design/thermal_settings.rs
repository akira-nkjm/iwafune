//! 熱の解析の設定（`analysis/thermal.toml`）。serde の形がそのままファイルの形。値はすべて任意で、既定値は解析が持つ。
//!
//! 部品・モード・表面材・節点の参照（`nodes` の値・`surface` と `heat_node` のキーと値・`case.<条件>.mode`）は
//! `check` が検査し、部品の名前の変更（RenamePart）で一緒に書き換わる（`refs`）。

use crate::enum_text::enum_text;
use crate::value::Quantity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

enum_text! {
    /// 最悪ケースのプリセット（`sat_analysis::thermal::ThermalPreset` に射影する。値は解析が持つ）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
    pub enum ThermalPresetKind: "thermal worst-case preset" {
        Hot = "hot",
        Cold = "cold",
        SafeMode = "safe_mode",
    }
}

/// 熱の環境条件 1 つ（`[case.<名前>]`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ThermalCaseSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// 最悪ケースのプリセット（書かれていない太陽光・アルベド・中心天体の赤外に解析の値を使う）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<ThermalPresetKind>,
    /// この条件で使うモード
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solar_flux: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub albedo: Option<f64>,
    /// 中心天体の一律の赤外放射（古い名前 `earth_ir` も読む。store が warning にし、`fune fmt` が書き換える）
    #[serde(default, alias = "earth_ir", skip_serializing_if = "Option::is_none")]
    pub planet_ir: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beta: Option<Quantity>,
}

/// 解析マージンの方針（`[margin]`）。温度差は K で書く。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ThermalMarginSettings {
    /// 予測温度の不確かさ（最高に足し、最低から引いて温度範囲と比べる）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty: Option<Quantity>,
    /// 範囲の端からこの幅以内を警告にする
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning_band: Option<Quantity>,
}

/// 表面材 1 つ（`[coating.<名前>]`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ThermalCoatingSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// 太陽光の吸収率 α（0〜1）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absorptivity: Option<f64>,
    /// 赤外の放射率 ε（0〜1）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emissivity: Option<f64>,
}

/// 熱の解析（`analysis/thermal.toml`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ThermalSettings {
    /// 取付ごとの既定の接触熱コンダクタンス
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_conductance: Option<Quantity>,
    /// 節点間の既定の輻射結合（ε と面積の積）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiative_coupling: Option<Quantity>,
    /// 非定常解析の初期温度
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_temperature: Option<Quantity>,
    /// 非定常解析の時間刻み
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_step: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orbits: Option<u32>,
    /// 解析マージンの方針
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin: Option<ThermalMarginSettings>,
    /// 環境条件の名前 → 条件
    #[serde(default, rename = "case", skip_serializing_if = "BTreeMap::is_empty")]
    pub cases: BTreeMap<String, ThermalCaseSettings>,
    /// 節点の名前 → まとめる部品の ID
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub nodes: BTreeMap<String, Vec<String>>,
    /// 表面材の名前 → α/ε
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub coating: BTreeMap<String, ThermalCoatingSettings>,
    /// 外面の部品 ID → 表面材の名前 → 面積割合（合計 1。書けば部品の absorptivity・emissivity より優先）
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub surface: BTreeMap<String, BTreeMap<String, f64>>,
    /// 部品 ID → 発熱を置く節点（既定は部品の属する節点）
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub heat_node: BTreeMap<String, String>,
}
