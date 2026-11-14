//! β と蝕の季節変動。各標本日の1周では太陽位置を固定し、中心天体（地球・月・惑星。半径は `CentralBody::radius_m`）の
//! 円錐影を集計する。月周回で地球に隠される（月食）時間は数えない。
//! 数値伝搬のマヌーバがあれば、標本日の軌道はマヌーバの順の軌道の推移（`propulsion::orbit_history`）から引く。

use crate::propulsion::{OrbitHistory, PropulsionInput, orbit_history};
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Channel, ChannelKind, ConditionTopic,
    Series, ensure_positive, time_fraction,
};
use sat_astro::ephem::{Body, CentralBody};
use sat_astro::orbit::{OrbitSpec, beta_angle_rad};
use sat_astro::shadow::{ShadowModel, central_shadow_factor};
use sat_astro::time::Epoch;
use sat_astro::timeline::sample_times;
use sat_units::typed::{Angle, Qty, Time};
use serde::{Deserialize, Serialize};

/// 季節の標本の既定間隔 [s]（1 日）。
pub const DEFAULT_STEP_S: f64 = sat_astro::constants::SECONDS_PER_DAY;
/// 1 周内の影の標本間隔 [s]。境界の誤差は各境界で最大この間隔程度。
pub const SHADOW_STEP_S: f64 = 5.0;

/// 季節解析の入力。ケースの β 上書きは使わず、実軌道の季節変動を求める。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EclipseSeasonInput {
    pub orbit: OrbitSpec,
    pub duration: Qty<Time>,
    /// 省略時は1日。
    pub step: Option<Qty<Time>>,
    /// マヌーバ（数値伝搬のマヌーバがあるときだけ projection が渡す）。あれば各標本日の軌道を
    /// マヌーバの順の軌道の推移（`propulsion::orbit_history`）から引く
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maneuvers: Option<PropulsionInput>,
}

/// 日付付きの標本（半影は本影を含まない部分の時間）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EclipseSeasonPoint {
    pub epoch: Epoch,
    pub beta: Qty<Angle>,
    pub umbra: Qty<Time>,
    pub penumbra: Qty<Time>,
}

/// 季節解析の出力。極値の同値は最初の標本を選ぶ。判定・違反は付けない。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EclipseSeasonOutput {
    pub points: Vec<EclipseSeasonPoint>,
    /// 本影と半影の合計が最大の標本。
    pub longest: EclipseSeasonPoint,
    pub max_eclipse_duration: Qty<Time>,
    pub beta_min: EclipseSeasonPoint,
    pub beta_max: EclipseSeasonPoint,
    /// 蝕が無いと判定した標本の数（端数の最終標本も含む。日数ではない）。
    pub eclipse_free_samples: usize,
    /// 季節の標本の蝕なしフラグを台形則で積算した期間（推定）。
    pub eclipse_free_duration: Qty<Time>,
    pub step: Qty<Time>,
    pub shadow_step: Qty<Time>,
    /// 1 周中の太陽位置を標本日の位置に固定する近似。
    pub fixed_sun_per_orbit: bool,
    /// 標本日の軌道を変えたマヌーバ（実行の順。マヌーバの推移を使わなければ空）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub maneuvers: Vec<String>,
    pub series: Series,
}

/// 軌道条件の季節解析。
pub struct EclipseSeason;

impl Analysis for EclipseSeason {
    const ID: &'static str = "orbit.eclipse_season";
    const TITLE: &'static str = "β と蝕の季節変動";
    const TITLE_EN: &'static str = "Beta angle and eclipse seasons";
    const AREA: AnalysisArea = AnalysisArea::Condition(ConditionTopic::Orbit);
    const KIND: AnalysisKind = AnalysisKind::TimeSeries;
    const DESCRIPTION: &'static str =
        "ミッション期間の β と1周の本影・半影（1周中の太陽位置は固定）";
    const DESCRIPTION_EN: &'static str =
        "Mission beta angle and umbra/penumbra per orbit, holding the Sun fixed within each orbit";
    type Input = EclipseSeasonInput;
    type Output = EclipseSeasonOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        if input.orbit.central == CentralBody::Sun {
            return Err(AnalysisError::InvalidInput(
                "太陽を中心天体とする軌道には β と蝕の季節変動がありません".into(),
            ));
        }
        ensure_positive("duration", input.duration.value())?;
        let step = input.step.map_or(DEFAULT_STEP_S, |q| q.value());
        let times = sample_times(input.duration.value(), step)?;
        // 日ごとの1周を積み上げる。時刻格子・伝搬・太陽・影は既存の正本を使う。
        let history = match &input.maneuvers {
            Some(p) => orbit_history(p)?,
            None => OrbitHistory::constant(input.orbit.clone()),
        };
        let mut used = Vec::new();
        let mut points = Vec::with_capacity(times.len());
        for &t in &times {
            let (seg, dt) = history.locate(t)?;
            if let Some(id) = &seg.maneuver
                && !used.contains(id)
            {
                used.push(id.clone());
            }
            points.push(point(&seg.orbit, dt)?);
        }
        let mut out = summarize(points, times, step)?;
        out.maneuvers = used;
        Ok(out)
    }
}

/// 軌道 `orbit` の元期から `t` 秒後の標本日（時刻は区間の軌道の元期から）。
fn point(orbit: &OrbitSpec, t: f64) -> Result<EclipseSeasonPoint, AnalysisError> {
    let epoch = orbit.epoch.plus_seconds(t);
    let sun = orbit.central.relative_position_m(Body::Sun, epoch);
    let (umbra, penumbra) = shadow_durations(orbit, t, sun, ShadowModel::Conical)?;
    Ok(EclipseSeasonPoint {
        epoch,
        beta: Qty::from_raw(beta_angle_rad(orbit, epoch)?),
        umbra: Qty::from_raw(umbra),
        penumbra: Qty::from_raw(penumbra),
    })
}

fn shadow_durations(
    orbit: &OrbitSpec,
    start: f64,
    sun: sat_astro::math::Vec3,
    model: ShadowModel,
) -> Result<(f64, f64), AnalysisError> {
    let period = orbit.period_s();
    ensure_positive("period", period)?;
    let times = sample_times(period, SHADOW_STEP_S)?;
    let offsets: Vec<_> = times.iter().map(|t| start + t).collect();
    let states = orbit.states_at(&offsets)?;
    let light: Vec<_> = states
        .iter()
        .map(|s| central_shadow_factor(orbit.central, s.r_eci, sun, model))
        .collect();
    let umbra: Vec<_> = light.iter().map(|&f| f <= 0.0).collect();
    let penumbra: Vec<_> = light.iter().map(|&f| f > 0.0 && f < 1.0).collect();
    Ok((
        time_fraction(&times, &umbra, &[]).value() * period,
        time_fraction(&times, &penumbra, &[]).value() * period,
    ))
}

fn eclipse(p: &EclipseSeasonPoint) -> f64 {
    p.umbra.value() + p.penumbra.value()
}

fn summarize(
    points: Vec<EclipseSeasonPoint>,
    times: Vec<f64>,
    step: f64,
) -> Result<EclipseSeasonOutput, AnalysisError> {
    let first = points
        .first()
        .ok_or_else(|| AnalysisError::InvalidInput("標本がありません".into()))?;
    let (mut longest, mut beta_min, mut beta_max) = (first, first, first);
    for p in &points {
        if eclipse(p) > eclipse(longest) {
            longest = p;
        }
        if p.beta.value() < beta_min.beta.value() {
            beta_min = p;
        }
        if p.beta.value() > beta_max.beta.value() {
            beta_max = p;
        }
    }
    let free: Vec<_> = points.iter().map(|p| eclipse(p) == 0.0).collect();
    let eclipse_free_duration = Qty::from_raw(
        time_fraction(&times, &free, &[]).value() * times.last().copied().unwrap_or(0.0),
    );
    let channels = season_channels(&points);
    Ok(EclipseSeasonOutput {
        longest: longest.clone(),
        max_eclipse_duration: Qty::from_raw(eclipse(longest)),
        beta_min: beta_min.clone(),
        beta_max: beta_max.clone(),
        eclipse_free_samples: free.iter().filter(|&&f| f).count(),
        eclipse_free_duration,
        step: Qty::from_raw(step),
        shadow_step: Qty::from_raw(SHADOW_STEP_S),
        fixed_sun_per_orbit: true,
        maneuvers: Vec::new(),
        series: Series { t: times, channels },
        points,
    })
}

fn season_channels(points: &[EclipseSeasonPoint]) -> Vec<Channel> {
    vec![
        Channel::of::<Angle>(
            "beta",
            ChannelKind::Beta,
            points.iter().map(|p| p.beta.value()).collect(),
        ),
        Channel::of::<Time>(
            "umbra",
            ChannelKind::UmbraDuration,
            points.iter().map(|p| p.umbra.value()).collect(),
        ),
        Channel::of::<Time>(
            "penumbra",
            ChannelKind::PenumbraDuration,
            points.iter().map(|p| p.penumbra.value()).collect(),
        ),
    ]
}

#[cfg(test)]
mod tests;
