//! `propulsion.launch_window` の出力の型。

use super::axes::LaunchWindowSpan;
use sat_astro::ephem::{CentralBody, EphemerisSource};
use sat_astro::time::Epoch;
use sat_units::typed::{Angle, Qty, SpecificEnergy, Time, Velocity};
use serde::{Deserialize, Serialize};

/// 結果の注記。**文は持たない**（service の語彙が引く）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LaunchWindowNote {
    /// 天体の位置が解析式（DE440 でない）。火星で 10⁵ km 級の誤差があり、最小の日付も数日ずれうる
    AnalyticEphemeris,
    /// ΔV が最小の点の出発の赤緯が駐機軌道の傾斜角より大きい（面の変更は数えない。`dated_transfer_plane` と同じ見方）
    DeclinationAboveInclination,
    /// ΔV が最小の点が格子の端（軸の端・飛行時間の範囲の端・解けない点の隣）にある。窓を広げると下がるかもしれない
    MinimumAtGridEdge,
    /// マヌーバに書かれた日付では解けない
    WrittenDatesUnsolved,
}

/// 格子の量の種類（表示名は service の語彙が引く）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LaunchWindowQuantity {
    C3,
    VInfDeparture,
    VInfArrival,
    DepartureDeclination,
    DeltaV,
}

/// 格子の 1 つの量。値は SI で、出発 `i`・到着 `j` の組が `i * arrivals.len() + j` 番目（解けない・範囲外の点は `None`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchWindowLayer {
    pub kind: LaunchWindowQuantity,
    /// 次元の名前（`sat_units::named_dimensions` の名前。型から取る）
    pub dimension: String,
    pub values: Vec<Option<f64>>,
}

/// 格子: 軸（日付）と、軸の組ごとの量（出発の軸が行、到着の軸が列）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchWindowGrid {
    pub departures: Vec<Epoch>,
    pub arrivals: Vec<Epoch>,
    /// 量ごとの値（長さは `departures.len() × arrivals.len()`）
    pub layers: Vec<LaunchWindowLayer>,
}

/// 日付の組が何の点か。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LaunchWindowPointKind {
    /// 格子で ΔV が最小の点
    MinDeltaV,
    /// 格子で C3 が最小の点
    MinC3,
    /// マヌーバに書かれた日付
    Written,
}

/// 日付の組 1 つの結果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchWindowPoint {
    /// マヌーバの ID
    pub maneuver: String,
    pub kind: LaunchWindowPointKind,
    pub departure: Epoch,
    pub arrival: Epoch,
    pub time_of_flight: Qty<Time>,
    /// 出発の特性エネルギー C3
    pub c3: Qty<SpecificEnergy>,
    pub v_inf_departure: Qty<Velocity>,
    pub v_inf_arrival: Qty<Velocity>,
    /// 出発の漸近線の赤緯（出発天体の赤道に対して）
    pub departure_declination: Qty<Angle>,
    /// マヌーバの ΔV（出発の脱出 + 到着の捕獲。損失を除く）
    pub delta_v: Qty<Velocity>,
}

/// マヌーバに書かれた日付での値と、格子の最小との差。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchWindowWritten {
    /// 解けなければ `None`
    pub point: Option<LaunchWindowPoint>,
    /// 格子で ΔV が最小の点との ΔV の差（書かれた日付 − 最小。格子より書かれた日付のほうが良ければ負）
    pub delta_v_above_minimum: Option<Qty<Velocity>>,
    /// 格子で C3 が最小の点との C3 の差
    pub c3_above_minimum: Option<Qty<SpecificEnergy>>,
}

/// マヌーバ 1 つの探索の結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchWindowResult {
    /// マヌーバの ID
    pub maneuver: String,
    pub from: CentralBody,
    pub to: CentralBody,
    /// 探索の範囲（設定と既定値で決まったもの）
    pub span: LaunchWindowSpan,
    /// 2 天体の会合周期・ホーマン遷移の時間（既定の窓と飛行時間の元）
    pub synodic_period: Qty<Time>,
    pub hohmann_time: Qty<Time>,
    /// 格子（軸と量）。汎用の表の画面は入れ子のオブジェクトを出さないので、大きな配列はここに入れる
    pub grid: LaunchWindowGrid,
    /// 飛行時間が範囲に入って解こうとした点の数と、そのうち解けた点の数
    pub evaluated_cells: u32,
    pub solved_cells: u32,
    pub minimum_delta_v: Option<LaunchWindowPoint>,
    pub minimum_c3: Option<LaunchWindowPoint>,
    /// マヌーバに日付が書かれていれば、その値
    pub written: Option<LaunchWindowWritten>,
    /// 天体の位置の出所（解いた点がすべて DE440 のときだけ `de440`）
    pub ephemeris: EphemerisSource,
    pub notes: Vec<LaunchWindowNote>,
}

/// `propulsion.launch_window` の出力。天体間の遷移を持つマヌーバが無ければ `windows` は空。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchWindowOutput {
    pub windows: Vec<LaunchWindowResult>,
    /// 各マヌーバの最小の点と書かれた日付の点を 1 つの表に並べたもの（`windows` の同じ点。汎用の表の画面が行として出す）
    pub points: Vec<LaunchWindowPoint>,
}

impl LaunchWindowResult {
    /// 最小の点（ΔV・C3）と書かれた日付の点（表の行の順）。
    pub(super) fn points(&self) -> impl Iterator<Item = &LaunchWindowPoint> {
        [&self.minimum_delta_v, &self.minimum_c3]
            .into_iter()
            .flatten()
            .chain(self.written.iter().filter_map(|w| w.point.as_ref()))
    }
}
