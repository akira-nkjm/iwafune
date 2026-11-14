//! マヌーバごとの ΔV・噴射時間・推薬・前後の軌道（`propulsion.maneuvers`）。
//!
//! 公称の値（ΔV のマージンも残る推薬も見込まない）。初期質量は乾燥質量 + 使う推薬。マージン込みの推薬は
//! `propulsion.propellant`、区分ごとの合計とマージンは `propulsion.dv_budget`。

use super::input::{DvCategory, PropulsionInput, TransferMethod};
pub use super::plan::ManeuverNote;
use super::plan::Orbit;
use super::sequence::{Leg, Options, Sequence, run};
use crate::spec::{Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, Verdict};
use sat_astro::ephem::CentralBody;
use sat_units::typed::{Angle, Length, Mass, Power, Qty, Time, Velocity};
use serde::{Deserialize, Serialize};

/// マヌーバの前後の軌道（高度と傾斜角）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ManeuverOrbit {
    pub perigee_altitude: Qty<Length>,
    pub apogee_altitude: Qty<Length>,
    pub inclination: Qty<Angle>,
}

impl ManeuverOrbit {
    pub(super) fn of(central: CentralBody, o: &Orbit) -> Self {
        let h = |r: f64| Qty::from_raw(r - central.radius_m());
        Self {
            perigee_altitude: h(o.apsides.periapsis_m),
            apogee_altitude: h(o.apsides.apoapsis_m),
            inclination: Qty::from_raw(o.inclination_rad),
        }
    }
}

/// 噴射 1 回。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ManeuverBurn {
    /// 噴射する位置の高度
    pub altitude: Qty<Length>,
    /// ΔV（有限燃焼なら損失込み）
    pub delta_v: Qty<Velocity>,
    /// 噴射時間（スラスタが無ければ `None`）
    pub burn_time: Option<Qty<Time>>,
}

/// マヌーバ 1 つの結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ManeuverResult {
    pub id: String,
    pub category: DvCategory,
    pub method: TransferMethod,
    pub thruster: Option<String>,
    pub before: ManeuverOrbit,
    pub after: ManeuverOrbit,
    /// ΔV（有限燃焼なら損失込み）
    pub delta_v: Qty<Velocity>,
    /// 損失を除いた ΔV（インパルス・低推力では `delta_v` と同じ）
    pub ideal_delta_v: Qty<Velocity>,
    pub burns: Vec<ManeuverBurn>,
    /// 噴射時間の合計（スラスタが無ければ `None`）
    pub burn_time: Option<Qty<Time>>,
    /// 遷移にかかる時間（インパルスは噴射の間の慣性飛行、低推力は噴射時間。低推力でスラスタが無ければ `None`）
    pub transfer_time: Option<Qty<Time>>,
    /// 推薬（スラスタが無ければ `None`）
    pub propellant: Option<Qty<Mass>>,
    pub mass_before: Qty<Mass>,
    pub mass_after: Qty<Mass>,
    /// 噴射中の消費電力（スラスタの値）
    pub firing_power: Option<Qty<Power>>,
    /// 書かれているが粗い段階では効かないので無視した制約のキー
    pub ignored_constraints: Vec<String>,
    pub notes: Vec<ManeuverNote>,
    pub checks: Vec<Check>,
}

/// `propulsion.maneuvers` の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ManeuversOutput {
    pub maneuvers: Vec<ManeuverResult>,
    /// ΔV の合計（公称）
    pub total_delta_v: Qty<Velocity>,
    /// 推薬の合計（スラスタのあるマヌーバの公称）
    pub total_propellant: Qty<Mass>,
    pub dry_mass: Qty<Mass>,
    /// 最初のマヌーバの前の質量（乾燥質量 + 公称の推薬）
    pub initial_mass: Qty<Mass>,
    /// 最初のマヌーバの前と、最後のマヌーバのあとの軌道
    pub initial_orbit: ManeuverOrbit,
    pub final_orbit: ManeuverOrbit,
}

/// 区間 1 つ → 結果（ΔV のマージンを含まない公称の値）。
pub(super) fn result(central: CentralBody, leg: &Leg<'_>) -> ManeuverResult {
    let m = leg.maneuver;
    let checks = if leg.notes.contains(&ManeuverNote::FiniteBurnTooLong) {
        vec![Check::new(
            "finite_burn_arc",
            "有限燃焼の近似",
            Verdict::Warning,
            format!(
                "{} の噴射が軌道の半周を超えるので、損失を見込めません（method を low_thrust にしてください）",
                m.id
            ),
        )]
    } else {
        Vec::new()
    };
    ManeuverResult {
        id: m.id.clone(),
        category: m.category,
        method: m.method,
        thruster: m.thruster.clone(),
        before: ManeuverOrbit::of(central, &leg.planned.from),
        after: ManeuverOrbit::of(central, &leg.planned.to),
        delta_v: Qty::from_raw(leg.delta_v_m_s),
        ideal_delta_v: Qty::from_raw(leg.ideal_delta_v_m_s),
        burns: leg
            .burns
            .iter()
            .map(|b| ManeuverBurn {
                altitude: Qty::from_raw(b.radius_m - central.radius_m()),
                delta_v: Qty::from_raw(b.delta_v_m_s),
                burn_time: b.burn_time_s.map(Qty::from_raw),
            })
            .collect(),
        burn_time: leg.burn_time_s.map(Qty::from_raw),
        transfer_time: leg.transfer_time_s.map(Qty::from_raw),
        propellant: leg.propellant_kg.map(Qty::from_raw),
        mass_before: Qty::from_raw(leg.mass_before_kg),
        mass_after: Qty::from_raw(leg.mass_after_kg),
        firing_power: leg.thruster.and_then(|t| t.firing_power),
        ignored_constraints: m.ignored_constraints.clone(),
        notes: leg.notes.clone(),
        checks,
    }
}

/// 公称の列（ΔV のマージン・残る推薬なし）。
pub(super) fn nominal(input: &PropulsionInput) -> Result<Sequence<'_>, AnalysisError> {
    run(input, Options::NOMINAL)
}

/// マヌーバごとの ΔV・噴射時間・推薬・前後の軌道。
pub struct Maneuvers;

impl Analysis for Maneuvers {
    const ID: &'static str = "propulsion.maneuvers";
    const TITLE: &'static str = "マヌーバ";
    const TITLE_EN: &'static str = "Maneuvers";
    const AREA: AnalysisArea = AnalysisArea::System(System::Propulsion);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "マヌーバを順に方式（インパルス・有限燃焼・低推力）で計算し、ΔV・噴射時間・推薬・前後の軌道を出す";
    const DESCRIPTION_EN: &'static str = "Computes each maneuver in order by its method (impulsive, finite burn, low thrust): delta-V, burn time, propellant and the orbits before and after";

    type Input = PropulsionInput;
    type Output = ManeuversOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let central = input.initial_orbit.central;
        let seq = nominal(input)?;
        let maneuvers: Vec<ManeuverResult> = seq.legs.iter().map(|l| result(central, l)).collect();
        Ok(ManeuversOutput {
            total_delta_v: Qty::from_raw(seq.legs.iter().map(|l| l.delta_v_m_s).sum()),
            total_propellant: Qty::from_raw(seq.used_propellant_kg()),
            dry_mass: input.dry_mass,
            initial_mass: Qty::from_raw(seq.initial_mass_kg),
            initial_orbit: ManeuverOrbit::of(central, &Orbit::of(&input.initial_orbit)?),
            final_orbit: ManeuverOrbit::of(central, &seq.final_orbit),
            maneuvers,
        })
    }
}
