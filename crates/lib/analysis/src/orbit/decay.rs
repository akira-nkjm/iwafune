//! 大気抵抗による高度低下と軌道寿命、高度を保つ ΔV（軌道維持）と推薬の寿命。
//!
//! **方法（周回平均の半解析）**: 1 周の平均の長半径の変化率 `da/dt = 2a²/μ ⟨v · a_D⟩`（Gauss の式。
//! `a_D` は NRLMSISE-00 の密度・共回転大気の相対速度・`Cd·A/m` の抗力の加速度）を、周回ごとに求め直しながら
//! RK4 で積分する（[`rates`]・[`simulate`]）。King-Hele（*Satellite Orbits in an Atmosphere*, 1987）の
//! 近円軌道の式 `da/dt = −ρ (Cd·A/m) √(μa)` を、密度の緯度・地方時・季節の変化まで含めて周回平均したものに当たる。
//!
//! **適用の範囲**: 近円軌道（離心率 ≲ 0.01。離心率と近地点引数の変化は追わず、それより大きければ注意を出す）。
//! 高度はおよそ 180〜1000 km（1 周の平均が意味を持つ範囲。再突入の直前の数日は粗い）。抗力の面積は一定
//! （姿勢・パネルの向きの変化は面積の与え方で見込む）。推力による高度の変化は ΔV の積算としてだけ扱う。
//!
//! **軌道維持の推薬**: 推進系の解析（`propulsion.propellant`）の値から、軌道維持のマヌーバのスラスタと推薬（無ければ推薬が 1 種類のときだけ全量。
//! 推薬をまたいで換算しない。[`propellant`]）。推薬の量をここで持たない。
//!
//! **太陽活動の条件**: ECSS-E-ST-10-04C の長期のプリセット（低・中・高）を全期間で一定にしたものと、
//! 宇宙天気の表から日付で引いた列（[`DecayWeatherSchedule`]。呼び出し側が表から引いて入力に入れる）。

mod propellant;
mod rates;
mod simulate;
mod weather;

pub use rates::ORBIT_SAMPLES;
pub use simulate::{MAX_DROP_PER_STEP_M, MAX_STEP_S, MIN_STEP_S, RECORD_STEP_S};
pub use weather::{
    DecayCondition, DecayWeatherOrigins, DecayWeatherSample, DecayWeatherSchedule,
    DecayWeatherSummary, WEATHER_STEP_S, dated_schedule,
};

use crate::propulsion::PropulsionInput;
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Channel, ChannelKind, Check,
    ConditionTopic, Series, ensure_positive,
};
use sat_astro::atmosphere::space_weather::{DatedSpaceWeather, SolarActivity};
use sat_astro::constants::{SECONDS_PER_DAY, SECONDS_PER_JULIAN_CENTURY};
use sat_astro::ephem::CentralBody;
use sat_astro::math::Vec3;
use sat_astro::orbit::{OrbitSpec, altitude_from_semi_major_axis};
use sat_units::typed::{Area, Length, Mass, MassDensity, Qty, Time, Velocity};
use serde::{Deserialize, Serialize};
use simulate::{DecayTrack, HoldTrack, Model};
use weather::Weather;

mod checks;
mod output;
pub use output::{DecayCase, DecayUsed, DragAreaSource, StationKeeping};

/// 1 年 [s]（ユリウス年）。
pub const YEAR_S: f64 = SECONDS_PER_JULIAN_CENTURY / 100.0;
/// 抗力係数の既定値（自由分子流の中の箱形の衛星の慣例値 2.2。Vallado, 4th ed., §8.6.2）。
pub const DEFAULT_DRAG_COEFFICIENT: f64 = 2.2;
/// 積分の期間の上限の既定値（25 年。IADC のデブリ低減ガイドラインの廃棄後 25 年）。
pub const DEFAULT_SPAN_S: f64 = 25.0 * YEAR_S;
/// 再突入とみなす高度の既定値（120 km。これより下は数時間で落ちる）。
pub const DEFAULT_REENTRY_ALTITUDE_M: f64 = 120e3;
/// ミッション期間が無いとき、軌道維持の ΔV を積算する期間（1 年）。
pub const DEFAULT_STATION_KEEPING_SPAN_S: f64 = YEAR_S;
/// 近円軌道とみなす離心率の上限（これより大きければ注意を出す）。
pub const NEAR_CIRCULAR_ECCENTRICITY: f64 = 0.01;
/// 一定の条件として求める太陽活動のプリセット（並べる順）。
pub const PRESETS: [SolarActivity; 3] = [
    SolarActivity::Low,
    SolarActivity::Moderate,
    SolarActivity::High,
];

/// 高度低下の解析の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrbitDecayInput {
    /// 初期の軌道（元期・長半径・傾斜角・RAAN）
    pub orbit: OrbitSpec,
    /// 機体の質量
    pub mass: Qty<Mass>,
    /// 抗力の面積（無ければ `body_size` の箱の平均の投影面積）
    pub drag_area: Option<Qty<Area>>,
    /// 機体の外形の箱 [m]（`drag_area` が無いときに使う）
    pub body_size: Option<Vec3>,
    /// 抗力係数 Cd（無ければ [`DEFAULT_DRAG_COEFFICIENT`]）
    pub drag_coefficient: Option<f64>,
    /// ミッション期間（寿命の判定と、軌道維持の ΔV を積算する期間）
    pub mission_duration: Option<Qty<Time>>,
    /// 積分の期間の上限（無ければ [`DEFAULT_SPAN_S`]）
    pub span: Option<Qty<Time>>,
    /// 再突入とみなす高度（無ければ [`DEFAULT_REENTRY_ALTITUDE_M`]）
    pub reentry_altitude: Option<Qty<Length>>,
    /// 軌道維持の高度の帯（無ければ抗力を連続して打ち消す）
    pub band: Option<Qty<Length>>,
    /// 寿命の上限（廃棄の要求。例: 25 年）
    pub max_lifetime: Option<Qty<Time>>,
    /// 軌道維持に使う推進系（推進系の解析と同じ入力。使える推薬は `propulsion.propellant` が決める。
    /// 無ければ推薬の寿命を求めない）
    pub propulsion: Option<PropulsionInput>,
    /// 宇宙天気の表から日付で引いた列（無ければプリセットだけ）
    #[serde(default)]
    pub dated_weather: Option<DecayWeatherSchedule>,
}

impl OrbitDecayInput {
    /// 積分の期間の上限 [s]（書かれていなければ既定値）。日付で引く列の長さにも使う。
    pub fn span_s(&self) -> f64 {
        self.span.map_or(DEFAULT_SPAN_S, |q| q.value())
    }
}

/// 高度低下の解析の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrbitDecayOutput {
    /// 太陽活動の条件ごとの結果（低・中・高、日付で引いた列があればその条件）
    pub cases: Vec<DecayCase>,
    /// 使った値（既定値を含む）
    pub used: DecayUsed,
    pub checks: Vec<Check>,
}

/// 大気抵抗による高度低下・軌道寿命・軌道維持の ΔV。
pub struct OrbitDecay;

impl Analysis for OrbitDecay {
    const ID: &'static str = "orbit.decay";
    const USES: &'static [&'static str] = &[crate::propulsion::Propellant::ID];
    const TITLE: &'static str = "高度低下と軌道維持";
    const TITLE_EN: &'static str = "Drag decay and station keeping";
    const AREA: AnalysisArea = AnalysisArea::Condition(ConditionTopic::Orbit);
    const KIND: AnalysisKind = AnalysisKind::TimeSeries;
    const DESCRIPTION: &'static str = "太陽活動の条件ごとの高度低下の速さ・軌道寿命と、高度を保つ ΔV（NRLMSISE-00 の周回平均。近円軌道）";
    const DESCRIPTION_EN: &'static str = "Altitude decay rate, orbital lifetime and station-keeping delta-v per solar-activity condition (orbit-averaged NRLMSISE-00, near-circular orbits)";
    type Input = OrbitDecayInput;
    type Output = OrbitDecayOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let (used, undetermined) = used_values(input)?;
        let cd_a_over_m = used.drag_coefficient * used.drag_area.value() / input.mass.value();
        let mut conditions: Vec<(DecayCondition, Weather<'_>, DatedSpaceWeather)> = PRESETS
            .iter()
            .map(|&activity| {
                let w = DatedSpaceWeather::preset(activity);
                (
                    DecayCondition::Preset { activity },
                    Weather::Constant(w.atmosphere_model()),
                    w,
                )
            })
            .collect();
        if let Some(s) = &input.dated_weather
            && let Some(first) = s.samples.first()
        {
            let condition = DecayCondition::Dated {
                fallback: s.fallback,
            };
            conditions.push((condition, Weather::Dated(&s.samples), first.weather));
        }
        let cases = conditions
            .into_iter()
            .map(|(condition, weather, initial)| {
                let model = Model {
                    base: &input.orbit,
                    cd_a_over_m,
                    weather,
                };
                run_case(input, &used, &model, condition, initial)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let checks = checks::checks(input, &used, &cases, undetermined.as_deref());
        Ok(OrbitDecayOutput {
            cases,
            used,
            checks,
        })
    }
}

/// 既定値を当てた値と、入力の検査。軌道維持のスラスタを決められないときは、その推薬の名前も返す。
fn used_values(input: &OrbitDecayInput) -> Result<(DecayUsed, Option<Vec<String>>), AnalysisError> {
    if input.orbit.central != CentralBody::Earth {
        return Err(AnalysisError::InvalidInput(
            "Altitude decay is not supported for bodies other than Earth".into(),
        ));
    }
    ensure_positive("Spacecraft mass", input.mass.value())?;
    let drag_coefficient = input.drag_coefficient.unwrap_or(DEFAULT_DRAG_COEFFICIENT);
    ensure_positive("Drag coefficient", drag_coefficient)?;
    let (drag_area, area_source) = match (input.drag_area, input.body_size) {
        (Some(a), _) => (a, DragAreaSource::Given),
        (None, Some(size)) => (
            Qty::from_raw(tumbling_area(size)),
            DragAreaSource::TumblingBox,
        ),
        (None, None) => {
            return Err(AnalysisError::InvalidInput(
                "Either a drag area or the spacecraft outline is required".into(),
            ));
        }
    };
    ensure_positive("Drag area", drag_area.value())?;
    let span = input.span_s();
    ensure_positive("Integration span", span)?;
    let reentry = reentry_altitude(input)?;
    let station_keeping_span = input
        .mission_duration
        .map_or(DEFAULT_STATION_KEEPING_SPAN_S, |q| q.value());
    ensure_positive("Mission duration", station_keeping_span)?;
    if let Some(b) = input.band {
        ensure_positive("Station-keeping altitude band", b.value())?;
    }
    let (sk, note) = match input.propulsion.as_ref().map(propellant::capacity) {
        Some(Ok(propellant::StationKeeper::Found(c))) => (Some(c), None),
        Some(Ok(propellant::StationKeeper::Undetermined(names))) => (None, Some(names)),
        Some(Ok(propellant::StationKeeper::NoThruster)) | None => (None, None),
        Some(Err(e)) => return Err(e),
    };
    let used = DecayUsed {
        drag_coefficient,
        drag_area,
        area_source,
        span: Qty::from_raw(span),
        reentry_altitude: Qty::from_raw(reentry),
        station_keeping_span: Qty::from_raw(station_keeping_span),
        band: input.band,
        propellant: sk.as_ref().map(|c| c.propellant),
        thruster: sk.as_ref().map(|c| c.thruster.clone()),
        propulsion_delta_v: sk.map(|c| c.delta_v),
        orbit_samples: ORBIT_SAMPLES as u32,
        max_step: Qty::from_raw(MAX_STEP_S),
        weather_step: input
            .dated_weather
            .as_ref()
            .map(|_| Qty::from_raw(WEATHER_STEP_S)),
    };
    Ok((used, note))
}

/// 再突入とみなす高度 [m]（既定値を当てる）。初期の近地点がそれより上か確かめる。
fn reentry_altitude(input: &OrbitDecayInput) -> Result<f64, AnalysisError> {
    let reentry = input
        .reentry_altitude
        .map_or(DEFAULT_REENTRY_ALTITUDE_M, |q| q.value());
    ensure_positive("Re-entry altitude", reentry)?;
    let e = input.orbit.elements.eccentricity;
    if !(0.0..1.0).contains(&e) {
        return Err(AnalysisError::InvalidInput(format!(
            "Eccentricity must be 0 or more and less than 1 ({e})"
        )));
    }
    let perigee = altitude_from_semi_major_axis(
        input.orbit.central,
        input.orbit.elements.semi_major_axis.value() * (1.0 - e),
    );
    if perigee <= reentry {
        return Err(AnalysisError::InvalidInput(
            "The initial perigee altitude is at or below the re-entry altitude".into(),
        ));
    }
    Ok(reentry)
}

/// 箱（辺 `size` [m]）がどの向きも同じ確率でとるときの平均の投影面積 [m²]。
/// 凸の物体の平均の投影面積は表面積の 1/4（Cauchy の公式）なので、箱では `(xy + yz + zx) / 2`。
pub fn tumbling_area(size: Vec3) -> f64 {
    (size.x * size.y + size.y * size.z + size.z * size.x) / 2.0
}

fn run_case(
    input: &OrbitDecayInput,
    used: &DecayUsed,
    model: &Model<'_>,
    condition: DecayCondition,
    initial_weather: DatedSpaceWeather,
) -> Result<DecayCase, AnalysisError> {
    let a0 = input.orbit.elements.semi_major_axis.value();
    let h0 = input.orbit.altitude_m();
    let first = model.mean(0.0, a0, input.orbit.elements.raan.value())?;
    let track = simulate::decay(
        model,
        a0,
        used.span.value(),
        used.reentry_altitude.value(),
        YEAR_S,
    )?;
    let held = simulate::hold(
        model,
        a0,
        used.station_keeping_span.value(),
        used.band.map(|b| b.value()),
    )?;
    let weather_sources = match model.weather {
        Weather::Dated(samples) => Some(weather::summarize(input.orbit.epoch, samples)),
        Weather::Constant(_) => None,
    };
    Ok(DecayCase {
        id: case_id(condition).to_owned(),
        condition,
        initial_weather,
        weather_sources,
        initial_density: Qty::<MassDensity>::from_raw(first.density),
        initial_decay_rate: Qty::<Velocity>::from_raw(-first.a_dot),
        decay_per_day: Qty::from_raw(-first.a_dot * SECONDS_PER_DAY),
        decay_first_year: Qty::from_raw(h0 - track.altitude_at_probe),
        lifetime: track.reentry.map(Qty::from_raw),
        reentry_epoch: track.reentry.map(|t| input.orbit.epoch.plus_seconds(t)),
        final_altitude: Qty::from_raw(track.final_altitude),
        station_keeping: station_keeping(used, &held),
        series: altitude_series(&track),
    })
}

/// 中程度のプリセットの条件の ID。
pub const MODERATE: &str = "moderate";
/// 日付で引いた条件の ID。
pub const DATED: &str = "dated";

/// 条件の ID（結果の並びの中で一意）。
fn case_id(condition: DecayCondition) -> &'static str {
    match condition {
        DecayCondition::Preset {
            activity: SolarActivity::Low,
        } => "low",
        DecayCondition::Preset {
            activity: SolarActivity::Moderate,
        } => MODERATE,
        DecayCondition::Preset {
            activity: SolarActivity::High,
        } => "high",
        DecayCondition::Dated { .. } => DATED,
    }
}

fn station_keeping(used: &DecayUsed, held: &HoldTrack) -> StationKeeping {
    let span = used.station_keeping_span.value();
    let per_year = held.delta_v * YEAR_S / span;
    let burns = held.burns;
    let propellant_life = used
        .propulsion_delta_v
        .filter(|_| per_year > 0.0)
        .map(|dv| Qty::from_raw(dv.value() / per_year * YEAR_S));
    StationKeeping {
        delta_v_per_year: Qty::from_raw(per_year),
        delta_v_total: Qty::from_raw(held.delta_v),
        burns,
        burn_interval: burns
            .zip(held.last_burn)
            .filter(|(n, _)| *n > 0)
            .map(|(n, last)| Qty::from_raw(last / f64::from(n))),
        delta_v_per_burn: burns
            .filter(|n| *n > 0)
            .map(|n| Qty::from_raw(held.delta_v / f64::from(n))),
        propellant_life,
    }
}

fn altitude_series(track: &DecayTrack) -> Series {
    Series {
        t: track.points.iter().map(|p| p.0).collect(),
        channels: vec![Channel::of::<Length>(
            "altitude",
            ChannelKind::Altitude,
            track.points.iter().map(|p| p.1).collect(),
        )],
    }
}

#[cfg(test)]
mod station_keeping_tests;
#[cfg(test)]
mod tests;
