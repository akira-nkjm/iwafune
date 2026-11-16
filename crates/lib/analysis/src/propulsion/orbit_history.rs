//! マヌーバの順の軌道の推移（`propulsion.orbit_history`）。数値伝搬のマヌーバは推力を入れて伝搬した軌道の時系列。
//!
//! 推移の作り方は `history`（時刻と軌道の決め方）と `flight`（伝搬）。ここは公称の列（マージンなし）の推移を
//! 時系列（区間の始めごとの高度・傾斜角・質量）にして出す。

use super::history::OrbitHistory;
use super::input::PropulsionInput;
use super::maneuvers::nominal;
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Channel, ChannelKind, Series, System,
};
use sat_units::typed::{Angle, Length, Mass, Qty, Time};
use serde::{Deserialize, Serialize};

/// `propulsion.orbit_history` の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrbitHistoryOutput {
    pub history: OrbitHistory,
    /// 最後のマヌーバが終わる時刻（最初の軌道の元期からの経過）
    pub maneuvers_end: Qty<Time>,
    /// 数値伝搬したマヌーバ（実行の順）
    pub propagated: Vec<String>,
    /// 時系列（区間の始めごと）: `altitude`（平均の長半径 − 赤道半径）・`inclination`・`mass`（推薬を求めたときだけ）
    pub series: Series,
}

/// マヌーバの順の軌道の推移（公称の列。ほかの解析がその時期の軌道を引くのに使う入口）。
pub fn orbit_history(input: &PropulsionInput) -> Result<OrbitHistory, AnalysisError> {
    Ok(nominal(input)?.history)
}

/// マヌーバの順の軌道の推移。
pub struct OrbitHistoryAnalysis;

impl Analysis for OrbitHistoryAnalysis {
    const ID: &'static str = "propulsion.orbit_history";
    const TITLE: &'static str = "軌道の推移";
    const TITLE_EN: &'static str = "Orbit history";
    const AREA: AnalysisArea = AnalysisArea::System(System::Propulsion);
    const KIND: AnalysisKind = AnalysisKind::TimeSeries;
    const DESCRIPTION: &'static str = "マヌーバの順に、いつどの軌道にいるか（数値伝搬のマヌーバは推力を入れた伝搬の高度・傾斜角・質量）";
    const DESCRIPTION_EN: &'static str = "Which orbit the spacecraft is on over time, maneuver by maneuver (altitude, inclination and mass from the thrust propagation for propagated maneuvers)";

    type Input = PropulsionInput;
    type Output = OrbitHistoryOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let seq = nominal(input)?;
        let history = seq.history.clone();
        let segs = &history.segments;
        let t: Vec<f64> = segs.iter().map(|s| s.start.value()).collect();
        let mut channels = vec![
            Channel::of::<Length>(
                "altitude",
                ChannelKind::Altitude,
                segs.iter().map(|s| s.orbit.altitude_m()).collect(),
            ),
            Channel::of::<Angle>(
                "inclination",
                ChannelKind::Inclination,
                segs.iter()
                    .map(|s| s.orbit.elements.inclination.value())
                    .collect(),
            ),
        ];
        let mass: Option<Vec<f64>> = segs
            .iter()
            .enumerate()
            .map(|(i, s)| match (i, s.mass) {
                (0, None) => Some(seq.initial_mass_kg),
                (_, m) => m.map(|m| m.value()),
            })
            .collect();
        if let Some(m) = mass {
            channels.push(Channel::of::<Mass>("mass", ChannelKind::Mass, m));
        }
        let propagated = seq
            .legs
            .iter()
            .zip(&seq.flights)
            .filter(|(_, f)| f.is_some())
            .map(|(l, _)| l.maneuver.id.clone())
            .collect();
        let maneuvers_end = seq.legs.iter().zip(0..).fold(0.0, |t, (l, i)| {
            let f = seq.flights.get(i).and_then(Option::as_ref);
            f.map_or(t + l.transfer_time_s.unwrap_or(0.0), |f| {
                f.start_s + f.arc.duration_s
            })
        });
        Ok(OrbitHistoryOutput {
            history,
            maneuvers_end: Qty::from_raw(maneuvers_end),
            propagated,
            series: Series { t, channels },
        })
    }
}
