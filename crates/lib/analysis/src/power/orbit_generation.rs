//! 軌道上の発電（`power.orbit_generation`）。
//!
//! 軌道と姿勢則の時系列（`sat_astro::timeline`）に沿って発電を求める。発電モデルは [`super::solar`]。
//! 軌道平均は回り切った周回（[`crate::spec::orbit_ranges`]）のサンプルの平均（1 周未満なら全期間）。

use super::solar::{SolarArray, SolarLife, SolarProfile, eclipses};
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Channel, ChannelKind, Series,
    SpanFallback, System, longest_window, masked_time_mean, orbit_mean, orbit_ranges, span_note,
    time_fraction,
};
use sat_astro::orbit::Cycle;
use sat_astro::timeline::{self, TimelineSpec, Window};
use sat_units::typed::{Energy, Fraction, Power, Qty, Time};
use serde::{Deserialize, Serialize};

/// 軌道上の発電の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrbitGenerationInput {
    pub timeline: TimelineSpec,
    pub array: SolarArray,
    /// 時系列と平均に使う寿命の時点（省略時 EOL。[`SolarLife::default`]）。BOL・EOL の軌道平均はどちらも出す
    #[serde(default)]
    pub life: Option<SolarLife>,
}

impl OrbitGenerationInput {
    /// 使う寿命の時点（省略時は [`SolarLife::default`]）。出力の `life` に載る。
    pub fn life(&self) -> SolarLife {
        self.life.unwrap_or_default()
    }
}

/// 軌道上の発電の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrbitGenerationOutput {
    /// 時系列と平均に使った寿命の時点
    pub life: SolarLife,
    /// 軌道周期
    pub period: Qty<Time>,
    /// 回り切った周回の数（0 なら平均は全期間）
    pub complete_orbits: usize,
    /// 軌道平均の発電（`life`）
    pub orbit_average: Qty<Power>,
    pub orbit_average_bol: Qty<Power>,
    pub orbit_average_eol: Qty<Power>,
    /// 日照中の平均発電（`life`）
    pub sunlit_average: Qty<Power>,
    /// 最大の発電（`life`）
    pub peak: Qty<Power>,
    /// 1 周あたりの発電量（軌道平均 × 周期）
    pub energy_per_orbit: Qty<Energy>,
    /// 日照の割合（0〜1、回り切った周回）
    pub sunlit_fraction: Fraction,
    /// 最長の日陰
    pub max_eclipse_duration: Qty<Time>,
    /// 日陰の区間（端はサンプル時刻）
    pub eclipses: Vec<Window>,
    /// パネル別の寄与（入力のパネルの順）
    pub panels: Vec<OrbitGenerationPanel>,
    /// 時系列: `generation`・`sunlit`・`panel:<id>`
    pub series: Series,
    /// パネルの面の日照割合（ほかの部品の影を引いた割合）の、日照中の標本での最小と平均。影を数えたときだけ付く
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadowing: Option<crate::structure::external_geometry::ShadowingReport>,
    /// 1 周に満たないときの代わりの集計（回り切った周回があれば `None`。service が注意 `span` にする）
    pub span_fallback: Option<SpanFallback>,
}

/// 1 パネルの寄与。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrbitGenerationPanel {
    pub id: String,
    /// 軌道平均の発電（`life`）
    pub orbit_average: Qty<Power>,
    /// 組全体の軌道平均に対する割合（0〜1）
    pub share: Fraction,
}

/// 軌道上の発電。
pub struct OrbitGeneration;

impl Analysis for OrbitGeneration {
    const ID: &'static str = "power.orbit_generation";
    const TITLE: &'static str = "軌道上の発電";
    const AREA: AnalysisArea = AnalysisArea::System(System::Power);
    const KIND: AnalysisKind = AnalysisKind::TimeSeries;
    const DESCRIPTION: &'static str = "軌道 1〜N 周の発電電力と日陰の時系列";
    const TITLE_EN: &'static str = "Orbital power generation";
    const DESCRIPTION_EN: &'static str =
        "Time series of generated power and eclipse over one or more orbits";
    type Input = OrbitGenerationInput;
    type Output = OrbitGenerationOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        input.array.validate()?;
        let samples = timeline::build(&input.timeline)?;
        let cycle = input.timeline.orbit().cycle();
        let profile = SolarProfile::build(&input.array, &samples);
        Ok(summarize(input, &profile, eclipses(&samples), cycle))
    }
}

/// 発電の時系列を集計する。
fn summarize(
    input: &OrbitGenerationInput,
    profile: &SolarProfile,
    eclipses: Vec<Window>,
    cycle: Cycle,
) -> OrbitGenerationOutput {
    let life = input.life();
    let period_s = cycle.length_s();
    let ranges = orbit_ranges(&profile.t, cycle);
    let gen_life = profile.total(life);
    let flags = &profile.lit;
    let orbit_average = orbit_mean(&profile.t, &gen_life, &ranges);
    let panels = panel_shares(input, profile, &ranges, orbit_average);
    OrbitGenerationOutput {
        life,
        period: Qty::from_raw(period_s),
        complete_orbits: ranges.len(),
        orbit_average: Qty::from_raw(orbit_average),
        orbit_average_bol: Qty::from_raw(orbit_mean(
            &profile.t,
            &profile.total(SolarLife::Bol),
            &ranges,
        )),
        orbit_average_eol: Qty::from_raw(orbit_mean(
            &profile.t,
            &profile.total(SolarLife::Eol),
            &ranges,
        )),
        sunlit_average: Qty::from_raw(
            masked_time_mean(&profile.t, &gen_life, flags).unwrap_or(0.0),
        ),
        peak: Qty::from_raw(gen_life.iter().fold(0.0, |a: f64, &p| a.max(p))),
        energy_per_orbit: Qty::<Power>::from_raw(orbit_average) * Qty::<Time>::from_raw(period_s),
        sunlit_fraction: time_fraction(&profile.t, flags, &ranges),
        max_eclipse_duration: longest_window(&eclipses),
        eclipses,
        panels,
        series: generation_series(input, profile, gen_life),
        shadowing: profile.shadowing.clone(),
        span_fallback: span_note(cycle, ranges.len(), SpanFallback::OrbitMeanOverSpan),
    }
}

/// 時系列: 発電・日照率・パネル別の発電。
fn generation_series(
    input: &OrbitGenerationInput,
    profile: &SolarProfile,
    gen_life: Vec<f64>,
) -> Series {
    let life = input.life();
    let mut channels = vec![
        Channel::of::<Power>("generation", ChannelKind::Generation, gen_life),
        Channel::of::<Fraction>("sunlit", ChannelKind::Sunlit, profile.sunlit.clone()),
    ];
    for (i, p) in input.array.panels.iter().enumerate() {
        channels.push(
            Channel::of::<Power>(
                format!("panel:{}", p.id),
                ChannelKind::PanelGeneration,
                profile.panel(i, life),
            )
            .about(&p.id),
        );
    }
    Series {
        t: profile.t.clone(),
        channels,
    }
}
fn panel_shares(
    input: &OrbitGenerationInput,
    profile: &SolarProfile,
    ranges: &[std::ops::Range<usize>],
    orbit_average: f64,
) -> Vec<OrbitGenerationPanel> {
    input
        .array
        .panels
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let avg = orbit_mean(&profile.t, &profile.panel(i, input.life()), ranges);
            OrbitGenerationPanel {
                id: p.id.clone(),
                orbit_average: Qty::from_raw(avg),
                share: Fraction::new(if orbit_average > 0.0 {
                    avg / orbit_average
                } else {
                    0.0
                }),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::power::solar::SolarPanel;
    use crate::power::testkit::{leo, leo_timeline, sun_pointing_z};
    use sat_astro::constants::R_EARTH;
    use sat_astro::math::Vec3;
    use sat_astro::orbit::beta_angle_rad;

    fn array() -> SolarArray {
        SolarArray {
            panels: vec![SolarPanel {
                id: "PZ".to_owned(),
                area: Qty::from_raw(1.0),
                cell_efficiency: 0.3,
                packing_factor: Some(1.0),
                eol_factor: Some(0.8),
                normal: Vec3::Z,
            }],
            solar_flux: Some(Qty::from_raw(1361.0)),
            conversion_efficiency: None,
            shadowing: None,
        }
    }

    fn input(orbits: f64) -> OrbitGenerationInput {
        OrbitGenerationInput {
            timeline: leo_timeline(sun_pointing_z(), orbits, 10.0),
            array: array(),
            life: Some(SolarLife::Bol),
        }
    }

    #[test]
    fn synthetic_profile_by_hand() {
        // 10 s 刻み、周期 100 s のうち t = 0..50 が日照、+Z 面 1 枚に垂直入射（408.3 W）
        let t: Vec<f64> = (0..=20).map(|k| f64::from(k) * 10.0).collect();
        let sunlit: Vec<f64> = t
            .iter()
            .map(|&x| if x % 100.0 < 60.0 { 1.0 } else { 0.0 })
            .collect();
        let mut p = SolarProfile::build(&array(), &[]);
        p.panel_bol = vec![sunlit.iter().map(|s| 408.3 * s).collect()];
        p.t = t;
        p.lit = sunlit.iter().map(|&s| s > 0.5).collect();
        p.sunlit = sunlit;
        let ecl = vec![Window {
            start: Qty::from_raw(55.0),
            end: Qty::from_raw(95.0),
            duration: Qty::from_raw(40.0),
        }];
        let out = summarize(&input(1.0), &p, ecl, Cycle::Periodic { period_s: 100.0 });
        assert_eq!(out.complete_orbits, 2);
        // 1 周 10 サンプルのうち 6 サンプルが日照 → 408.3 × 0.6 = 244.98 W
        assert!((out.orbit_average.value() - 244.98).abs() < 1e-9);
        assert!((out.orbit_average_eol.value() - 244.98 * 0.8).abs() < 1e-9);
        assert!((out.sunlit_average.value() - 408.3).abs() < 1e-9);
        assert!((out.sunlit_fraction.value() - 0.6).abs() < 1e-12);
        assert!((out.energy_per_orbit.value() - 24498.0).abs() < 1e-6);
        assert!((out.max_eclipse_duration.value() - 40.0).abs() < 1e-12);
        assert!((out.panels[0].share.value() - 1.0).abs() < 1e-12);
        assert_eq!(out.span_fallback, None);
    }

    #[test]
    fn sun_pointing_leo_against_cylindrical_eclipse_formula() {
        // 太陽指向で +Z 面は日照中つねに垂直入射 → 日照中の平均 = 408.3 W。
        // 日陰の割合（円柱影）= acos(√(h² + 2Rh) / (r cos β)) / π
        let out = OrbitGeneration::run(&input(2.0)).unwrap();
        assert_eq!(out.complete_orbits, 2);
        assert!(
            (out.sunlit_average.value() - 408.3).abs() < 1e-6,
            "{}",
            out.sunlit_average.value()
        );
        let orbit = leo();
        let beta = beta_angle_rad(&orbit, orbit.epoch).unwrap();
        let r = orbit.elements.semi_major_axis.value();
        let h = r - R_EARTH;
        let eclipse_fraction =
            ((h * h + 2.0 * R_EARTH * h).sqrt() / (r * beta.cos())).acos() / std::f64::consts::PI;
        let period = orbit.period_s();
        let expect = eclipse_fraction * period;
        assert!(
            (out.max_eclipse_duration.value() - expect).abs() < 0.01 * expect,
            "{} vs {expect}",
            out.max_eclipse_duration.value()
        );
        assert!((out.sunlit_fraction.value() - (1.0 - eclipse_fraction)).abs() < 0.01);
        let avg = 408.3 * (1.0 - eclipse_fraction);
        assert!((out.orbit_average.value() - avg).abs() < 0.01 * avg);
        assert_eq!(out.series.channels.len(), 3);
        assert_eq!(out.series.t.len(), out.series.channels[0].values.len());
    }

    #[test]
    fn beta_zero_gives_the_longest_eclipse() {
        // β = 0（TimelineSpec.beta）で円柱影の日陰は 2·asin(R/r)/(2π) 周 = asin(R/r)/π
        let mut i = input(1.0);
        i.timeline.sampling.beta = Some(Qty::from_raw(0.0));
        let out = OrbitGeneration::run(&i).unwrap();
        let r = i
            .timeline
            .orbit()
            .elements()
            .expect("elements")
            .elements
            .semi_major_axis
            .value();
        let expect = (R_EARTH / r).asin() / std::f64::consts::PI * out.period.value();
        let got = out.max_eclipse_duration.value();
        assert!((got - expect).abs() < 0.01 * expect, "{got} vs {expect}");
    }

    #[test]
    fn short_span_falls_back_to_the_whole_span() {
        let out = OrbitGeneration::run(&input(0.5)).unwrap();
        assert_eq!(out.complete_orbits, 0);
        assert_eq!(out.span_fallback, Some(SpanFallback::OrbitMeanOverSpan));
        assert_eq!(
            OrbitGeneration::run(&input(1.5)).unwrap().span_fallback,
            None
        );
    }

    /// 太陽を回る円軌道（0.7・1.0・1.5 AU）で太陽指向: 食は無く（日照率 1）、発電は太陽光 S₁(AU/d)² に比例する。
    /// 手計算: 面積 1 m²・効率 0.3・充填率 1 の +Z 面に垂直入射で P = 0.3 · 1361 · (AU/d)²（太陽光は距離から決める）。
    #[test]
    fn heliocentric_generation_scales_with_inverse_square_distance() {
        use sat_astro::constants::AU;
        use sat_astro::ephem::CentralBody;
        use sat_astro::orbit::{OrbitElements, OrbitSpec, Propagator};
        use sat_astro::time::Epoch;
        use sat_astro::timeline::TimelineSpec;
        let mut a = array();
        a.solar_flux = None;
        for d_au in [0.7_f64, 1.0, 1.5] {
            let elements = OrbitElements::with_defaults(
                Qty::from_raw(d_au * AU),
                Qty::from_raw(0.1),
                None,
                None,
                None,
                None,
            );
            let mut orbit = OrbitSpec::new(
                Epoch::parse("2030-01-01T00:00:00Z").unwrap(),
                elements,
                Propagator::Kepler,
            );
            orbit.central = CentralBody::Sun;
            let period = orbit.period_s();
            let timeline = TimelineSpec::new(
                orbit,
                sun_pointing_z(),
                Qty::from_raw(2.0 * period),
                Qty::from_raw(period / 100.0),
            );
            let input = OrbitGenerationInput {
                timeline,
                array: a.clone(),
                life: Some(SolarLife::Bol),
            };
            let out = OrbitGeneration::run(&input).unwrap();
            let expect = 0.3 * 1361.0 / (d_au * d_au);
            assert!(
                (out.orbit_average.value() / expect - 1.0).abs() < 1e-9,
                "{d_au} AU: {} vs {expect}",
                out.orbit_average.value()
            );
            assert!((out.sunlit_fraction.value() - 1.0).abs() < 1e-12);
            assert_eq!(out.max_eclipse_duration.value(), 0.0);
        }
    }
}
