//! 地上局の可視（`comm.visibility`）。局ごとのパス（AOS・LOS・最大仰角・時間）と、日ごとの回数・通信時間・最大の空白。
//! パスの検出は `sat_astro::ground::find_passes`（[`crate::comm::passes`] 経由）。

use crate::comm::passes::{
    SearchWindow, day_windows, find_all_passes, overlap_s, pass_interval, search_window,
    union_sorted,
};
use crate::spec::{Analysis, AnalysisArea, AnalysisError, AnalysisKind, System};
use sat_astro::ground::{GroundPass, GroundStation};
use sat_astro::time::Epoch;
use sat_astro::timeline::OrbitSampling;
use sat_units::typed::{Angle, Qty, Time};
use serde::{Deserialize, Serialize};

/// 地上局の可視の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct VisibilityInput {
    /// 軌道・期間・探索の刻み（AOS・LOS は二分法で詰める）・β（時系列と同じ標本の取り方。開始は軌道の元期）
    pub sampling: OrbitSampling,
    pub stations: Vec<GroundStation>,
}

/// 1 日の可視の集計。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct VisibilityDay {
    /// 0 始まりの日の番号
    pub day: u32,
    pub start: Epoch,
    /// その日に AOS があったパスの数
    pub pass_count: u32,
    /// その日の通信時間（日の境界で切って数える）
    pub contact: Qty<Time>,
}

/// 局（または全局の合計）の可視の集計。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct VisibilitySummary {
    /// 局の ID（全局の合計は `"*"`）
    pub station: String,
    pub pass_count: u32,
    /// 期間中の通信時間の合計（全局の合計は同時に見える時間を二重に数えない）
    pub total_contact: Qty<Time>,
    /// 1 日あたりの平均通信時間
    pub contact_per_day: Qty<Time>,
    /// 1 パスの平均時間
    pub mean_pass_duration: Qty<Time>,
    /// 最長のパス
    pub longest_pass: Qty<Time>,
    /// 通信できない時間の最大（期間の始めと終わりの空白を含む）
    pub max_gap: Qty<Time>,
    /// 最大仰角の最大
    pub max_elevation: Qty<Angle>,
    pub days: Vec<VisibilityDay>,
}

/// 地上局の可視の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct VisibilityOutput {
    /// 全局のパス（AOS の順）
    pub passes: Vec<GroundPass>,
    /// 局ごとの集計（入力の局の順）
    pub stations: Vec<VisibilitySummary>,
    /// 全局を合わせた集計
    pub combined: VisibilitySummary,
}

/// 地上局の可視。
pub struct Visibility;

impl Analysis for Visibility {
    const ID: &'static str = "comm.visibility";
    const TITLE: &'static str = "地上局の可視";
    const AREA: AnalysisArea = AnalysisArea::System(System::Comm);
    const KIND: AnalysisKind = AnalysisKind::TimeSeries;
    const DESCRIPTION: &'static str = "地上局ごとのパス（AOS・LOS・最大仰角・時間）";
    const TITLE_EN: &'static str = "Ground station visibility";
    const DESCRIPTION_EN: &'static str =
        "Passes per ground station (AOS, LOS, maximum elevation, duration)";
    type Input = VisibilityInput;
    type Output = VisibilityOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let w = search_window(&input.sampling)?;
        let passes = find_all_passes(&w.orbit, &input.stations, w.start, w.span_s, w.step_s)?;
        let stations = input
            .stations
            .iter()
            .map(|st| {
                let own: Vec<&GroundPass> = passes.iter().filter(|p| p.station == st.id).collect();
                summarize(&st.id, &own, &w, Contact::PerPass)
            })
            .collect();
        let all: Vec<&GroundPass> = passes.iter().collect();
        let combined = summarize("*", &all, &w, Contact::Union);
        Ok(VisibilityOutput {
            passes,
            stations,
            combined,
        })
    }
}

/// 通信時間の数え方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Contact {
    /// パスごとの時間を足す（1 局の集計。1 局のパスは重ならない）
    PerPass,
    /// 重なる区間をつなげてから足す（全局の集計。同時に見える時間を二重に数えない）
    Union,
}

/// パスの一覧（AOS の順）を集計する。通信時間の数え方は `contact`。
fn summarize(
    station: &str,
    passes: &[&GroundPass],
    w: &SearchWindow,
    contact: Contact,
) -> VisibilitySummary {
    let (start, span_s) = (w.start, w.span_s);
    let raw: Vec<(f64, f64)> = passes
        .iter()
        .map(|p| pass_interval(p, start, span_s))
        .collect();
    let intervals = match contact {
        Contact::Union => union_sorted(&raw),
        Contact::PerPass => raw.clone(),
    };
    let total: f64 = intervals.iter().map(|(s, e)| e - s).sum();
    let longest = raw.iter().map(|(s, e)| e - s).fold(0.0, f64::max);
    let max_el = passes
        .iter()
        .map(|p| p.max_elevation.value())
        .fold(0.0, f64::max);
    let days = day_windows(span_s);
    let day_summaries = days
        .iter()
        .enumerate()
        .map(|(i, &w)| VisibilityDay {
            day: i as u32,
            start: start.plus_seconds(w.0),
            pass_count: raw.iter().filter(|(s, _)| *s >= w.0 && *s < w.1).count() as u32,
            contact: Qty::from_raw(intervals.iter().map(|&iv| overlap_s(iv, w)).sum()),
        })
        .collect();
    let count = passes.len();
    VisibilitySummary {
        station: station.to_owned(),
        pass_count: count as u32,
        total_contact: Qty::from_raw(total),
        contact_per_day: Qty::from_raw(total / (span_s / sat_astro::constants::SECONDS_PER_DAY)),
        mean_pass_duration: Qty::from_raw(if count > 0 {
            raw.iter().map(|(s, e)| e - s).sum::<f64>() / count as f64
        } else {
            0.0
        }),
        longest_pass: Qty::from_raw(longest),
        max_gap: Qty::from_raw(max_gap_s(&union_sorted(&raw), span_s)),
        max_elevation: Qty::from_raw(max_el),
        days: day_summaries,
    }
}

/// 重ならない昇順の区間の間の空白の最大 [s]（期間の始めと終わりを含む）。
fn max_gap_s(intervals: &[(f64, f64)], span_s: f64) -> f64 {
    let mut prev_end = 0.0;
    let mut gap: f64 = 0.0;
    for &(s, e) in intervals {
        gap = gap.max(s - prev_end);
        prev_end = e;
    }
    gap.max(span_s - prev_end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comm::passes::fixtures::{one_day, sso_orbit, tokyo};

    #[test]
    fn gaps_include_edges() {
        assert_eq!(max_gap_s(&[], 100.0), 100.0);
        assert_eq!(max_gap_s(&[(10.0, 20.0), (50.0, 60.0)], 100.0), 40.0);
        assert_eq!(max_gap_s(&[(0.0, 20.0), (25.0, 90.0)], 100.0), 10.0);
    }

    /// 高度 550 km の SSO を中緯度の局で 1 日見ると、パスは数回（概ね 3〜6 回）、1 回は 15 分以下。
    #[test]
    fn sso_over_mid_latitude_station() {
        let out = Visibility::run(&VisibilityInput {
            sampling: one_day(sso_orbit()),
            stations: vec![tokyo()],
        })
        .unwrap();
        let s = &out.stations[0];
        assert!((2..=8).contains(&s.pass_count), "{}", s.pass_count);
        assert!(s.longest_pass.value() < 15.0 * 60.0);
        assert!(s.max_elevation.value() <= std::f64::consts::FRAC_PI_2);
        assert_eq!(s.days.len(), 1);
        assert!((s.days[0].contact.value() - s.total_contact.value()).abs() < 1e-6);
        assert_eq!(out.combined.pass_count, s.pass_count);
    }

    /// 同じ場所の局を 2 つ並べると、全局の通信時間は 1 局ぶん（重なりを二重に数えない）でパスの数は 2 倍。
    #[test]
    fn combined_does_not_double_count() {
        let mut twin = tokyo();
        twin.id = "TKY2".to_owned();
        let out = Visibility::run(&VisibilityInput {
            sampling: one_day(sso_orbit()),
            stations: vec![tokyo(), twin],
        })
        .unwrap();
        let one = &out.stations[0];
        assert_eq!(out.combined.pass_count, 2 * one.pass_count);
        assert!((out.combined.total_contact.value() - one.total_contact.value()).abs() < 1e-6);
        assert!((out.combined.max_gap.value() - one.max_gap.value()).abs() < 1e-6);
    }

    /// β を指定すると RAAN を置き換えた軌道でパスを探す（`OrbitSampling::resolved_orbit` の軌道で探したのと一致し、
    /// β を指定しないときとはパスの時刻が変わる）。
    #[test]
    fn beta_reaches_the_pass_search() {
        let run = |sampling: OrbitSampling| {
            Visibility::run(&VisibilityInput {
                sampling,
                stations: vec![tokyo()],
            })
            .unwrap()
        };
        let mut sampling = one_day(sso_orbit());
        sampling.beta = Some(Qty::from_raw(60f64.to_radians()));
        let with_beta = run(sampling.clone());
        let direct = run(one_day(sampling.resolved_orbit().unwrap()));
        assert_eq!(with_beta.passes.len(), direct.passes.len());
        let contact = |o: &VisibilityOutput| o.combined.total_contact.value();
        assert!((contact(&with_beta) - contact(&direct)).abs() < 1e-9);
        let first = |o: &VisibilityOutput| o.passes.first().map(|p| p.aos.j2000_s);
        assert_ne!(first(&run(one_day(sso_orbit()))), first(&with_beta));
    }
}
