//! 格子の点 1 つ（出発の日・到着の日の組）の評価。**遷移の解き方も ΔV の数え方も `propulsion.maneuvers` と同じ関数**:
//! マヌーバの `transfer` にその 2 つの日付を入れて [`plan`]（→ `sat_astro::transfer::dated_transfer` と、
//! 駐機軌道からの脱出・目標の軌道への捕獲の噴射）を呼ぶだけで、Lambert の設定も ΔV の式もここには無い。

use super::super::input::{DatedTransferInput, ManeuverInput};
use super::super::plan::{Ideal, Orbit, plan};
use crate::spec::AnalysisError;
use sat_astro::ephem::EphemerisSource;
use sat_astro::time::Epoch;

/// 1 つの日付の組の結果（SI）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct CellValues {
    pub time_of_flight_s: f64,
    pub c3_m2_s2: f64,
    pub v_inf_departure_m_s: f64,
    pub v_inf_arrival_m_s: f64,
    pub declination_rad: f64,
    /// 出発の脱出 + 到着の捕獲の ΔV（損失を除く。`ManeuverResult.ideal_delta_v` と同じ）
    pub delta_v_m_s: f64,
    pub ephemeris: EphemerisSource,
}

/// マヌーバ 1 つの評価器（前の軌道と、`transfer` だけを差し替えるマヌーバ）。
pub(super) struct Evaluator {
    from: Orbit,
    maneuver: ManeuverInput,
}

impl Evaluator {
    pub fn new(from: Orbit, maneuver: &ManeuverInput) -> Self {
        Self {
            from,
            maneuver: maneuver.clone(),
        }
    }

    /// 出発の日 `departure`・到着の日 `arrival` での値。解けなければ（暦の外・面が決まらない・到着が出発より前など）エラー。
    pub fn at(&mut self, departure: Epoch, arrival: Epoch) -> Result<CellValues, AnalysisError> {
        // 出発の噴射を誰が行うか（`departure_by`）は、書かれたものを保つ（打ち上げ機なら捕獲だけを数える）
        let departure_by = self
            .maneuver
            .transfer
            .map(|t| t.departure_by)
            .unwrap_or_default();
        self.maneuver.transfer = Some(DatedTransferInput {
            departure,
            arrival,
            departure_by,
        });
        let planned = plan(self.from, &self.maneuver)?;
        let (Some(d), Ideal::Impulsive(t)) = (planned.dated, &planned.ideal) else {
            return Err(AnalysisError::InvalidInput(format!(
                "Maneuver {}: the launch window search needs a dated transfer between bodies",
                self.maneuver.id
            )));
        };
        Ok(CellValues {
            time_of_flight_s: d.transfer_time_s,
            c3_m2_s2: d.c3_m2_s2,
            v_inf_departure_m_s: d.v_inf_departure_m_s.norm(),
            v_inf_arrival_m_s: d.v_inf_arrival_m_s.norm(),
            declination_rad: d.departure_declination_rad,
            // `sequence::apply` と同じ順・同じ足し方（0 から噴射の順に）
            delta_v_m_s: t.burns.iter().fold(0.0, |sum, b| sum + b.delta_v_m_s),
            ephemeris: d.ephemeris,
        })
    }
}
