//! 推進系の解析の共通の入力と列挙。モデル化の既定値もここに置く。

use crate::aocs::RwUnloadingInput;
use crate::spec::{AnalysisError, ensure_fraction, ensure_positive};
use sat_astro::orbit::OrbitSpec;
use sat_units::typed::{
    Angle, Force, Fraction, Impulse, Length, Mass, MassDensity, Power, Qty, Time, Volume,
};
use serde::{Deserialize, Serialize};

/// ΔV のマージンの既定値（合計 ΔV に対する割合）。概念設計の ΔV の内訳表に 5〜10 % の余裕を見込む
/// 概念設計の慣例の保守側。`analysis.propulsion.dv_margin` で変える。
pub const DEFAULT_DV_MARGIN: f64 = 0.10;

/// 使い切れずに残る推薬の既定の割合（使う推薬に対する割合。配管・タンクに残る分）。
/// 残留に 2〜3 % を見込む慣例の保守側。`analysis.propulsion.residual_fraction` で変える。
pub const DEFAULT_RESIDUAL_FRACTION: f64 = 0.03;

/// タンクの容積の余裕がこれ未満なら警告（容積の比の余裕 `ratio_margin`）。
pub const TANK_MARGIN_WARNING: f64 = 0.10;

/// 定格（総インパルス・累積噴射時間・噴射回数）に対する余裕がこれ未満なら警告。
pub const RATING_MARGIN_WARNING: f64 = 0.25;

/// ΔV の区分（内訳表の行）。綴りは設計データの `category` と同じ（projection の対応とテストで一致を確かめる）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DvCategory {
    /// 軌道投入・軌道上昇
    Insertion,
    /// 軌道維持
    StationKeeping,
    /// RW のアンロード
    Unloading,
    /// 廃棄（デオービット）
    Deorbit,
    /// その他
    Other,
}

impl DvCategory {
    /// 内訳表の並び。
    pub const ALL: [Self; 5] = [
        Self::Insertion,
        Self::StationKeeping,
        Self::Unloading,
        Self::Deorbit,
        Self::Other,
    ];
}

/// 遷移の計算の段階。綴りは設計データの `method` と同じ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TransferMethod {
    /// インパルスの噴射（ホーマン・平面変更・バイエリプティック）
    Impulsive,
    /// 化学推進の有限燃焼（インパルスの ΔV に重力・向きの損失の近似を足す）
    FiniteBurn,
    /// 電気推進の連続噴射（Edelbaum の式）
    LowThrust,
}

/// 目標の軌道（変えたい要素だけ。書かない要素は前の軌道のまま）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrbitTarget {
    /// 円軌道の高度（近地点と遠地点の両方）
    #[serde(default)]
    pub altitude: Option<Qty<Length>>,
    /// 近地点の高度
    #[serde(default)]
    pub perigee_altitude: Option<Qty<Length>>,
    /// 遠地点の高度
    #[serde(default)]
    pub apogee_altitude: Option<Qty<Length>>,
    /// 傾斜角
    #[serde(default)]
    pub inclination: Option<Qty<Angle>>,
}

/// マヌーバ 1 つ（実行の順に並べる）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ManeuverInput {
    pub id: String,
    pub category: DvCategory,
    pub method: TransferMethod,
    /// 使うスラスタ（[`PropulsionThruster::id`]）。無ければ ΔV だけを出し、推薬は求めない
    #[serde(default)]
    pub thruster: Option<String>,
    #[serde(default)]
    pub target: OrbitTarget,
    /// バイエリプティック遷移の中間の遠地点の高度（インパルス・有限燃焼のときだけ使う）
    #[serde(default)]
    pub via_altitude: Option<Qty<Length>>,
    /// 書かれているが粗い段階では効かない制約のキー（`sunlit_only`・`mode`。結果に無視したことを書く）
    #[serde(default)]
    pub ignored_constraints: Vec<String>,
}

/// 推進系のスラスタ 1 本。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PropulsionThruster {
    pub id: String,
    pub thrust: Qty<Force>,
    pub isp: Qty<Time>,
    /// 推薬の名前（タンクの `propellant` と突き合わせる）
    #[serde(default)]
    pub propellant: Option<String>,
    /// 最小インパルスビット（表示だけ）
    #[serde(default)]
    pub min_impulse_bit: Option<Qty<Impulse>>,
    /// 定格の総インパルス
    #[serde(default)]
    pub rated_total_impulse: Option<Qty<Impulse>>,
    /// 定格の累積噴射時間
    #[serde(default)]
    pub rated_firing_time: Option<Qty<Time>>,
    /// 定格の噴射回数
    #[serde(default)]
    pub rated_firings: Option<u32>,
    /// 噴射中の消費電力（表示だけ。電力の収支はモードで数える）
    #[serde(default)]
    pub firing_power: Option<Qty<Power>>,
}

impl PropulsionThruster {
    /// 推力と比推力が正か調べ、有効排気速度 [m/s] を返す。
    pub(super) fn exhaust_velocity(&self) -> Result<f64, AnalysisError> {
        ensure_positive(&format!("スラスタ {} の推力", self.id), self.thrust.value())?;
        ensure_positive(&format!("スラスタ {} の比推力", self.id), self.isp.value())?;
        Ok(sat_astro::rocket::exhaust_velocity(self.isp.value()))
    }
}

/// 推薬のタンク 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PropellantTank {
    pub id: String,
    /// タンクの容積（推薬を入れられる容積）
    pub volume: Qty<Volume>,
    /// 推薬の名前
    #[serde(default)]
    pub propellant: Option<String>,
    /// 推薬の密度（無ければ容積の判定をしない）
    #[serde(default)]
    pub density: Option<Qty<MassDensity>>,
}

/// RCS でのアンロードの ΔV を足すための入力（`aocs.rw_unloading` の入力と、日数を掛けるミッション期間）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UnloadingInput {
    pub rw_unloading: RwUnloadingInput,
    pub mission_duration: Qty<Time>,
}

/// アンロードの ΔV を足さなかった理由。**文は持たない**（service の語彙が引く）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum UnloadingNote {
    /// AOCS に RCS のスラスタが無い
    NoRcs,
    /// `aocs.rw_unloading` の入力が足りない
    UnloadingInputsMissing,
    /// ミッション期間が無い
    NoMissionDuration,
    /// RCS では捨てられない（捨てる向きのスラスタが無いなど）
    NotUnloadable,
}

/// 推進系の解析の入力（4 つの解析で共通）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PropulsionInput {
    /// 最初のマヌーバの前の軌道
    pub initial_orbit: OrbitSpec,
    /// 推薬を除いた機体の質量（ドライ）
    pub dry_mass: Qty<Mass>,
    pub thrusters: Vec<PropulsionThruster>,
    pub tanks: Vec<PropellantTank>,
    /// 実行の順に並べたマヌーバ
    pub maneuvers: Vec<ManeuverInput>,
    /// ΔV のマージン（既定 [`DEFAULT_DV_MARGIN`]）
    #[serde(default)]
    pub dv_margin: Option<Fraction>,
    /// 残る推薬の割合（既定 [`DEFAULT_RESIDUAL_FRACTION`]）
    #[serde(default)]
    pub residual_fraction: Option<Fraction>,
    /// RCS でのアンロード（`propulsion.dv_budget` だけが使う）
    #[serde(default)]
    pub unloading: Option<UnloadingInput>,
    /// アンロードを足さない理由（`unloading` が無いとき）
    #[serde(default)]
    pub unloading_note: Option<UnloadingNote>,
}

impl PropulsionInput {
    /// ΔV のマージン（既定値込み）。
    pub fn dv_margin(&self) -> Result<f64, AnalysisError> {
        let m = self.dv_margin.map_or(DEFAULT_DV_MARGIN, Fraction::value);
        crate::spec::ensure_non_negative("ΔV のマージン", m)?;
        Ok(m)
    }

    /// 残る推薬の割合（既定値込み）。
    pub fn residual_fraction(&self) -> Result<f64, AnalysisError> {
        let r = self
            .residual_fraction
            .map_or(DEFAULT_RESIDUAL_FRACTION, Fraction::value);
        ensure_fraction("残る推薬の割合", r)?;
        Ok(r)
    }

    /// マヌーバのスラスタ（無ければ `None`。ID が一覧に無ければ `InvalidInput`）。
    pub(super) fn thruster_of(
        &self,
        m: &ManeuverInput,
    ) -> Result<Option<&PropulsionThruster>, AnalysisError> {
        let Some(id) = &m.thruster else {
            return Ok(None);
        };
        self.thrusters
            .iter()
            .find(|t| &t.id == id)
            .map(Some)
            .ok_or_else(|| {
                AnalysisError::InvalidInput(format!(
                    "マヌーバ {} のスラスタ {id} がありません",
                    m.id
                ))
            })
    }
}
