//! 局ごとの「回線が閉じる最低仰角」と、それを重ねた局の一覧（`comm.data_throughput` の部品）。

use super::{
    DataThroughputInput, DataThroughputLink, DataThroughputLinkClosure,
    DataThroughputStationClosure,
};
use crate::comm::passes::SearchWindow;
use crate::spec::AnalysisError;
use sat_astro::ephem::CentralBody;
use sat_astro::ground::{EarthRange, GroundStation, earth_range};
use sat_astro::orbit::OrbitSpec;
use sat_units::typed::{Angle, Qty};

/// 局に使う回線の条件（その局の専用）。
fn link_for<'a>(input: &'a DataThroughputInput, station: &str) -> Option<&'a DataThroughputLink> {
    input
        .station_links
        .iter()
        .find(|s| s.station == station)
        .map(|s| &s.link)
}

/// 局ごとの、回線が閉じる最低仰角。回線の条件が無い局は [`DataThroughputLinkClosure::NotEvaluated`]。
/// 帯域の設定が無い局（`stations_without_band`）は最後に [`DataThroughputLinkClosure::NoBandSettings`] で並べる。
///
/// 中心天体が地球でないときは仰角で距離が変わらない（距離は地球から衛星まで）ので、窓（解析の 1 日）の最大の距離で
/// 回線余裕を判定し、足りれば水平線（仰角 0）から閉じ、足りなければ `NeverCloses`。ミッション期間全体の最大距離での判定は
/// `comm.link_budget` で見る。
pub(super) fn station_closures(
    input: &DataThroughputInput,
    w: &SearchWindow,
) -> Result<Vec<DataThroughputStationClosure>, AnalysisError> {
    // 高度は平均（長半径 − 中心天体の半径）で代表させる。
    let altitude = w.orbit.altitude_m();
    let far = (w.orbit.central != CentralBody::Earth && !input.station_links.is_empty())
        .then(|| earth_range(&w.orbit, w.start, w.span_s, w.step_s))
        .transpose()?;
    let counted = input.stations.iter().map(|st| {
        let closure = match link_for(input, &st.id) {
            None => DataThroughputLinkClosure::NotEvaluated,
            Some(link) => {
                if altitude <= 0.0 {
                    return Err(AnalysisError::InvalidInput(
                        "Orbit altitude is negative".to_owned(),
                    ));
                }
                closure_of(input, link, &w.orbit, altitude, far.as_ref())
            }
        };
        Ok(DataThroughputStationClosure {
            station: st.id.clone(),
            closure,
        })
    });
    let without_band = input.stations_without_band.iter().map(|id| {
        Ok(DataThroughputStationClosure {
            station: id.clone(),
            closure: DataThroughputLinkClosure::NoBandSettings,
        })
    });
    counted.chain(without_band).collect()
}

/// 1 局の回線が閉じる仰角（`far` は地球以外の中心天体のときの、窓の中の地球からの距離の範囲）。
fn closure_of(
    input: &DataThroughputInput,
    link: &DataThroughputLink,
    orbit: &OrbitSpec,
    altitude: f64,
    far: Option<&EarthRange>,
) -> DataThroughputLinkClosure {
    let (rate, required) = (input.downlink_rate.value(), link.required_eb_n0_db);
    let margin = input.required_margin_db();
    let elevation = match far {
        Some(range) => {
            (link.rf.eb_n0_margin_db(range.max_m, rate, required) >= margin.value()).then_some(0.0)
        }
        None => link.rf.min_elevation_for_margin(
            orbit.central.radius_m(),
            altitude,
            rate,
            required,
            margin,
        ),
    };
    match elevation {
        Some(e) => DataThroughputLinkClosure::ClosesAbove {
            min_elevation: Qty::from_raw(e),
        },
        None => DataThroughputLinkClosure::NeverCloses,
    }
}

/// 全体の閉じ方: どれかの局で閉じるなら最も低い仰角、どの局でも閉じなければ `NeverCloses`、
/// どの局も評価していなければ `NotEvaluated`。
pub(super) fn summarize(closures: &[DataThroughputStationClosure]) -> DataThroughputLinkClosure {
    use DataThroughputLinkClosure::{ClosesAbove, NeverCloses, NoBandSettings, NotEvaluated};
    let mut summary = NotEvaluated;
    // 帯域の設定が無い局は数えないので、要約に入れない。
    for c in closures.iter().filter(|c| c.closure != NoBandSettings) {
        summary = match (summary, c.closure) {
            (s, NotEvaluated) => s,
            (NotEvaluated | NeverCloses, c) => c,
            (ClosesAbove { min_elevation: a }, ClosesAbove { min_elevation: b }) => ClosesAbove {
                min_elevation: if b.value() < a.value() { b } else { a },
            },
            (s @ ClosesAbove { .. }, _) => s,
            (NoBandSettings, c) => c,
        };
    }
    summary
}

/// 数える局（閉じない局は除き、閉じる局は最低仰角を回線が閉じる仰角まで上げる）。
pub(super) fn usable_stations(
    input: &DataThroughputInput,
    closures: &[DataThroughputStationClosure],
) -> Vec<GroundStation> {
    input
        .stations
        .iter()
        .zip(closures)
        .filter_map(|(st, c)| match c.closure {
            DataThroughputLinkClosure::NeverCloses | DataThroughputLinkClosure::NoBandSettings => {
                None
            }
            DataThroughputLinkClosure::NotEvaluated => Some(st.clone()),
            DataThroughputLinkClosure::ClosesAbove { min_elevation } => {
                Some(raise_min_elevation(st, min_elevation))
            }
        })
        .collect()
}

fn raise_min_elevation(station: &GroundStation, min_el: Qty<Angle>) -> GroundStation {
    let mut st = station.clone();
    st.min_elevation = Qty::from_raw(st.min_elevation.value().max(min_el.value()));
    st
}
