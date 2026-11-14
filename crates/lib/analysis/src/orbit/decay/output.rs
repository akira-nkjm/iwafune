//! 高度低下の解析の出力の型（条件ごとの結果・軌道維持・使った値）。

use super::weather::{DecayCondition, DecayWeatherSummary};
use crate::spec::Series;
use sat_astro::atmosphere::space_weather::DatedSpaceWeather;
use sat_astro::time::Epoch;
use sat_units::typed::{Area, Length, Mass, MassDensity, Qty, Time, Velocity};
use serde::{Deserialize, Serialize};

/// 抗力の面積の出所。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DragAreaSource {
    /// 入力で与えた面積
    Given,
    /// 機体の外形の箱の、向きをならした平均の投影面積（表面積の 1/4）
    TumblingBox,
}

/// 使った値（既定値を含む）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DecayUsed {
    /// 抗力係数 Cd
    pub drag_coefficient: f64,
    /// 抗力の面積
    pub drag_area: Qty<Area>,
    /// 面積の出所
    pub area_source: DragAreaSource,
    /// 積分の期間の上限
    pub span: Qty<Time>,
    /// 再突入とみなす高度
    pub reentry_altitude: Qty<Length>,
    /// 軌道維持の ΔV を積算した期間（ミッション期間か 1 年）
    pub station_keeping_span: Qty<Time>,
    /// 軌道維持の高度の帯（無ければ連続して打ち消す）
    pub band: Option<Qty<Length>>,
    /// 軌道維持に使える推薬（推進系の解析 `propulsion.propellant` の値から。推進系が無いか、軌道維持のスラスタを決められなければ `None`）
    pub propellant: Option<Qty<Mass>>,
    /// 比推力を取ったスラスタ（軌道維持のマヌーバのスラスタ、無ければ推薬が 1 種類のときの最も低い比推力。決められなければ `None`）
    pub thruster: Option<String>,
    /// 使える推薬をすべて使ったときの ΔV（ロケット方程式。`propellant` が無ければ `None`）
    pub propulsion_delta_v: Option<Qty<Velocity>>,
    /// 1 周の標本の数
    pub orbit_samples: u32,
    /// 積分の刻みの上限
    pub max_step: Qty<Time>,
    /// 宇宙天気の表から引いた間隔（日付で引いた列が無ければ `None`）
    pub weather_step: Option<Qty<Time>>,
}

/// 太陽活動の条件 1 つの結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DecayCase {
    /// 条件の ID（`low`・`moderate`・`high`・`dated`）
    pub id: String,
    /// 太陽活動の条件
    pub condition: DecayCondition,
    /// 元期の宇宙天気（F10.7・Ap と出所）
    pub initial_weather: DatedSpaceWeather,
    /// 日付で引いた列の出所の数（プリセットの条件は `None`）
    pub weather_sources: Option<DecayWeatherSummary>,
    /// 元期の周回平均の密度
    pub initial_density: Qty<MassDensity>,
    /// 元期の高度低下の速さ（正が低下）
    pub initial_decay_rate: Qty<Velocity>,
    /// 元期の 1 日あたりの高度低下
    pub decay_per_day: Qty<Length>,
    /// 最初の 1 年の高度低下（それより前に再突入すれば再突入までの低下）
    pub decay_first_year: Qty<Length>,
    /// 軌道寿命（期間の上限までに再突入しなければ `None`）
    pub lifetime: Option<Qty<Time>>,
    /// 再突入の日時
    pub reentry_epoch: Option<Epoch>,
    /// 最後の平均の高度（再突入したら近地点が再突入の高度に達したとき、しなければ期間の上限のとき）
    pub final_altitude: Qty<Length>,
    /// 高度を保つときの ΔV と推薬の寿命
    pub station_keeping: StationKeeping,
    /// 高度の推移
    pub series: Series,
}

/// 初期の高度を保つ ΔV（軌道維持。#1 の ΔV バジェットが `delta_v_per_year` を使える）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StationKeeping {
    /// 1 年あたりの ΔV
    pub delta_v_per_year: Qty<Velocity>,
    /// 積算した期間全体の ΔV
    pub delta_v_total: Qty<Velocity>,
    /// 帯の下端で高度を上げた回数（帯が無ければ `None`）
    pub burns: Option<u32>,
    /// 上げる間隔の平均
    pub burn_interval: Option<Qty<Time>>,
    /// 1 回あたりの ΔV の平均
    pub delta_v_per_burn: Option<Qty<Velocity>>,
    /// 推薬で高度を保てる期間（推進系が無ければ `None`）
    pub propellant_life: Option<Qty<Time>>,
}
