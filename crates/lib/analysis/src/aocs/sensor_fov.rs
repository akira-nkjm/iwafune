//! センサ FoV と禁止角（`aocs.sensor_fov`）。
//!
//! 軌道と姿勢則の時系列（`sat_astro::timeline`）の各点で、センサの視線（機体座標系）と
//! 太陽・中心天体（地球）の縁・月のなす角を求め、禁止角を下回る時間（侵入）の割合を時間平均（台形則）で数える。
//!
//! - 太陽: 日陰中（[`crate::spec::is_sunlit`] が偽）は侵入としない。
//! - 中心天体の縁（`earth_limb`）: `視線と中心方向のなす角 − 見込み半角 asin(R/|r|)`。R は `TimelineSpec.orbit.central`
//!   の半径。負なら視線が円盤の中。
//! - 月: 衛星から月への方向（`CentralBody::relative_position_m`）。中心天体の陰にあるとき
//!   （`sat_astro::shadow::occulted_by`）は侵入としない。

use crate::spec::quantity_text;
use crate::spec::ratio_margin;
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Channel, ChannelKind, Check, Series,
    System, Verdict, ensure_fraction, ensure_non_negative, is_sunlit, longest_window,
    time_fraction,
};
use sat_astro::ephem::{Body, CentralBody};
use sat_astro::math::Vec3;
use sat_astro::shadow::occulted_by;
use sat_astro::timeline::{self, TimelineSample, TimelineSpec, Window, windows};
use sat_units::typed::{Angle, Fraction, Qty, Time};
use serde::{Deserialize, Serialize};

/// センサ FoV と禁止角の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SensorFovInput {
    pub timeline: TimelineSpec,
    pub sensors: Vec<SensorFovSensor>,
}

/// センサ 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SensorFovSensor {
    pub id: String,
    /// 視線方向（機体座標系。長さは問わない）
    pub boresight: Vec3,
    /// 視野の半角（参考として出力に返す。禁止角の判定には使わない）
    #[serde(default)]
    pub fov_half_angle: Option<Qty<Angle>>,
    /// 太陽の禁止角（省略時は判定しない）
    #[serde(default)]
    pub exclusion_sun: Option<Qty<Angle>>,
    /// 地球の縁の禁止角（省略時は判定しない）
    #[serde(default)]
    pub exclusion_earth: Option<Qty<Angle>>,
    /// 月の禁止角（省略時は判定しない）
    #[serde(default)]
    pub exclusion_moon: Option<Qty<Angle>>,
    /// 侵入してよい時間の割合の上限（0〜1）。省略時は侵入があれば警告
    #[serde(default)]
    pub max_intrusion_fraction: Option<Fraction>,
}

/// 禁止角を調べる天体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SensorFovTarget {
    Sun,
    EarthLimb,
    Moon,
}

impl SensorFovTarget {
    const ALL: [SensorFovTarget; 3] = [Self::Sun, Self::EarthLimb, Self::Moon];

    fn key(self) -> &'static str {
        match self {
            Self::Sun => "sun",
            Self::EarthLimb => "earth_limb",
            Self::Moon => "moon",
        }
    }

    /// この天体となす角の時系列の種類（表示名は service が語彙で引く）。
    fn channel(self) -> ChannelKind {
        match self {
            Self::Sun => ChannelKind::SunAngle,
            Self::EarthLimb => ChannelKind::EarthLimbAngle,
            Self::Moon => ChannelKind::MoonAngle,
        }
    }

    fn exclusion(self, s: &SensorFovSensor) -> Option<Qty<Angle>> {
        match self {
            Self::Sun => s.exclusion_sun,
            Self::EarthLimb => s.exclusion_earth,
            Self::Moon => s.exclusion_moon,
        }
    }
}

/// センサ FoV と禁止角の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SensorFovOutput {
    pub sensors: Vec<SensorFovResult>,
    /// 時系列: `<センサ>:sun`・`<センサ>:earth_limb`・`<センサ>:moon`（視線とのなす角）
    pub series: Series,
    pub checks: Vec<Check>,
}

/// センサ 1 つの結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SensorFovResult {
    pub id: String,
    pub fov_half_angle: Option<Qty<Angle>>,
    /// 禁止角を与えた天体ごとの結果
    pub targets: Vec<SensorFovTargetResult>,
    /// どれかの天体が侵入している時間の割合（0〜1）
    pub intrusion_fraction: Fraction,
    /// どれかの天体が侵入している区間
    pub intrusions: Vec<Window>,
}

/// 天体 1 つの結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SensorFovTargetResult {
    pub target: SensorFovTarget,
    pub exclusion: Qty<Angle>,
    /// 視線とのなす角の最小（侵入とみなさない時間も含む）
    pub min_angle: Qty<Angle>,
    /// 侵入している時間の割合（0〜1）
    pub intrusion_fraction: Fraction,
    /// 最長の侵入
    pub longest_intrusion: Qty<Time>,
    /// 侵入の区間（端はサンプル時刻）
    pub windows: Vec<Window>,
}

/// センサ FoV と禁止角。
pub struct SensorFov;

impl Analysis for SensorFov {
    const ID: &'static str = "aocs.sensor_fov";
    const TITLE: &'static str = "センサ FoV と禁止角";
    const AREA: AnalysisArea = AnalysisArea::System(System::Aocs);
    const KIND: AnalysisKind = AnalysisKind::TimeSeries;
    const DESCRIPTION: &'static str = "センサの視野への太陽・地球・月の侵入を時系列で調べる";
    const TITLE_EN: &'static str = "Sensor FoV and exclusion angles";
    const DESCRIPTION_EN: &'static str =
        "Time series of Sun, Earth and Moon intrusion into sensor fields of view";
    type Input = SensorFovInput;
    type Output = SensorFovOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        for s in &input.sensors {
            validate(s)?;
        }
        let samples = timeline::build(&input.timeline)?;
        let central = input.timeline.orbit().central();
        let geometry: Vec<Geometry> = samples.iter().map(|s| Geometry::at(central, s)).collect();
        let t: Vec<f64> = samples.iter().map(|s| s.t_s).collect();
        let mut series = Series {
            t: t.clone(),
            channels: Vec::new(),
        };
        let mut sensors = Vec::new();
        for s in &input.sensors {
            sensors.push(evaluate_sensor(s, &t, &geometry, &mut series));
        }
        let checks = input
            .sensors
            .iter()
            .zip(&sensors)
            .map(|(s, r)| verdict(s, r))
            .collect();
        Ok(SensorFovOutput {
            sensors,
            series,
            checks,
        })
    }
}

fn validate(s: &SensorFovSensor) -> Result<(), AnalysisError> {
    if s.boresight.normalized().is_none() {
        return Err(AnalysisError::InvalidInput(format!(
            "Sensor {} boresight is a zero vector",
            s.id
        )));
    }
    let angles = [
        ("FoV half angle", s.fov_half_angle),
        ("Sun exclusion angle", s.exclusion_sun),
        ("Earth exclusion angle", s.exclusion_earth),
        ("Moon exclusion angle", s.exclusion_moon),
    ];
    for (what, a) in angles {
        if let Some(a) = a {
            ensure_non_negative(&format!("Sensor {} {what}", s.id), a.value())?;
        }
    }
    if let Some(f) = s.max_intrusion_fraction {
        ensure_fraction(
            &format!("Sensor {} intrusion fraction limit", s.id),
            f.value(),
        )?;
    }
    Ok(())
}

/// 1 点での天体の方向（機体座標系）。
struct Geometry {
    sun_body: Vec3,
    sun_visible: bool,
    nadir_body: Vec3,
    /// 地球の見込み半角 [rad]
    earth_radius_rad: f64,
    moon_body: Vec3,
    moon_visible: bool,
}

impl Geometry {
    /// `central` は軌道の中心天体。位置はどれも中心天体から（`TimelineSample.r_eci` と同じ）。
    fn at(central: CentralBody, s: &TimelineSample) -> Self {
        let radius = central.radius_m();
        let earth_radius_rad = (radius / s.r_eci.norm().max(radius)).asin();
        let r_moon = central.relative_position_m(Body::Moon, s.epoch);
        // 中心天体の陰にある月は侵入としない（`sat_astro::shadow::occulted_by`）
        let hidden = occulted_by(Vec3::ZERO, radius, s.r_eci, r_moon);
        let to_moon_eci = r_moon - s.r_eci;
        Self {
            sun_body: s.sun_dir_body(),
            sun_visible: is_sunlit(s),
            nadir_body: s.nadir_body(),
            earth_radius_rad,
            moon_body: s.to_body(to_moon_eci),
            moon_visible: !hidden,
        }
    }

    /// 視線 `b` と天体のなす角 [rad] と、侵入を数えてよいか。
    fn angle(&self, target: SensorFovTarget, b: Vec3) -> (f64, bool) {
        match target {
            SensorFovTarget::Sun => (b.angle_to(self.sun_body), self.sun_visible),
            SensorFovTarget::EarthLimb => {
                (b.angle_to(self.nadir_body) - self.earth_radius_rad, true)
            }
            SensorFovTarget::Moon => (b.angle_to(self.moon_body), self.moon_visible),
        }
    }
}

fn evaluate_sensor(
    s: &SensorFovSensor,
    t: &[f64],
    geometry: &[Geometry],
    series: &mut Series,
) -> SensorFovResult {
    let b = s.boresight.normalized().unwrap_or(Vec3::Z);
    let mut any = vec![false; t.len()];
    let mut targets = Vec::new();
    for target in SensorFovTarget::ALL {
        let (angles, countable): (Vec<f64>, Vec<bool>) =
            geometry.iter().map(|g| g.angle(target, b)).unzip();
        let key = format!("{}:{}", s.id, target.key());
        series
            .channels
            .push(Channel::of::<Angle>(key, target.channel(), angles.clone()).about(&s.id));
        let Some(exclusion) = target.exclusion(s) else {
            continue;
        };
        let flags: Vec<bool> = angles
            .iter()
            .zip(&countable)
            .map(|(&a, &c)| c && a < exclusion.value())
            .collect();
        any.iter_mut().zip(&flags).for_each(|(a, &f)| *a |= f);
        let w = windows(t, &flags);
        targets.push(SensorFovTargetResult {
            target,
            exclusion,
            min_angle: Qty::from_raw(angles.iter().copied().fold(f64::INFINITY, f64::min)),
            intrusion_fraction: time_fraction(t, &flags, &[]),
            longest_intrusion: longest_window(&w),
            windows: w,
        });
    }
    SensorFovResult {
        id: s.id.clone(),
        fov_half_angle: s.fov_half_angle,
        targets,
        // 侵入の割合は周回で区切らず、全期間で数える
        intrusion_fraction: time_fraction(t, &any, &[]),
        intrusions: windows(t, &any),
    }
}

fn verdict(s: &SensorFovSensor, r: &SensorFovResult) -> Check {
    let key = sat_checkkey::sensor_intrusion(&s.id);
    let label = format!("{} exclusion angle", s.id);
    let pct = quantity_text::<Fraction>(r.intrusion_fraction.value());
    // 余裕 = 上限 / 侵入の割合 − 1（上限が無い・侵入が無ければ余裕を持たない）
    let margin = s
        .max_intrusion_fraction
        .and_then(|max| ratio_margin(max.value(), r.intrusion_fraction.value()));
    let (v, msg) = match s.max_intrusion_fraction {
        Some(max) if r.intrusion_fraction <= max => (
            Verdict::Ok,
            format!(
                "Intrusion {pct} (limit {})",
                quantity_text::<Fraction>(max.value())
            ),
        ),
        Some(max) => (
            Verdict::Violation,
            format!(
                "Intrusion {pct} (limit {})",
                quantity_text::<Fraction>(max.value())
            ),
        ),
        None if r.intrusion_fraction.value() > 0.0 => {
            (Verdict::Warning, format!("Intrusion {pct}"))
        }
        None => (Verdict::Ok, "No intrusion".to_owned()),
    };
    Check::new(&key, &label, v, msg).with_ratio_margin(margin)
}

#[cfg(test)]
mod tests;
