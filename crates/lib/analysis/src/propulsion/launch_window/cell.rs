//! 格子の点 1 つ（出発の日・到着の日の組）の評価。**遷移の解き方も ΔV の数え方も `propulsion.maneuvers` と同じ関数**:
//! マヌーバの `transfer` にその 2 つの日付を入れて [`plan`]（→ `sat_astro::transfer::dated_transfer` と、
//! パーキング軌道からの脱出・目標の軌道への捕獲の噴射）を呼ぶだけで、Lambert の設定も ΔV の式もここには無い。

use super::super::input::{DatedTransferInput, FlybyInput, ManeuverInput};
use super::super::plan::{Ideal, Orbit, plan};
use crate::spec::AnalysisError;
use sat_astro::ephem::EphemerisSource;
use sat_astro::time::Epoch;
use sat_astro::transfer::BALLISTIC_DV_M_S;

/// 1 つの日付の組の結果（SI）。
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CellValues {
    pub time_of_flight_s: f64,
    pub c3_m2_s2: f64,
    pub v_inf_departure_m_s: f64,
    pub v_inf_arrival_m_s: f64,
    pub declination_rad: f64,
    /// 出発の脱出 + 到着の捕獲の ΔV（損失を除く。`ManeuverResult.ideal_delta_v` と同じ）
    pub delta_v_m_s: f64,
    pub ephemeris: EphemerisSource,
    /// 通過の日（出発から到着の順。通過が無ければ空）
    pub flyby_dates: Vec<Epoch>,
    /// どの通過も、下限の高度で偏向が実現できるか（通過が無ければ真）。偽の点は探索から除く
    pub feasible: bool,
    /// 探索の目安（[m/s]）: ΔV に、動力なしと書かれて ΔV に数えていない通過の「足りない偏向の分」を足したもの。
    /// 実現できない日付でも連続に増えるので、探索が実現できる範囲の縁を伝って下りられる（値として出すのは実現できる点だけ）
    pub guide_m_s: f64,
    /// 選ぶ基準（[m/s]）: ΔV に、動力なしと書かれた通過の数えていない近点の噴射（`uncounted_flyby_dv_m_s`）を足したもの。
    /// 動力なしの通過の超過速度が揃う（弾道の）日付へ探索を向ける。動力ありの通過・直接の遷移では ΔV と同じ。出す ΔV は足さない値
    pub objective_m_s: f64,
    /// 動力なしと書かれて実現できる通過のどれかが、`propulsion.maneuvers` の警告 `swing_by_unpowered` と同じ条件（近点の噴射 > `BALLISTIC_DV_M_S`）か
    pub not_ballistic: bool,
    /// 動力なしと書かれた通過の、数えていない近点の噴射の ΔV の合計 [m/s]（入りと出の超過速度の大きさの差。動力ありの通過は ΔV に数えている）
    pub uncounted_flyby_dv_m_s: f64,
}

/// マヌーバ 1 つの評価器（前の軌道と、`transfer` だけを差し替えるマヌーバ）。
/// 通過の天体・近点の高度の下限・動力の有無は書かれたマヌーバのもので、日付だけを探索が決める。
pub(super) struct Evaluator {
    from: Orbit,
    maneuver: ManeuverInput,
    flybys: Vec<FlybyInput>,
}

impl Evaluator {
    pub fn new(from: Orbit, maneuver: &ManeuverInput) -> Self {
        let flybys = maneuver
            .transfer
            .as_ref()
            .map(|t| t.flybys.clone())
            .unwrap_or_default();
        Self {
            from,
            maneuver: maneuver.clone(),
            flybys,
        }
    }

    /// 探索する通過の数（書かれたマヌーバの通過の数。直接の遷移は 0）。
    pub fn flyby_count(&self) -> usize {
        self.flybys.len()
    }

    /// 通過の天体（書かれた順）。
    pub fn flyby_bodies(&self) -> Vec<sat_astro::ephem::CentralBody> {
        self.flybys.iter().map(|f| f.body).collect()
    }

    /// 動力なしと書かれた通過があるか。
    pub fn has_unpowered_flyby(&self) -> bool {
        self.flybys.iter().any(|f| !f.powered)
    }

    /// 書かれた通過の日（書かれた順）。
    pub fn written_flyby_dates(&self) -> Vec<Epoch> {
        self.flybys.iter().map(|f| f.date).collect()
    }

    /// 直接の遷移（通過なし）の、出発の日 `departure`・到着の日 `arrival` での値。[`Evaluator::at_via`] の通過ゼロの場合。
    pub fn at(&mut self, departure: Epoch, arrival: Epoch) -> Result<CellValues, AnalysisError> {
        self.at_via(departure, &[], arrival)
    }

    /// 出発の日 `departure`・通過の日 `via`（書かれた通過と同じ数・日付の順）・到着の日 `arrival` での値。
    /// 解けなければ（暦の外・面が決まらない・到着が出発より前など）エラー。通過が実現できない日付は解けるが `feasible` が偽。
    pub fn at_via(
        &mut self,
        departure: Epoch,
        via: &[Epoch],
        arrival: Epoch,
    ) -> Result<CellValues, AnalysisError> {
        // 出発の噴射を誰が行うか（`departure_by`）は、書かれたものを保つ（打ち上げ機なら捕獲だけを数える）
        let departure_by = self
            .maneuver
            .transfer
            .as_ref()
            .map(|t| t.departure_by)
            .unwrap_or_default();
        self.maneuver.transfer = Some(DatedTransferInput {
            departure,
            arrival,
            departure_by,
            // 通過は書かれたもの（天体・高度の下限・動力）のまま、日付だけを差し替える
            flybys: self
                .flybys
                .iter()
                .zip(via)
                .map(|(f, date)| FlybyInput { date: *date, ..*f })
                .collect(),
        });
        let planned = plan(self.from, &self.maneuver)?;
        let (Some(d), Ideal::Impulsive(t)) = (planned.dated.as_ref(), &planned.ideal) else {
            return Err(AnalysisError::InvalidInput(format!(
                "Maneuver {}: the launch window search needs a dated transfer between bodies",
                self.maneuver.id
            )));
        };
        // `sequence::apply` と同じ順・同じ足し方（0 から噴射の順に）
        let delta_v_m_s = t.burns.iter().fold(0.0, |sum, b| sum + b.delta_v_m_s);
        Ok(CellValues {
            time_of_flight_s: d.transfer_time_s,
            c3_m2_s2: d.c3_m2_s2,
            v_inf_departure_m_s: d.v_inf_departure_m_s.norm(),
            v_inf_arrival_m_s: d.v_inf_arrival_m_s.norm(),
            declination_rad: d.departure_declination_rad,
            delta_v_m_s,
            ephemeris: d.ephemeris,
            flyby_dates: via.to_vec(),
            feasible: d.flybys.iter().all(|f| f.feasible),
            guide_m_s: guide(delta_v_m_s, d, &self.flybys),
            objective_m_s: delta_v_m_s + uncounted_flyby_dv(d, &self.flybys),
            not_ballistic: d
                .flybys
                .iter()
                .zip(&self.flybys)
                .any(|(f, w)| !w.powered && f.feasible && f.powered_dv_m_s > BALLISTIC_DV_M_S),
            uncounted_flyby_dv_m_s: uncounted_flyby_dv(d, &self.flybys),
        })
    }
}

/// 動力なしと書かれた通過の、数えていない近点の噴射の ΔV の合計（`FlybyPass::powered_dv_m_s`。`swing_by_unpowered` が見る量と同じ）。
fn uncounted_flyby_dv(d: &sat_astro::transfer::DatedTransfer, written: &[FlybyInput]) -> f64 {
    d.flybys
        .iter()
        .zip(written)
        .filter(|(_, w)| !w.powered)
        .map(|(f, _)| f.powered_dv_m_s)
        .sum()
}

/// 探索の目安: 選ぶ基準（ΔV + 動力なしの通過の数えていない近点の噴射）+ 動力なしの通過の足りない偏向の分（動力ありの通過は、その分が噴射に入っていて ΔV に数えている）。
fn guide(delta_v_m_s: f64, d: &sat_astro::transfer::DatedTransfer, written: &[FlybyInput]) -> f64 {
    let unpowered_shortfall: f64 = d
        .flybys
        .iter()
        .zip(written)
        .filter(|(_, w)| !w.powered)
        .map(|(f, _)| f.shortfall_dv_m_s)
        .sum();
    delta_v_m_s + uncounted_flyby_dv(d, written) + unpowered_shortfall
}
