//! マヌーバごとの前後の軌道と、方式ごとの理想の ΔV（質量によらない部分）。遷移の式は `sat_astro::transfer`。
//!
//! - 軌道は近地点・遠地点の半径と傾斜角で持つ（近点線は揃っているとみなす。RAAN の変化は扱わない）。
//! - 目標の軌道は、書いた要素だけを前の軌道から変える（`altitude` は近地点と遠地点の両方）。
//! - インパルス・有限燃焼: 2 回噴射の遷移（中間の遠地点を書けばバイエリプティック）。平面変更は遠い方の噴射に合わせる。
//! - 低推力: Edelbaum の式（円軌道どうし。円でなければ長半径の円で近似したと注記する）。

use super::input::{ManeuverInput, OrbitTarget, TransferMethod};
use crate::spec::AnalysisError;
use sat_astro::ephem::CentralBody;
use sat_astro::orbit::OrbitSpec;
use sat_astro::transfer::{Apsides, ImpulsiveTransfer, apse_transfer, bi_elliptic, edelbaum};
use serde::{Deserialize, Serialize};

/// 離心率がこれを超える軌道は、低推力の円軌道の近似から外れるとみなす。
const CIRCULAR_ECCENTRICITY: f64 = 1e-3;

/// マヌーバの計算の注記。**文は持たない**（service の語彙が引く）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ManeuverNote {
    /// 中間の遠地点を通るバイエリプティック遷移で計算した
    BiElliptic,
    /// 低推力では中間の遠地点を使わない
    ViaAltitudeIgnored,
    /// 低推力の Edelbaum の式は円軌道どうしの式なので、長半径の円軌道で近似した
    LowThrustCircularApproximation,
    /// スラスタが無いので ΔV だけを求め、推薬・噴射時間は求めない
    NoThruster,
    /// 有限燃焼の噴射が軌道の半周を超えるので、損失を見込まずインパルスの ΔV を使った
    FiniteBurnTooLong,
}

/// 軌道（近地点・遠地点の半径と傾斜角）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Orbit {
    pub apsides: Apsides,
    pub inclination_rad: f64,
}

impl Orbit {
    /// 軌道の仕様の元期の要素から（`a(1 ∓ e)`）。
    pub fn of(spec: &OrbitSpec) -> Result<Self, AnalysisError> {
        let el = spec.kepler_elements()?;
        Ok(Self {
            apsides: Apsides::new(el.a_m * (1.0 - el.e), el.a_m * (1.0 + el.e)),
            inclination_rad: el.i_rad,
        })
    }

    fn eccentricity(&self) -> f64 {
        let (p, a) = (self.apsides.periapsis_m, self.apsides.apoapsis_m);
        (a - p) / (a + p)
    }

    /// 目標の要素を当てた軌道。
    fn with_target(self, central: CentralBody, t: &OrbitTarget) -> Self {
        let r = |h: sat_units::typed::Qty<sat_units::typed::Length>| central.radius_m() + h.value();
        let (mut rp, mut ra) = (self.apsides.periapsis_m, self.apsides.apoapsis_m);
        if let Some(h) = t.altitude {
            (rp, ra) = (r(h), r(h));
        }
        if let Some(h) = t.perigee_altitude {
            rp = r(h);
        }
        if let Some(h) = t.apogee_altitude {
            ra = r(h);
        }
        Self {
            apsides: Apsides::new(rp, ra),
            inclination_rad: t.inclination.map_or(self.inclination_rad, |i| i.value()),
        }
    }
}

/// 理想の ΔV（損失・質量によらない部分）。
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Ideal {
    /// インパルスの噴射の列（有限燃焼はこれに損失を足す）
    Impulsive(ImpulsiveTransfer),
    /// 低推力の連続噴射の ΔV [m/s]
    LowThrust(f64),
}

/// 1 つのマヌーバの前後の軌道と理想の ΔV。
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Planned {
    pub from: Orbit,
    pub to: Orbit,
    pub ideal: Ideal,
    pub notes: Vec<ManeuverNote>,
}

/// 軌道 `from` からマヌーバ `m` を行う計画。
pub(super) fn plan(
    central: CentralBody,
    from: Orbit,
    m: &ManeuverInput,
) -> Result<Planned, AnalysisError> {
    let to = from.with_target(central, &m.target);
    let angle = (to.inclination_rad - from.inclination_rad).abs();
    let mu = central.gm();
    let mut notes = Vec::new();
    let ideal = match m.method {
        TransferMethod::Impulsive | TransferMethod::FiniteBurn => match m.via_altitude {
            Some(h) => {
                notes.push(ManeuverNote::BiElliptic);
                let via = central.radius_m() + h.value();
                Ideal::Impulsive(bi_elliptic(mu, from.apsides, to.apsides, via, angle)?)
            }
            None => Ideal::Impulsive(apse_transfer(mu, from.apsides, to.apsides, angle)?),
        },
        TransferMethod::LowThrust => {
            if m.via_altitude.is_some() {
                notes.push(ManeuverNote::ViaAltitudeIgnored);
            }
            if from.eccentricity() > CIRCULAR_ECCENTRICITY
                || to.eccentricity() > CIRCULAR_ECCENTRICITY
            {
                notes.push(ManeuverNote::LowThrustCircularApproximation);
            }
            Ideal::LowThrust(edelbaum(
                mu,
                from.apsides.semi_major_axis_m(),
                to.apsides.semi_major_axis_m(),
                angle,
            )?)
        }
    };
    Ok(Planned {
        from,
        to,
        ideal,
        notes,
    })
}
