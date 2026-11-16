//! 日照時の発電（`power.solar_sunlit`）。
//!
//! ケースごとに、太陽方向を直接与えるか（機体座標系）、軌道と姿勢則の時系列を与える。
//! 時系列のケースは日照中（[`crate::spec::is_sunlit`]）の時間だけの時間平均（台形則）と最小・最大を出し、
//! **全日照（日照率 1）として**発電を評価する（姿勢による差だけを見るため）。発電モデルは [`super::solar`]。

use super::solar::{SolarArray, SolarLife, total};
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, System, is_sunlit, masked_time_mean,
};
use sat_astro::math::Vec3;
use sat_astro::timeline::{self, TimelineSpec};
use sat_units::typed::{Angle, Power, Qty};
use serde::{Deserialize, Serialize};

/// 日照時の発電の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SolarSunlitInput {
    pub array: SolarArray,
    pub cases: Vec<SolarSunlitCase>,
}

/// 評価するケース 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SolarSunlitCase {
    pub id: String,
    #[serde(default)]
    pub label: String,
    pub sun: SolarSunlitSun,
}

/// 太陽方向の与え方。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SolarSunlitSun {
    /// 機体座標系の太陽方向（長さは問わない）。太陽光の強さは上書きが無ければ 1 AU の太陽定数
    Direction { sun_dir_body: Vec3 },
    /// 軌道と姿勢則から作る時系列。日照中の時間平均・最小・最大を出す
    Timeline { timeline: Box<TimelineSpec> },
}

/// 日照時の発電の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SolarSunlitOutput {
    pub cases: Vec<SolarSunlitCaseResult>,
}

/// 1 ケースの結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SolarSunlitCaseResult {
    pub id: String,
    pub label: String,
    /// 日照中の平均発電（BOL。方向のケースは 1 点の値）
    pub bol: Qty<Power>,
    /// 日照中の平均発電（EOL）
    pub eol: Qty<Power>,
    /// 日照中の最小発電（EOL）
    pub eol_min: Qty<Power>,
    /// 日照中の最大発電（EOL）
    pub eol_max: Qty<Power>,
    /// パネル別の内訳（入力のパネルの順）
    pub panels: Vec<SolarSunlitPanel>,
    /// 評価に使った日照中のサンプル数（方向のケースは 1）。0 なら値はすべて 0
    pub sunlit_samples: usize,
}

/// 1 パネルの内訳。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SolarSunlitPanel {
    pub id: String,
    /// 日照中の平均発電（BOL）
    pub bol: Qty<Power>,
    /// 日照中の平均発電（EOL）
    pub eol: Qty<Power>,
    /// 法線と太陽方向のなす角（日照中の平均）
    pub mean_sun_angle: Qty<Angle>,
}

/// 日照時の発電。
pub struct SolarSunlit;

impl Analysis for SolarSunlit {
    const ID: &'static str = "power.solar_sunlit";
    const TITLE: &'static str = "日照時の発電";
    const AREA: AnalysisArea = AnalysisArea::System(System::Power);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "姿勢則ごとの日照時の発電電力（BOL/EOL）";
    const TITLE_EN: &'static str = "Sunlit power generation";
    const DESCRIPTION_EN: &'static str =
        "Solar array output in sunlight for each attitude case (BOL/EOL)";
    type Input = SolarSunlitInput;
    type Output = SolarSunlitOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        input.array.validate()?;
        let cases = input
            .cases
            .iter()
            .map(|c| evaluate_case(&input.array, c))
            .collect::<Result<_, _>>()?;
        Ok(SolarSunlitOutput { cases })
    }
}

/// 太陽方向（機体座標系）と太陽光の強さの組。
type SunPoint = (Vec3, Qty<sat_units::typed::Irradiance>);

/// 評価する標本（時刻 [s]・太陽方向と強さ・日照中か）。
struct SunSamples {
    t: Vec<f64>,
    points: Vec<SunPoint>,
    lit: Vec<bool>,
}

fn evaluate_case(
    array: &SolarArray,
    case: &SolarSunlitCase,
) -> Result<SolarSunlitCaseResult, AnalysisError> {
    let samples = match &case.sun {
        SolarSunlitSun::Direction { sun_dir_body } => {
            if sun_dir_body.normalized().is_none() {
                return Err(AnalysisError::InvalidInput(format!(
                    "Sun direction of case {} is a zero vector",
                    case.id
                )));
            }
            SunSamples {
                t: vec![0.0],
                points: vec![(*sun_dir_body, array.flux(None))],
                lit: vec![true],
            }
        }
        SolarSunlitSun::Timeline { timeline: spec } => {
            let tl = timeline::build(spec)?;
            SunSamples {
                t: tl.iter().map(|s| s.t_s).collect(),
                points: tl
                    .iter()
                    .map(|s| (s.sun_dir_body(), array.flux(Some(s.sun_distance_m))))
                    .collect(),
                lit: tl.iter().map(is_sunlit).collect(),
            }
        }
    };
    Ok(summarize(array, case, &samples))
}

/// 日照中の時間平均（台形則、`crate::spec::masked_time_mean`）と最小・最大。
fn summarize(array: &SolarArray, case: &SolarSunlitCase, s: &SunSamples) -> SolarSunlitCaseResult {
    let n = array.panels.len();
    let mut bol = vec![Vec::with_capacity(s.t.len()); n];
    let mut eol = vec![Vec::with_capacity(s.t.len()); n];
    let mut angle = vec![Vec::with_capacity(s.t.len()); n];
    let (mut eol_min, mut eol_max) = (f64::INFINITY, 0.0_f64);
    for (&(sun, flux), &lit) in s.points.iter().zip(&s.lit) {
        let pb = array.panel_powers(sun, 1.0, flux, SolarLife::Bol);
        let pe = array.panel_powers(sun, 1.0, flux, SolarLife::Eol);
        if lit {
            let te = total(&pe).value();
            eol_min = eol_min.min(te);
            eol_max = eol_max.max(te);
        }
        for i in 0..n {
            bol[i].push(pb[i].value());
            eol[i].push(pe[i].value());
            angle[i].push(array.panels[i].normal.angle_to(sun));
        }
    }
    let mean = |v: &[f64]| masked_time_mean(&s.t, v, &s.lit).unwrap_or(0.0);
    let panels: Vec<SolarSunlitPanel> = array
        .panels
        .iter()
        .enumerate()
        .map(|(i, p)| SolarSunlitPanel {
            id: p.id.clone(),
            bol: Qty::from_raw(mean(&bol[i])),
            eol: Qty::from_raw(mean(&eol[i])),
            mean_sun_angle: Qty::from_raw(mean(&angle[i])),
        })
        .collect();
    let lit_count = s.lit.iter().filter(|&&l| l).count();
    SolarSunlitCaseResult {
        id: case.id.clone(),
        label: case.label.clone(),
        bol: panels.iter().fold(Qty::ZERO, |a, p| a + p.bol),
        eol: panels.iter().fold(Qty::ZERO, |a, p| a + p.eol),
        eol_min: Qty::from_raw(if lit_count == 0 { 0.0 } else { eol_min }),
        eol_max: Qty::from_raw(eol_max),
        panels,
        sunlit_samples: lit_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::power::solar::SolarPanel;

    fn input(sun: Vec3) -> SolarSunlitInput {
        let panel = |id: &str, normal: Vec3| SolarPanel {
            id: id.to_owned(),
            area: Qty::from_raw(1.0),
            cell_efficiency: 0.3,
            packing_factor: Some(1.0),
            eol_factor: Some(0.8),
            normal,
        };
        SolarSunlitInput {
            array: SolarArray {
                panels: vec![panel("PZ", Vec3::Z), panel("PX", Vec3::X)],
                solar_flux: None,
                conversion_efficiency: None,
            },
            cases: vec![SolarSunlitCase {
                id: "sun_z".to_owned(),
                label: "+Z Sun pointing".to_owned(),
                sun: SolarSunlitSun::Direction { sun_dir_body: sun },
            }],
        }
    }

    #[test]
    fn direction_case_by_hand() {
        // +Z 面に垂直入射: 1 m² × 0.3 × 1361 = 408.3 W、+X 面は 0
        let out = SolarSunlit::run(&input(Vec3::Z)).unwrap();
        let c = &out.cases[0];
        assert!((c.bol.value() - 408.3).abs() < 1e-9);
        assert!((c.eol.value() - 326.64).abs() < 1e-9);
        assert_eq!(c.eol_min, c.eol_max);
        assert_eq!(c.panels[1].bol.value(), 0.0);
        assert!((c.panels[1].mean_sun_angle.value() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    }

    #[test]
    fn diagonal_sun_on_two_panels() {
        // 45° で 2 面に当たる: 2 × 408.3 × cos 45° = 577.4 W
        let out = SolarSunlit::run(&input(Vec3::new(1.0, 0.0, 1.0))).unwrap();
        let expect = 2.0 * 408.3 * std::f64::consts::FRAC_1_SQRT_2;
        assert!((out.cases[0].bol.value() - expect).abs() < 1e-9);
    }

    #[test]
    fn zero_direction_is_rejected() {
        assert!(SolarSunlit::run(&input(Vec3::ZERO)).is_err());
    }

    #[test]
    fn timeline_cases_sun_and_nadir_pointing() {
        use crate::power::testkit::{leo_timeline, nadir_pointing_z, sun_pointing_z};
        // 太陽指向は日照中ずっと +Z に垂直入射 → 408.3 W（太陽光の強さは 1361 W/m² に固定）。
        // 地心指向は +Z が日照中に太陽を向かない時間があるので小さい
        let mut i = input(Vec3::Z);
        i.array.solar_flux = Some(Qty::from_raw(1361.0));
        i.cases = [("sun", sun_pointing_z()), ("nadir", nadir_pointing_z())]
            .into_iter()
            .map(|(id, law)| SolarSunlitCase {
                id: id.to_owned(),
                label: String::new(),
                sun: SolarSunlitSun::Timeline {
                    timeline: Box::new(leo_timeline(law, 1.0, 30.0)),
                },
            })
            .collect();
        let out = SolarSunlit::run(&i).unwrap();
        assert!((out.cases[0].bol.value() - 408.3).abs() < 1e-6);
        assert!(out.cases[0].sunlit_samples > 0);
        assert!(out.cases[1].bol.value() < 408.3);
        assert!(out.cases[1].eol_min.value() <= out.cases[1].eol_max.value());
    }
}
